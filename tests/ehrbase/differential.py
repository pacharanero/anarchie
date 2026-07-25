#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Load one synthetic record into EHRbase and anarchie and compare AQL results."""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import subprocess
import tempfile
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

EHR_ID = "11111111-1111-4111-8111-111111111111"
TEMPLATE_ID = "patient_blood_pressure.v1"
OBSERVATION_V1 = "openEHR-EHR-OBSERVATION.blood_pressure.v1"
OBSERVATION_V2 = "openEHR-EHR-OBSERVATION.blood_pressure.v2"


def main() -> int:
    args = parse_args()
    root = Path(args.root).resolve()

    with tempfile.TemporaryDirectory(prefix="anarchie-ehrbase-") as temporary:
        work = Path(temporary)
        opt = fetch(args.opt_url, args.opt_sha256)
        composition = prepare_composition(root / "tests/fixtures/blood-pressure-composition.json")
        composition_path = work / "composition.json"
        composition_path.write_text(json.dumps(composition, indent=2) + "\n", encoding="utf-8")

        load_ehrbase(args, opt, composition)
        deployment, anarchie_ehr_id = load_anarchie(args, work, composition_path)
        queries = json.loads((root / "tests/ehrbase/queries.json").read_text(encoding="utf-8"))

        failures = []
        for case in queries:
            ehrbase = normalize(query_ehrbase(args, case["aql"]))
            anarchie = normalize(query_anarchie(args, deployment, case["aql"]))
            if ehrbase == anarchie:
                print(f'{case["id"]}: ok')
            else:
                failures.append(case["id"])
                print(f'{case["id"]}: mismatch')
                print("  EHRbase: " + json.dumps(ehrbase, sort_keys=True))
                print("  anarchie: " + json.dumps(anarchie, sort_keys=True))

        if failures:
            print("Differential AQL failures: " + ", ".join(failures))
            return 1

        print(f"Loaded synthetic EHRbase EHR {EHR_ID}")
        print(f"Loaded synthetic anarchie EHR {anarchie_ehr_id}")
        print(f"All {len(queries)} differential AQL cases matched")
        return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True)
    parser.add_argument("--binary", required=True)
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--user", required=True)
    parser.add_argument("--password", required=True)
    parser.add_argument("--opt-url", required=True)
    parser.add_argument("--opt-sha256", required=True)
    return parser.parse_args()


def fetch(url: str, expected_sha256: str) -> bytes:
    with urllib.request.urlopen(url, timeout=30) as response:
        content = response.read()
    actual = hashlib.sha256(content).hexdigest()
    if actual != expected_sha256:
        raise RuntimeError(f"checksum mismatch for {url}: {actual}")
    return content


def prepare_composition(path: Path) -> dict[str, Any]:
    composition = json.loads(path.read_text(encoding="utf-8"))
    composition.pop("uid", None)
    composition["archetype_details"]["template_id"]["value"] = TEMPLATE_ID
    replace_archetype_id(composition)
    observation = composition["content"][0]
    observation["archetype_details"] = {
        "_type": "ARCHETYPED",
        "archetype_id": {"_type": "ARCHETYPE_ID", "value": OBSERVATION_V1},
        "template_id": {"_type": "TEMPLATE_ID", "value": TEMPLATE_ID},
        "rm_version": "1.1.0",
    }
    return composition


def replace_archetype_id(value: Any) -> None:
    if isinstance(value, dict):
        for key, child in value.items():
            if key == "archetype_node_id" and child == OBSERVATION_V2:
                value[key] = OBSERVATION_V1
            else:
                replace_archetype_id(child)
    elif isinstance(value, list):
        for child in value:
            replace_archetype_id(child)


def request(
    args: argparse.Namespace,
    method: str,
    path: str,
    body: bytes | None = None,
    content_type: str = "application/json",
) -> bytes:
    token = base64.b64encode(f"{args.user}:{args.password}".encode()).decode()
    accept = "application/xml" if content_type == "application/xml" else "application/json"
    headers = {
        "Authorization": f"Basic {token}",
        "Accept": accept,
        "Content-Type": content_type,
    }
    req = urllib.request.Request(
        args.base_url + "/rest/openehr/v1" + path,
        data=body,
        headers=headers,
        method=method,
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as response:
            return response.read()
    except urllib.error.HTTPError as error:
        detail = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"{method} {path} returned {error.code}: {detail}") from error


def load_ehrbase(args: argparse.Namespace, opt: bytes, composition: dict[str, Any]) -> None:
    request(
        args,
        "POST",
        "/definition/template/adl1.4",
        opt.removeprefix(b"\xef\xbb\xbf"),
        "application/xml",
    )
    request(args, "PUT", f"/ehr/{EHR_ID}", b"")
    request(
        args,
        "POST",
        f"/ehr/{EHR_ID}/composition",
        json.dumps(composition).encode("utf-8"),
    )


def run_anarchie(
    args: argparse.Namespace, cwd: Path, *arguments: str
) -> dict[str, Any]:
    completed = subprocess.run(
        [args.binary, "--format", "json", *arguments],
        cwd=cwd,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def load_anarchie(
    args: argparse.Namespace, work: Path, composition_path: Path
) -> tuple[Path, str]:
    deployment = work / "anarchie"
    run_anarchie(args, work, "init", str(deployment), "--minimal")
    ehr = run_anarchie(args, deployment, "ehr", "new")
    run_anarchie(
        args,
        deployment,
        "commit",
        ehr["ehr_id"],
        str(composition_path),
        "--no-validate",
    )
    run_anarchie(args, deployment, "index", "--rebuild")
    return deployment, ehr["ehr_id"]


def query_ehrbase(args: argparse.Namespace, aql: str) -> dict[str, Any]:
    body = request(args, "POST", "/query/aql", json.dumps({"q": aql}).encode())
    return json.loads(body)


def query_anarchie(
    args: argparse.Namespace, deployment: Path, aql: str
) -> dict[str, Any]:
    return run_anarchie(args, deployment, "aql", aql)


def normalize(result: dict[str, Any]) -> dict[str, Any]:
    columns = [column.get("name") for column in result.get("columns", [])]
    rows = result.get("rows", [])
    rows.sort(key=lambda row: json.dumps(row, sort_keys=True, separators=(",", ":")))
    return {"columns": columns, "rows": rows}


if __name__ == "__main__":
    raise SystemExit(main())

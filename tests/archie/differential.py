#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Compare Archie and anarchie validation verdicts over shared Compositions."""

from __future__ import annotations

import argparse
import copy
import json
import subprocess
import tempfile
from pathlib import Path
from typing import Any

ARCHETYPE_ID = "openEHR-EHR-COMPOSITION.anarchie_validator.v1"
TEMPLATE_ID = "anarchie_validator.v1"


def main() -> int:
    args = parse_args()
    root = Path(args.root).resolve()
    suite = root / "tests/archie"
    base = prepare_composition(root / "tests/fixtures/blood-pressure-composition.json")
    cases = json.loads((suite / "cases.json").read_text(encoding="utf-8"))
    failures: list[str] = []

    with tempfile.TemporaryDirectory(prefix="anarchie-archie-") as temporary:
        work = Path(temporary)
        deployment = work / "deployment"
        run([args.binary, "init", str(deployment), "--minimal"], cwd=root)
        run(
            [
                args.binary,
                "--format",
                "json",
                "template",
                "add",
                str(suite / "fixtures/anarchie-validator.opt.json"),
            ],
            cwd=deployment,
        )

        for case in cases:
            composition = copy.deepcopy(base)
            mutate(composition, case["mutation"])
            composition_path = work / f'{case["id"]}.json'
            composition_path.write_text(
                json.dumps(composition, indent=2) + "\n", encoding="utf-8"
            )

            archie = run_json(
                [
                    str(root / "s/archie-validate"),
                    str(suite / "fixtures/anarchie-validator.adls"),
                    str(composition_path),
                ],
                cwd=root,
            )
            anarchie = run_json(
                [
                    args.binary,
                    "--format",
                    "json",
                    "validate",
                    str(composition_path),
                    "--template",
                    TEMPLATE_ID,
                ],
                cwd=deployment,
                allow_failure=True,
            )
            expected = case["expected_valid"]
            if archie["valid"] == anarchie["valid"] == expected:
                print(f'{case["id"]}: ok')
            else:
                failures.append(case["id"])
                print(
                    f'{case["id"]}: mismatch '
                    f'(expected={expected}, archie={archie["valid"]}, '
                    f'anarchie={anarchie["valid"]})'
                )
                print("  Archie: " + json.dumps(archie.get("messages", [])))
                print("  anarchie: " + json.dumps(anarchie.get("violations", [])))

    if failures:
        print("Differential validation failures: " + ", ".join(failures))
        return 1
    print(f"All {len(cases)} differential validation cases matched")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True)
    parser.add_argument("--binary", required=True)
    return parser.parse_args()


def prepare_composition(path: Path) -> dict[str, Any]:
    composition = json.loads(path.read_text(encoding="utf-8"))
    composition["archetype_node_id"] = ARCHETYPE_ID
    composition["archetype_details"]["archetype_id"]["value"] = ARCHETYPE_ID
    composition["archetype_details"]["template_id"]["value"] = TEMPLATE_ID
    composition["content"] = []
    return composition


def mutate(composition: dict[str, Any], mutation: str) -> None:
    if mutation == "none":
        return
    if mutation == "wrong_category_code":
        composition["category"]["defining_code"]["code_string"] = "999"
    elif mutation == "empty_rm_version":
        composition["archetype_details"]["rm_version"] = ""
    elif mutation == "empty_language_code":
        composition["language"]["code_string"] = ""
    elif mutation == "empty_territory_code":
        composition["territory"]["code_string"] = ""
    elif mutation == "empty_archetype_node_id":
        composition["archetype_node_id"] = ""
    else:
        raise ValueError(f"unknown mutation: {mutation}")


def run(
    command: list[str], cwd: Path, allow_failure: bool = False
) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(command, cwd=cwd, capture_output=True, text=True)
    if completed.returncode != 0 and not allow_failure:
        raise RuntimeError(
            f'command failed ({completed.returncode}): {" ".join(command)}\n'
            f"stdout:\n{completed.stdout}\nstderr:\n{completed.stderr}"
        )
    return completed


def run_json(
    command: list[str], cwd: Path, allow_failure: bool = False
) -> dict[str, Any]:
    completed = run(command, cwd, allow_failure)
    if not completed.stdout.strip():
        raise RuntimeError(
            f'command produced no JSON: {" ".join(command)}\nstderr:\n{completed.stderr}'
        )
    return json.loads(completed.stdout)


if __name__ == "__main__":
    raise SystemExit(main())

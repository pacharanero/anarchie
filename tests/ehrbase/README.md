<!-- SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd -->
<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# Local EHRbase Oracle

This Compose setup runs a disposable, localhost-only EHRbase instance for manual interoperability work and future differential tests. It is a test-time oracle, never an anarchie runtime dependency.

Run it with:

```sh
s/ehrbase up
s/ehrbase logs
s/ehrbase test
s/ehrbase down
```

It listens on `http://127.0.0.1:8088/ehrbase` by default. Set `EHRBASE_ORACLE_PORT` before `up` to use another host port. The test-only basic-auth user is `ehrbase-oracle` with password `local-oracle-only`.

`s/ehrbase down` removes the Compose project's containers, networks, and Docker volumes. `s/ehrbase test` always starts from a clean oracle and tears it down when the test ends. Do not use this setup with real patient data or treat it as an EHRbase deployment. The images are pinned to EHRbase 2.34.0 and its matching EHRbase PostgreSQL image.

The differential test fetches EHRbase's Apache-2.0 `patient_blood_pressure.v1` test OPT from the pinned EHRbase 2.34.0 source revision into a temporary directory, verifies its SHA-256, and does not redistribute it. It derives one canonical Composition from anarchie's synthetic blood-pressure fixture: the server-assigned UID is omitted, the template and blood-pressure archetype identifiers are changed to match the OPT, and the nested OBSERVATION gains the `archetype_details` required by the RM invariant. That exact derived Composition is loaded into both systems. The three cases in `queries.json` compare normalised column names and rows for composition count, systolic projection, and systolic filtering. Response metadata and row order are intentionally excluded from comparison.

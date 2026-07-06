<!--
SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Security Policy

## Scope and status

`anarchie` is an experimental, offline-first openEHR toolset. It is **not for clinical use** and is not a deployed service, so a vulnerability here is a defect in a tool, not a patient-safety incident in a live system. It still handles clinical-shaped data, so we take reports seriously.

## Reporting a vulnerability

Please report security issues **privately**, not in a public issue or pull request:

- Preferred: open a private report via GitHub's [private vulnerability reporting](https://github.com/pacharanero/anarchie/security/advisories/new) (Security -> Report a vulnerability).
- Or email **marcus@bawmedical.co.uk** with the details and, if possible, a minimal reproduction.

Please include the affected version or commit, the impact, and reproduction steps. We aim to acknowledge reports within a few days. As a personal experimental project there is no formal SLA, but credible reports will be triaged and fixed as soon as practical, and we will credit reporters who wish to be named.

## Handling

Fixes are recorded with auditable evidence (commit, test, or dependency bump). Do not include exploit details or any real credentials in public channels; keep them in the private report until a fix is released.

<!-- SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd -->
<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# Archie validation oracle

This directory holds a test-only differential oracle for anarchie's native RM and Operational Template validator. `s/archie-conformance` runs each synthetic Composition through anarchie and Archie, then fails if either verdict differs from the expected result or from the other implementation.

The Java adapter is compiled as a Gradle composite build against Archie 3.17.0 at the exact source revision in `archie-reference.env`. `s/archie-validate` reuses a matching sibling `../archie` checkout when available; otherwise it fetches the pinned source into ignored `target/archie-oracle/` state. Archie and the JVM remain development dependencies and never enter anarchie's Rust dependency graph or shipped binary.

## Commands

- `s/archie-conformance` - build anarchie and run the complete differential corpus.
- `s/archie-validate tests/archie/fixtures/anarchie-validator.adls composition.json` - run one canonical Composition through Archie and print its JSON verdict.
- `ARCHIE_CHECKOUT=/path/to/archie s/archie-validate ...` - use an explicit checkout, provided its HEAD is the pinned revision.

## Corpus

`cases.json` defines six initial valid and invalid cases covering root Composition RM invariants and a template-bound category code. `differential.py` derives every case from the shared synthetic blood-pressure fixture in a temporary directory, so no patient data or generated third-party artefact is stored.

The Archie fixture is authored ADL2 source that the adapter flattens into an `OperationalTemplate`. Archie 3.17 supports ADL2/OPT2, not legacy ADL 1.4 OPT XML. Nested archetype constraints are intentionally deferred until the harness supplies Archie with a complete archetype repository; the current root-only template avoids pretending that unresolved child archetypes have been tested.

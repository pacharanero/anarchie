<!-- SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd -->
<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# AQL Conformance Corpus

This corpus records syntax behaviour at the boundary between the official openEHR AQL grammar and anarchie's hand-written Rust parser. It is test data owned by anarchie; it does not redistribute the openEHR grammar.

`cases.tsv` is the case manifest. Its columns are `id`, `fixture`, `reference`, `anarchie`, and `feature`.

- `reference` is the expected result from the official grammar.
- `anarchie` is the expected result from `anarchie::query::parse`.
- A `reference = accept`, `anarchie = reject` row is a deliberate, documented syntax gap. It is not a test failure until anarchie claims that feature.

`cargo test --test aql-conformance` verifies anarchie's expected results without Java or network access. `s/aql-conformance` fetches the grammar at the pinned revision in `aql-reference.env`, verifies its checksum, generates an official Java ANTLR parser in a temporary directory, and verifies the `reference` column. It leaves no grammar or generated parser in the checkout.

## Updating the reference

The grammar source is [openEHR/specifications-QUERY](https://github.com/openEHR/specifications-QUERY/tree/master/docs/AQL/grammar). When upstream changes:

1. Review the upstream grammar diff and licensing terms.
2. Update `AQL_GRAMMAR_REVISION` and the two grammar SHA-256 values in `aql-reference.env`.
3. Run `s/aql-conformance` and update the manifest deliberately for any intended grammar-result changes.
4. Add fixtures for newly relevant productions and record unsupported features as explicit gaps.

The Java runner is kept deliberately standalone so it can be donated upstream or reused by another implementation without depending on anarchie's production code.

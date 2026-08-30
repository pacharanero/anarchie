<!--
SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Rust SDK

## Decision

`anarchie` is both a local-first openEHR CDR and the first consumer of a reusable Rust openEHR SDK. The SDK will be published for reuse when it has an external consumer. GitEHR's openEHR compatibility is the intended first such consumer.

This is a boundary decision, not an immediate repository split. Keeping one crate is the simplest working arrangement while the CDR is the only consumer. New code below the CDR boundary must nevertheless be written so that extracting it does not require a redesign.

## SDK contract

The SDK owns pure openEHR operations over caller-provided data:

- Reference Model Rust types and canonical JSON parsing and serialisation.
- Archetype Object Model constraint types.
- Operational Template representation and source-format lowering into that representation.
- RM and template validation, returning structured, addressable violations.
- Deterministic conversions that require no deployment, filesystem, database, network, clock, or process-global state.

The SDK does not own:

- deployment layout, filesystem or git transactions, contribution history, or access control;
- SQLite/DuckDB indexes, AQL persistence, REST, MCP, CLI, or a server runtime;
- archetype authoring, ADL specialisation, template flattening, or terminology content.

The CDR layers remain adapters over this kernel. They provide persistence, versioning, query acceleration, and service delivery, but must not become prerequisites for an application that only needs openEHR representation, canonical serialisation, template validation, or conversion.

## Intended layers

| Layer | Current implementation | Extraction target | Dependency rule |
|---|---|---|---|
| RM | `rm` | `anarchie-rm` | `serde`, `serde_json`, and error support only |
| Constraints | `aom`, `opt` | `anarchie-aom` and template-format adapters | Pure parsing and lowering; no host I/O |
| Validation | `validate` | `anarchie-validate` | Takes RM values and template constraints; returns SDK-owned reports |
| CDR adapters | `store`, `query`, `serve`, `cli` | Remain `anarchie` product layers | May depend on the SDK, never the reverse |

The eventual crate names and exact grouping remain implementation decisions. The essential rule is dependency direction: SDK crates are leaves, and every host depends on them.

## GitEHR compatibility

GitEHR must consume the SDK directly for its openEHR compatibility layer rather than invoking the `anarchie` binary or importing its store. Shared behaviour includes RM representations, canonical JSON, template validation, and the declared conformance profiles. GitEHR's journal/state model and `anarchie`'s CDR filesystem/git model remain separate host concerns.

The first integration must add shared positive and negative conformance cases. It must prove that the same canonical Composition parses, serialises byte-stably where the profile guarantees it, and receives equivalent validation results in both products.

## Extraction and publication trigger

Extraction begins when GitEHR has a concrete integration needing the SDK. Before then, one crate avoids unnecessary release and dependency management. The migration sequence is:

1. Move the kernel behind a deliberately documented, semver-versioned API while keeping the CDR as its consumer.
2. Add independent tests and named conformance profiles covering that API.
3. Create a leaf crate with no host dependencies and retain history with `git subtree split` when it moves to its own repository.
4. Let GitEHR consume it by path dependency during co-development, then by revision-pinned git dependency once separated, and finally from crates.io after a release pipeline exists.

Publishing a crate does not imply a permissive licence. SDK code remains AGPL-3.0-or-later under the current project policy. A different licence or a dual-licensing model would require a separate, explicit decision.

## Readiness criteria

The SDK is ready for its first published release when:

- its public API is intentionally documented and semver-governed;
- its crates are leaves with no CDR, filesystem, git, database, HTTP/MCP, CLI, runtime, clock, or global-state dependency;
- supported RM, template, serialisation, and validation subsets map to named conformance profiles;
- every public operation has focused positive and negative tests plus shared GitEHR compatibility cases where applicable;
- the CDR and GitEHR both use the same published API rather than duplicate model or validation logic;
- the licence and dependency metadata are suitable for publication.

Until those criteria are met, `anarchie` must describe the exposed Rust modules as an internal, reusable kernel rather than a complete or stable openEHR SDK.

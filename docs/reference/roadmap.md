# Roadmap

`anarchie` is primarily a learning and experimentation project, so the roadmap
optimises for *learning something at each step* and for *always having a working
artefact*, rather than racing to feature-completeness. Each phase produces
something runnable and inspectable.

The authoritative, checkbox-level roadmap lives in
[specs/roadmap.md](https://github.com/pacharanero/anarchie/blob/main/specs/roadmap.md).
This page is a reader-friendly summary of where things stand.

## :material-check-circle:{ .mdx-pulse } Shipped

### Phase 1 - The Reference Model in Rust

Native Rust types for the core Reference Model, with byte-stable canonical JSON
(de)serialisation proven idempotent and diff-friendly.

- [anarchie info](../commands/info.md) - inspect any Composition file.
- [anarchie canonicalise](../commands/canonicalise.md) - re-emit canonical JSON.

### Phase 2 - The file store and git

A durable, versioned, inspectable store. One git repository per EHR, the
working-tree-holds-head convention, and the `CONTRIBUTION`-as-commit mapping.

- [anarchie init](../commands/init.md) - scaffold a deployment.
- [anarchie ehr](../commands/ehr.md) - create and list EHRs.
- [anarchie commit](../commands/commit.md) - store a Composition as a contribution.
- [anarchie cat](../commands/cat.md) - read the head or a historical version.
- [anarchie log](../commands/log.md) - version history.
- [anarchie diff](../commands/diff.md) - diff two versions.

## :material-hammer-wrench: Designed and planned

| Phase | Theme               | Highlights                                                        |
| ----- | ------------------- | ----------------------------------------------------------------- |
| 3     | Validation          | `anarchie-aom`, `anarchie-opt`, `anarchie template add`, validation wired into commit, cross-checked against Archie as a test-time oracle. |
| 3.5   | Starter templates   | `anarchie init` yields a CDR that can store real clinical data immediately. |
| 4     | Query (AQL)         | SQLite path index, an AQL parser, and AQL → SQL translation; a DuckDB/Parquet analytics path. |
| 5     | Services            | `anarchie serve` (openEHR REST API) and `anarchie mcp` (stdio MCP server for LLM agents). |
| 6     | Integration         | `sct` terminology binding, `gitehr` convergence, archetype packs, FHIR projection. |

No phase depends on a later phase to be useful. Each is intended to teach
something - the biggest open question being whether a pure-Rust validator can
agree with Archie on the conformance corpus (Phase 3).

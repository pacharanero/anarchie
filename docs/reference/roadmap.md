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

### Phase 3 - Validation

Reject invalid data at the door. Reference Model invariants are checked against
the typed RM tree; archetype constraints are checked against the canonical JSON
guided by an Archetype Object Model tree. Validation runs on every commit, so
non-conformant data never reaches the git history.

- [anarchie validate](../commands/validate.md) - check a Composition against the RM and an optional template.
- [anarchie template](../commands/template.md) - register Operational Templates as the schema.
- See [Validation and Templates](../walkthrough/validation-and-templates.md) for the walkthrough.

### Phase 3.5 - Starter templates

`anarchie init` seeds a deployment with a curated set of bundled starter
Operational Templates, so a fresh CDR can store real clinical data immediately.
Pass `--minimal` for an empty CDR.

### Phase 4 - Query (AQL)

A query engine over a path-extraction index - the read model (CQRS),
rebuildable from the canonical files and never authoritative.

- `anarchie index` - build or refresh the derived query index.
- `anarchie aql` - run an ad-hoc AQL query against the index.
- `anarchie query` - register, list, and run stored (named) AQL queries.

### Phase 5 - Services

The store, exposed over the wire.

- `anarchie serve` - the openEHR REST API over HTTP (binds to localhost).
- `anarchie mcp` - a stdio MCP server exposing the store to LLM agents.

### Phase 6 - Archetype packs

Sets of Operational Templates, installable as a unit.

- `anarchie pack add` - install a bundled pack by name or from a local directory.
- `anarchie pack list` - list the bundled packs available to install.

## :material-hammer-wrench: Designed and planned

| Theme                  | Highlights                                                        |
| ---------------------- | ---------------------------------------------------------------- |
| Analytics              | A DuckDB/Parquet path alongside the AQL engine for column-oriented analytics. |
| Template ingest        | Ingesting `.opt` XML exported from Archetype Designer / the ADL Workbench. |
| Terminology            | `sct` SNOMED CT terminology binding. |
| Convergence            | `gitehr` convergence. |
| Projection             | FHIR / IPS projection of stored data. |
| Distribution           | `cargo install` and a `curl \| sh` one-liner work today (see [Install](../install.md)); prebuilt binaries (cargo-dist), Homebrew, Windows installers, and `.deb` / `.rpm` are planned. |
| Interfaces             | A TUI / GUI over the store. |

No phase depends on a later phase to be useful. Each is intended to teach
something - the next big open question being how much of AQL a DuckDB-over-JSON
approach can handle before a bespoke engine is unavoidable. The Archie
conformance-corpus cross-check from Phase 3 remains a deferred, test-time-only
exercise.

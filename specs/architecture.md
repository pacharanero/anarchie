# anarchie — Architecture Overview

## Overview

`anarchie` is a local-first, file-backed openEHR Clinical Data Repository. It follows the same philosophy as [`sct`](https://github.com/pacharanero/sct): **data over services**. Where a conventional CDR (EHRbase, Better, Cabolabs) puts a server and a relational database between the application and the disk, `anarchie` stores clinical data as canonical openEHR documents directly on the filesystem and treats every other capability as a derived, regenerable view over those documents.

The design separates a small number of concerns strictly:

1. A **write path** that validates an incoming Composition and persists it as an immutable, versioned file.
2. A **canonical store** - the flat-file directory tree - which is the single source of truth.
3. A set of **derived consumer layers** (query index, REST API, MCP server) that never own data and can be rebuilt at any time.

---

## Design principles

These are inherited from `sct` and adapted for a read-write clinical store.

- **Offline-first** - no network dependency to read, write, or query.
- **Files are the database** - the canonical store is a directory of human-readable JSON, not an opaque binary blob. Greppable with `ripgrep`, inspectable with `jq`, versionable with `git`.
- **Immutable, append-only data** - mirroring openEHR's own versioning model. A version, once written, is never mutated. Corrections are new versions.
- **Derived layers are disposable** - delete the SQLite index and rebuild it from the files; patient data is never at risk because it does not live there.
- **Standard tooling** - a Composition is queryable with `jq` and `duckdb`; history is queryable with `git log`; no custom binary required for basic inspection.
- **Conformance over invention** - the on-disk objects are openEHR canonical JSON, and the API is the openEHR REST API. `anarchie` is an implementation of existing specifications, not a new data model.
- **Composable subcommands** - small, single-purpose commands connectable over Unix pipes, with machine-friendly stdout and human chatter on stderr.
- **LLM-native** - the document store and MCP layer are designed for direct agent consumption.

---

## The onion model

```
┌──────────────────────────────────────────────────────────┐
│   REST API server  +  MCP server                         │  ← Layer 4: services / AI tool use
├──────────────────────────────────────────────────────────┤
│   AQL query engine  (SQLite path index  /  DuckDB)       │  ← Layer 3: structured query
├──────────────────────────────────────────────────────────┤
│   Derived indexes  (path table, EHR manifest, term map)  │  ← Layer 2: regenerable acceleration
├──────────────────────────────────────────────────────────┤
│   Canonical Composition JSON on disk  (the EHR tree)     │  ← Layer 1: the source of truth
├──────────────────────────────────────────────────────────┤
│   Operational Templates (OPTs) + Reference Model         │  ← Layer 0: schema / substrate
└──────────────────────────────────────────────────────────┘
```

Each layer consumes the layer below it. **Layer 1 is the stable interface.** Everything above Layer 1 is rebuildable from Layer 1 plus Layer 0. Nothing above Layer 1 is authoritative.

Contrast with `sct`, whose centre (Layer 1) is a single immutable NDJSON file because terminology is read-only reference data. `anarchie`'s Layer 1 is a *growing tree of immutable files*, because a CDR accumulates new versions over time. The immutability is per-file, not per-store.

---

## Mapping openEHR concepts onto the filesystem

The key openEHR storage abstractions and their file-system counterparts:

| openEHR concept | What it is | anarchie representation |
|---|---|---|
| `EHR` | The per-patient container | A directory, named by `ehr_id` (UUID) |
| `EHR_STATUS` | Mutable status/subject of an EHR | A versioned object inside the EHR directory |
| `VERSIONED_COMPOSITION` | The version history of one clinical document | A directory holding one file per version |
| `ORIGINAL_VERSION<COMPOSITION>` | A single immutable version | One canonical JSON file |
| `CONTRIBUTION` | An audited set of versions committed together | One commit (git) + one contribution manifest file |
| `AUDIT_DETAILS` | Who/when/why of a change | The git author/timestamp/message, mirrored into the manifest |
| `FOLDER` / directory | Logical organisation within an EHR | A folder structure file (or real subdirectories) |
| Operational Template | The schema a Composition conforms to | A registered OPT under a templates directory |

The crucial alignment: **openEHR's `CONTRIBUTION` is a git commit.** Both are an audited, atomic set of changes to versioned objects, attributed to a committer at a point in time. **Decision: git is the versioning mechanism**, not an optional mirror - the commit graph *is* the contribution history. This is explored in [versioning-and-git.md](versioning-and-git.md).

---

## The write path

Committing a Composition is the one genuinely transactional operation. The sequence:

1. **Receive** a Composition (canonical JSON) plus the target `ehr_id` and template id.
2. **Resolve** the Operational Template referenced by the Composition.
3. **Validate** against the OPT and the Reference Model, using a **Rust-native validator** (no JVM, no external runtime - see [validation.md](validation.md)).
4. **Assign** a `version_uid` (`object_id::system_id::version_tree_id`), incrementing the version tree against the existing versioned object.
5. **Write** the new version file atomically (write-to-temp, fsync, rename).
6. **Record** the contribution manifest and commit (optionally a git commit) covering exactly the versions written.
7. **Invalidate / update** the derived index for the affected paths.

Optimistic concurrency uses the precondition that the latest known version matches (the openEHR `If-Match` precondition on the REST API), enforced at step 4 by checking the current head version of the versioned object before writing.

---

## The read / query path

Two distinct read modes:

- **Direct retrieval** by id (`get composition`, `get EHR`, `get version at time`) is a filesystem lookup. No index needed; this is `cat` of a known path. Fast and always correct.
- **AQL queries** (population queries across many EHRs) need acceleration. Scanning every JSON file per query is acceptable for tiny stores and unacceptable beyond that. Layer 2/3 builds a path-extraction index so AQL can be translated to SQL. See [query-engine.md](query-engine.md).

The index is **derived and rebuildable**. A query against a stale index is a correctness bug, so the write path must keep the index in step (or mark it dirty and rebuild lazily). The fallback - rebuild the whole index from the file tree - is always available and is the integrity backstop.

---

## What is deliberately out of scope (initially)

- **Multi-node clustering / replication** - single-node, single-filesystem to start. Git remotes are the crude replication story.
- **Full ADL 2 / ADL 1.4 authoring** - `anarchie` consumes Operational Templates; it does not author archetypes. That is the Archetype Designer's job.
- **Demographics / EHR_STATUS party resolution against an external PDS** - subject references are stored opaquely.
- **Terminology validation** - delegated. Notably this is exactly where `sct` slots in: an `anarchie` validator could call `sct` to validate SNOMED/terminology bindings. The two tools compose.

---

## Relationship to gitehr

[`gitehr`](https://gitehr.org/) (in `~/code/gitehr`) is a sibling project: a git-based, decentralised, multi-contributor patient record built around an immutable **journal** of clinical entries plus a mutable **state** directory. The two projects share a thesis - *a patient record is a git repository* - but approach it from opposite ends:

| | gitehr | anarchie |
|---|---|---|
| Primary model | Its own journal/state structure | openEHR Reference Model (RM) |
| Unit of content | Journal entry + Documents | `COMPOSITION` versions |
| Query | Chronological / file-based | AQL |
| Conformance target | gitehr's own format | openEHR REST API + RM |
| Audit | Git commit chain (hash-linked journal) | Git commit = openEHR `CONTRIBUTION` |

The collision point - and the interesting future question - is whether a gitehr patient record and an anarchie EHR could become **the same repository viewed through two lenses**: gitehr's journal as the human-facing narrative, anarchie's Compositions as the structured-data layer, both committed to one git history. That is explicitly out of scope for now but motivates keeping the on-disk layout and git conventions compatible where cheap to do so.

---

## Relationship to existing CDRs

`anarchie` is not trying to beat EHRbase on throughput. It targets a different niche, the way `sct` targets a different niche from Snowstorm:

- **Research and teaching** - a CDR you can inspect, diff, and reason about without a Postgres instance.
- **Local development** - spin up a conformant-ish CDR from a binary, no Docker, no database.
- **Small deployments** - clinics or registries with modest data volumes where operational simplicity beats horizontal scale.
- **AI/agent workflows** - an MCP-native CDR whose data is plain files an agent can read directly.
- **Archival and portability** - a patient's entire record as a tarball of readable JSON, under version control.

---

## Decisions taken

1. **Validation engine** - ✅ **Rust-native.** RM + OPT validation is reimplemented in Rust. This keeps the single-binary, no-runtime promise even though it is the largest single piece of work. See [validation.md](validation.md).
2. **Versioning mechanism** - ✅ **Git is intrinsic.** The git commit graph is the openEHR contribution/version history, not an optional export. See [versioning-and-git.md](versioning-and-git.md).

## Open questions still on the table

1. **AQL coverage**: which subset of AQL is the MVP, and how much can DuckDB's JSON support do for free before a bespoke engine is needed? See [query-engine.md](query-engine.md).
2. **Canonical JSON fidelity**: can we guarantee byte-stable canonical serialisation so that files are reliably diffable and hashable? This is a hard dependency of the whole approach.
3. **Index freshness**: synchronous index update on commit, or dirty-mark-and-lazy-rebuild?
4. **gitehr convergence**: how much on-disk compatibility with gitehr is worth preserving now versus deferring?

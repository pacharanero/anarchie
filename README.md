# anarchie

> *an-archie* — an **archie** without a server. Anarchic, file-first openEHR persistence.

A local-first openEHR Clinical Data Repository (CDR) that uses **flat files as its primary persistence layer** instead of a database server. Inspired by [`sct`](https://github.com/pacharanero/sct), which replaced a SNOMED CT terminology *server* with a single greppable artefact and a handful of derived views.

The wager is simple: **openEHR data is already document-oriented.** A Composition is a self-contained, versioned clinical document. So the most natural way to store it is... as a document on disk. One immutable JSON file per version. The EHR is a directory. The audit trail is the filesystem (and, optionally, git).

This is a **specification and design exploration**, not yet an implementation. The goal is to decide whether the idea is sound before writing a line of Rust.

---

## The core idea

```
An openEHR CDR is conventionally:

   App ──REST──▶ CDR server ──SQL──▶ Postgres ──▶ disk
                 (EHRbase etc.)      (jsonb)

anarchie collapses the middle:

   App ──REST──▶ anarchie ──▶ canonical JSON files on disk
                              (the EHR *is* the directory tree)
```

The canonical Composition JSON file is the source of truth. Everything else - the AQL query index, the REST API, the MCP server for LLMs - is a **derived, regenerable view** built from those files. Delete the index and rebuild it; the patient data is untouched because it never lived in the index.

This is the same onion model as `sct`: a stable, versionable, greppable artefact at the centre, with disposable performance layers wrapped around it.

---

## Why this might actually work for openEHR (better than for most domains)

openEHR is unusually well-suited to flat-file persistence, for reasons that are accidents of its design:

1. **Compositions are already documents.** Unlike a normalised relational schema, an openEHR Composition is a complete, self-describing tree. It does not need joining to be meaningful. It maps 1:1 to a file.
2. **There is a canonical serialisation.** openEHR defines canonical JSON (and XML). Two systems serialising the same Composition produce byte-comparable output. That makes files diffable and hashable.
3. **The data is immutable and versioned by design.** openEHR never updates a Composition in place; it creates a new `ORIGINAL_VERSION`. Immutable versions map perfectly onto immutable files and onto git commits.
4. **The contribution/audit model maps onto version control.** A `CONTRIBUTION` is an audited set of versions committed together - which is exactly what a git commit is. The committer, timestamp, and change-type already exist in both models.
5. **The schema lives outside the data.** Templates (OPTs) define structure; the RM defines the substrate. Neither is stored per-record, so the files stay lean.

If you wanted to design a clinical data standard that could be stored as flat files, you would end up with something very close to openEHR.

---

## What the hard parts are (being honest up front)

This is where the `sct` analogy gets stretched. SNOMED CT is **read-mostly reference data**; a CDR is a **read-write transactional store of patient data**. That raises problems `sct` never had to solve:

- **Validation.** Committing a Composition requires validating it against its Operational Template and the Reference Model. **Decision: this is reimplemented natively in Rust** (no JVM), so `anarchie` stays a single binary. See [specs/validation.md](specs/validation.md).
- **AQL.** Archetype Query Language is the standard query interface. Translating AQL over a pile of JSON files into something fast is the central engineering challenge. See [specs/query-engine.md](specs/query-engine.md).
- **Concurrency and transactions.** Two writers committing to the same versioned object need optimistic locking. **Decision: git is the versioning mechanism** - a `CONTRIBUTION` is a commit - which also gives us a familiar, battle-tested merge/conflict model and overlaps with the sibling [`gitehr`](https://gitehr.org/) project. See [specs/versioning-and-git.md](specs/versioning-and-git.md).
- **Scale.** Flat files are delightful at 10k compositions and questionable at 100M. We should be explicit about the target envelope. See [specs/scaling.md](specs/scaling.md).

The spec does not hand-wave these. Each has its own document.

---

## Proposed shape (single binary, layered subcommands)

Mirroring `sct`'s pluripotent-subcommands style, everything compiles into one `anarchie` binary:

```
anarchie init                 scaffold a new file-backed CDR repository
anarchie template add <opt>   register an Operational Template (the schema)
anarchie commit <comp.json>   validate + store a Composition as a new version
anarchie log <ehr_id>         show the version/contribution history of an EHR
anarchie cat <version_uid>    print a specific Composition version
anarchie index                build the derived query index (SQLite/DuckDB)
anarchie aql "SELECT ..."     run an AQL query against the index
anarchie validate <comp>      validate a Composition without committing
anarchie serve                expose the openEHR REST API over localhost
anarchie mcp                  stdio MCP server for LLM/agent access
anarchie diff <v1> <v2>       structural diff between two Composition versions
anarchie fsck                 integrity-check the repository against the RM
```

The artefact at the centre - the directory of canonical Composition JSON - is queryable with `jq`, `ripgrep`, `git log`, and `duckdb` without any custom binary, exactly as `sct`'s NDJSON is.

---

## Status

📐 **Design phase.** Nothing is built. The specs in [specs/](specs/) capture the intended architecture, the open questions, and the deliberately-out-of-scope items. Feedback and challenge welcome before any code is written.

## Documents

- [specs/architecture.md](specs/architecture.md) - the onion model, layers, and on-disk layout
- [specs/on-disk-format.md](specs/on-disk-format.md) - exact directory and file conventions
- [specs/versioning-and-git.md](specs/versioning-and-git.md) - mapping contributions/versions onto git
- [specs/reference-model-coverage.md](specs/reference-model-coverage.md) - which RM types are implemented vs deferred
- [specs/serialisation-formats.md](specs/serialisation-formats.md) - canonical JSON / XML / FLAT / STRUCTURED / Web Template
- [specs/query-engine.md](specs/query-engine.md) - how AQL gets executed over flat files
- [specs/validation.md](specs/validation.md) - RM + template validation strategy
- [specs/openehr-terminology-codes.md](specs/openehr-terminology-codes.md) - the openEHR-internal code groups (change-type, lifecycle, category, ISM)
- [specs/rest-api.md](specs/rest-api.md) - openEHR REST API surface and conformance
- [specs/scaling.md](specs/scaling.md) - the performance envelope and where files stop being a good idea
- [specs/bundled-archetypes.md](specs/bundled-archetypes.md) - "batteries included": shipping a curated, licensed OPT starter set
- [specs/regulatory-context.md](specs/regulatory-context.md) - how anarchie relates to EHDS, EEHRxF, xDHR, and ESHIA
- [specs/licensing.md](specs/licensing.md) - the four-layer licensing approach (code / specs / archetypes / terminology)
- [specs/roadmap.md](specs/roadmap.md) - phased path from spec to a usable CDR

---

## Licensing

`anarchie` combines four kinds of material under four different licences. The full detail is in [specs/licensing.md](specs/licensing.md); in short:

| Layer | Material | Licence |
|---|---|---|
| **Code** | the `anarchie` binary and our source | **AGPL-3.0-or-later** |
| **Specifications** | openEHR RM/AOM/ADL/OPT/AQL | **CC-BY-ND 3.0** - *implemented, never redistributed* |
| **Archetypes / OPTs** | bundled openEHR International models | **CC-BY-SA 3.0** - in a separate directory with attribution |
| **Terminology** | SNOMED CT, LOINC, ICD content | **not bundled** - only the *bindings* ship; bring your own terminology licence |

> **anarchie bundles archetype terminology bindings only. It does not include, and you must separately license, any clinical terminology (SNOMED CT, LOINC, ICD, etc.).**

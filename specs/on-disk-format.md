# On-Disk Format

The on-disk layout *is* the database. This document fixes the directory and file conventions so that the store is predictable, greppable, and stable enough to be a public interface. The guiding rule, inherited from `sct`: **a human with `ls`, `cat`, `jq`, and `git log` should be able to understand the whole store without `anarchie` installed.**

---

## Repository topology

One git repository per EHR (per patient) - see [versioning-and-git.md](versioning-and-git.md) for why. A *deployment* is a directory of such repositories plus shared, non-patient configuration.

```
my-cdr/                              ← an anarchie deployment root
├── anarchie.toml                    ← deployment config (system_id, terminology backend, …)
├── templates/                       ← registered Operational Templates (the schema), shared
│   ├── ehr-discharge-summary.v1.opt.json
│   ├── vital-signs-encounter.v2.opt.json
│   └── index.json                   ← template id → file, hash, registered-at
├── index/                           ← DERIVED, regenerable — never authoritative
│   ├── anarchie.db                  ← SQLite path index for AQL
│   ├── manifest.json                ← which EHRs exist, head versions, index freshness
│   └── .gitignore                   ← the index is not committed
└── ehrs/
    ├── 9f1c2e7a-…-uuid/             ← one git repository = one EHR
    │   ├── .git/
    │   ├── ehr.json                 ← EHR object (ehr_id, system_id, time_created)
    │   ├── ehr_status/              ← the mutable EHR_STATUS, itself versioned
    │   │   ├── status.json          ← current EHR_STATUS
    │   │   └── …                    ← history is in git, not duplicated here
    │   ├── compositions/
    │   │   ├── 8849182c-…-uuid/     ← one VERSIONED_COMPOSITION (object_id)
    │   │   │   └── composition.json ← current head version (history via git)
    │   │   └── b1d4…-uuid/
    │   │       └── composition.json
    │   ├── contributions/           ← contribution manifests (audit fast-path)
    │   │   └── 4f2a…-contrib.json
    │   └── folders/                 ← optional FOLDER hierarchy
    │       └── root.json
    └── a73b9c11-…-uuid/
        └── …
```

---

## Key conventions

### Identity and naming

- **UUIDs are directory/file stems**, lowercased, canonical hyphenated form. EHRs, versioned compositions, and contributions are all UUID-named directories or files.
- The `object_id` of a `VERSIONED_COMPOSITION` is the directory name under `compositions/`. All versions of that composition live in that one directory's git history.
- Template files are named `<template_id>.opt.json`. The `template_id` is the human-meaningful schema name.

### The working tree holds *current* state only

The file `compositions/<object_id>/composition.json` always contains the **head version**. Earlier versions are **not** kept as sibling files (`v1.json`, `v2.json`); they live in git history. This keeps the working tree clean and the "current state" trivially readable, while git remains the complete version store.

- `anarchie cat <object_id>` → reads the working-tree file (fast path for head).
- `anarchie cat <version_uid>` for an older version → `git show <commit>:<path>`.
- `anarchie log <object_id>` → `git log -- compositions/<object_id>/composition.json`.

This is a deliberate trade: the filesystem shows you *now*; git shows you *history*. It mirrors gitehr's split of a mutable `state/` from an immutable journal, but here history lives in git rather than in append-only files.

### Canonical JSON

Every `.json` clinical file is **openEHR canonical JSON**, serialised deterministically (stable key order, normalised number/whitespace formatting) so that:

- two semantically identical Compositions are byte-identical,
- `git diff` produces meaningful, minimal diffs,
- a content hash is a stable identity/integrity check.

Guaranteeing byte-stable canonical serialisation is a hard dependency of the whole design and an explicit open question in [architecture.md](architecture.md).

### Derived data is segregated and git-ignored

Everything under `index/` is rebuildable from `ehrs/` + `templates/`. It is `.gitignore`d within each context and can be deleted at any time:

```
anarchie index --rebuild        # blow away index/, walk the file tree, rebuild
```

A query against a stale index is a correctness bug, so `manifest.json` tracks index freshness (last-indexed commit per EHR) and the write path either updates the index synchronously or marks it dirty for lazy rebuild.

---

## File examples (illustrative)

### `anarchie.toml`

```toml
system_id = "anarchie.example.org"
rm_version = "1.1.0"

[terminology]
# optional; if absent, terminology bindings are not externally validated
backend = "sct"             # shells out to the `sct` binary
sct_db  = "~/snomed.db"

[index]
freshness = "synchronous"   # or "lazy"
```

### `ehr.json`

```jsonc
{
  "_type": "EHR",
  "ehr_id": { "_type": "HIER_OBJECT_ID", "value": "9f1c2e7a-…" },
  "system_id": { "_type": "HIER_OBJECT_ID", "value": "anarchie.example.org" },
  "time_created": { "_type": "DV_DATE_TIME", "value": "2026-06-18T10:00:00Z" }
}
```

### `contributions/<id>-contrib.json`

A denormalised mirror of the git commit's audit data, so the audit trail is readable without invoking git:

```jsonc
{
  "_type": "CONTRIBUTION",
  "uid": { "value": "4f2a…" },
  "commit": "git-commit-sha",
  "versions": [
    "8849182c-…::anarchie.example.org::1"
  ],
  "audit": {
    "committer": { "name": "Dr A. Smith" },
    "time_committed": "2026-06-18T10:14:22Z",
    "change_type": "creation",
    "description": "Admission vitals"
  }
}
```

---

## Why expose the raw layout at all?

The same reason `sct` exposes its NDJSON: **inspectability builds trust and enables tooling we did not write.** A researcher can `ripgrep` for a SNOMED code across a patient's record; a backup is `tar` or `git bundle`; a migration is a file walk; an LLM agent can read a Composition directly. The format being legible is a feature, not an implementation detail - so it is specified, versioned (`schema_version` in the manifest), and treated as a stable public interface.

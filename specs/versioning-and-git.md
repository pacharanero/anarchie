# Versioning and Git

> **Decision: git is the versioning mechanism.** Not an optional export, not a mirror - the git commit graph *is* the openEHR contribution and version history. `anarchie` is, at its core, a structured wrapper around `git` that happens to speak openEHR.

This is the most opinionated decision in the project, and the one that most clearly distinguishes `anarchie` from a conventional CDR (which buries versioning inside Postgres rows). It also deliberately overlaps with the sibling [`gitehr`](https://gitehr.org/) project.

---

## Why git is a natural fit for openEHR versioning

openEHR's versioning model and git's object model are describing the same shape:

| openEHR | git | Notes |
|---|---|---|
| `CONTRIBUTION` | commit | An atomic, audited set of changes |
| `AUDIT_DETAILS.committer` | commit author/committer | Who made the change |
| `AUDIT_DETAILS.time_committed` | commit timestamp | When |
| `AUDIT_DETAILS.description` | commit message | Why |
| `AUDIT_DETAILS.change_type` | (encoded in message/trailer) | creation / modification / deletion |
| `ORIGINAL_VERSION` | a file blob at a commit | One immutable version of a Composition |
| `VERSIONED_OBJECT.version history` | `git log -- <path>` | The chain of versions of one object |
| `preceding_version_uid` | parent commit (for that path) | The "this supersedes that" link |
| `version_tree_id` (e.g. `1`, `1.1.1`) | branch/lineage | Branching of versions |

openEHR even uses the language of version *trees* and *branches*, with `version_tree_id`s like `1.1.1` denoting a branch off version 1. Git branches are the obvious home for this, though the MVP can keep to linear `1, 2, 3` lineage on a single branch.

---

## Repository = EHR? Or repository = population?

Two viable topologies, with a real trade-off:

### Option A — one git repository per EHR (per patient)

```
ehrs/
  9f1c…uuid/            ← a git repository
    .git/
    ehr_status.json
    compositions/…
```

- **Pro:** maps cleanly to gitehr ("a patient record is a repository"), strong per-patient isolation, trivially portable (clone one patient), natural access-control boundary, no cross-patient merge contention.
- **Con:** population AQL must read across thousands of repos; the derived index becomes essential (you cannot `git log` your way across patients).

### Option B — one git repository for the whole CDR

```
cdr/                    ← a single git repository
  .git/
  ehrs/
    9f1c…uuid/…
    a73b…uuid/…
```

- **Pro:** one history, simple backup, population queries can in principle walk one repo.
- **Con:** does not match gitehr; one giant repo with millions of files strains git; commits from concurrent patients contend on a single HEAD.

**Leaning: Option A (repo-per-EHR)** for conceptual cleanliness and gitehr compatibility, accepting that population queries *require* the derived index (which we need anyway). The index is what makes the "many small repos" topology query-able. See [scaling.md](scaling.md) for where this breaks down.

---

## A commit = a CONTRIBUTION

The unit of write is the contribution. Committing one or several Compositions together produces exactly one git commit:

1. Write/replace the canonical JSON file(s) for each new version in the working tree.
2. Stage them.
3. Commit with structured metadata:

```
contribution: 3 versions

Author: Dr A. Smith <a.smith@example.nhs.uk>
Date:   2026-06-18T10:14:22Z

anarchie-contribution-id: 4f2a…
anarchie-change-type: creation
anarchie-system-id: anarchie.example.org
```

The commit hash becomes a stable contribution identifier. `AUDIT_DETAILS` is reconstructed from the commit object; no separate audit store is needed (though a manifest file may duplicate it for fast indexing without invoking git).

---

## Version identifiers

An openEHR `version_uid` is `object_id::creating_system_id::version_tree_id`, e.g.:

```
8849182c-82ad-4088-a07f-48ead4180515::anarchie.example.org::1
```

- **`object_id`** - stable across all versions of one Composition (the versioned object's identity). This is the directory/file stem on disk.
- **`creating_system_id`** - the `anarchie` instance identity, from config.
- **`version_tree_id`** - `1`, then `2`, then `3` for linear edits; `1.1.1` for a branch. `anarchie` derives the next id by inspecting the existing head version of that object (via the index, with git as the source of truth).

Mapping to git: the `(object_id, version_tree_id)` pair resolves to a specific blob at a specific commit. `anarchie cat <version_uid>` becomes a `git show <commit>:<path>` under the hood.

---

## Optimistic concurrency

The openEHR REST API supports `If-Match` preconditions to prevent lost updates. `anarchie` enforces this against git:

- A write of a new version asserts the caller's known `preceding_version_uid` equals the current head version of that object.
- If another contribution has advanced the object in the meantime, the precondition fails and the write is rejected with `412 Precondition Failed` - the openEHR-conformant response. The caller must re-read and retry.
- Internally this is a compare-and-swap against the object's current head commit before committing.

True git *merges* of divergent version branches are **out of scope for the MVP**. Conflicting concurrent writes are rejected, not auto-merged. Branch/merge of version trees is a later, advanced feature (and the most interesting place where gitehr's multi-contributor sync model could be borrowed).

---

## What git gives us for free

- **Complete, tamper-evident history** - the audit trail is the commit graph, cryptographically chained.
- **Time-travel** - `version at time` queries map to "the blob as of the commit at-or-before time T".
- **Portability** - a patient record is `git clone`-able; an export is a bundle.
- **Backup and replication** - `git push` to a remote is a backup story (crude, but real and familiar).
- **Diffing** - `anarchie diff v1 v2` builds on `git diff` plus a structural (RM-aware) layer on top.
- **Familiarity** - every developer already understands commits, history, and blame.

---

## What we must not assume git gives us

- **Performance at population scale** - git is great per-repo, poor as a cross-patient query substrate. The derived index, not git, answers AQL. Git is the system of record; the index is the read model. (This is CQRS, with git as the event/commit log.)
- **Concurrent single-object writes** - handled by the optimistic precondition above, not by git's own locking.
- **Large binary blobs** - DICOM and similar belong outside the Composition (referenced, possibly via `git-lfs` or content-addressed external storage), exactly as gitehr keeps Documents as first-class file objects.

---

## Convergence with gitehr (future, non-binding)

Because both projects commit clinical data to git, a single patient repository could one day carry **both** views: gitehr's human-readable journal/state narrative *and* anarchie's RM-structured Compositions, sharing one commit history. Keeping anarchie's per-EHR layout git-clean and its commit-trailer conventions documented makes that convergence cheap to attempt later without committing to it now.

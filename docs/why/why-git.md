# Why git?

A CDR is a read-write store of patient data, so it needs versioning,
concurrency control, and an audit trail. `anarchie` does not invent any of that
- it uses **git**, because openEHR's versioning model already lines up with
git's almost exactly.

## The model alignment

| openEHR concept            | git concept                                  |
| -------------------------- | -------------------------------------------- |
| `CONTRIBUTION`             | a commit                                     |
| `AUDIT_DETAILS`            | commit author, committer, timestamp, message |
| committer / time_committed | `%an %ae` / `%cI`                            |
| immutable `ORIGINAL_VERSION` | an immutable blob at a commit              |
| version history of an object | `git log -- <path>`                        |
| optimistic concurrency     | git's merge / conflict model                 |

A `CONTRIBUTION` is an audited set of versions committed together. That *is* a
git commit. So `anarchie` writes the canonical files, stamps the audit identity
into the commit, adds a few trailers to carry the openEHR-specific ids, and the
version history falls out for free.

## One repository per EHR

Each EHR is its own git repository. That keeps a patient's history
self-contained and portable - you can clone, back up, or hand over a single
record without touching the rest of the store - and it keeps `git log` for any
object scoped to just that patient.

## System git, not a library

`anarchie` shells out to the system `git` binary rather than embedding a git
library such as libgit2. Two reasons:

- **The store stays an ordinary git repository.** Anything git can do - `git
  log`, `git show`, `git bisect`, `git fsck`, your existing backup and mirroring
  tooling - works on an `anarchie` store unchanged.
- **The binary stays light.** No large native dependency; the only runtime
  requirement is the `git` you already have.

## What this gives you today

- Time-travel: reconstruct any historical version with `anarchie cat <ehr>
  <version_uid>` (a `git show` under the hood).
- Meaningful diffs: `anarchie diff <ehr> <object_id> <a> <b>` over canonical
  files shows exactly what changed clinically.
- A real, inspectable audit trail: the committer, timestamp, and change-type of
  every contribution live in the commit graph.

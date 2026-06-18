# Versioning and Git

`anarchie` uses git as its versioning mechanism. This page records the mapping
between the openEHR model and git, and the conventions `anarchie` follows.

## openEHR to git

| openEHR concept                | git mechanism                                  |
| ------------------------------ | ---------------------------------------------- |
| `CONTRIBUTION`                 | a single commit                                |
| `AUDIT_DETAILS.committer`      | commit author/committer name and email         |
| `AUDIT_DETAILS.time_committed` | commit timestamp (`%cI`)                       |
| `AUDIT_DETAILS.description`    | commit subject                                 |
| `AUDIT_DETAILS.change_type`    | `anarchie-change-type` trailer                 |
| immutable `ORIGINAL_VERSION`   | an immutable blob at a commit                  |
| version history of an object   | `git log -- <path>`                            |
| concurrency control            | git's merge / conflict model                   |

## Commit shape

Each contribution is one commit. `anarchie` forces a deterministic author and
committer identity from the supplied audit, writes the canonical files, and adds
trailers that carry the openEHR-specific identifiers:

```text
anarchie-contribution-id: <uuid>
anarchie-change-type:      creation | modification | deletion
anarchie-system-id:        <system_id>
```

## The contribution-to-commit link

A `CONTRIBUTION` references the commit that recorded it, but a commit cannot
contain its own hash. So the contribution manifest deliberately omits its commit
sha, and the link runs the other way: the commit carries the
`anarchie-contribution-id` trailer, and the relationship is resolved at read
time. This keeps the manifest content stable and the commit self-consistent.

## System git, not a library

`anarchie` shells out to the system `git` binary rather than embedding libgit2.
The store stays an ordinary git repository - usable with any git tooling,
backup, or mirror - and the `anarchie` binary stays light. See
[Why git?](../why/why-git.md).

The full design rationale is in
[specs/versioning-and-git.md](https://github.com/pacharanero/anarchie/blob/main/specs/versioning-and-git.md).

<!--
SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Agent Instructions

`anarchie` is an offline-first, files-based openEHR Clinical Data Repository (CDR) and toolset, written as a single Rust crate (library + `anarchie` binary). It also deliberately grows a reusable Rust openEHR SDK, with GitEHR as its intended first external consumer. The canonical Composition JSON file on disk is the system of record; everything else (the AQL index, the REST API, the MCP server) is a derived, regenerable view. It is experimental and explicitly **not for clinical use**.

This file is the entry point for AI coding agents. Read it before changing anything. It is the source of truth; `AGENTS.md` and `CLAUDE.md` are thin pointers to it.

## Read First

- [README.md](README.md) - what anarchie is, the core idea, setup, and the four-layer licensing.
- [specs/roadmap.md](specs/roadmap.md) - current state and remaining work, with `[x]`/`[~]`/`[ ]` status.
- [specs/rust-sdk.md](specs/rust-sdk.md) - the reusable Rust SDK boundary, extraction trigger, and GitEHR integration goal.
- [specs/](specs/) - durable design decisions (architecture, on-disk format, validation, conformance, knowledge packages, versioning-and-git, query-engine, licensing, ips-readiness, regulatory-context).
- [docs/walkthrough/](docs/walkthrough/) - the guided feature tour; keep it runnable (see the invariant below).
- [~/code/house-style/AGENTS.md](~/code/house-style/AGENTS.md) - cross-repo engineering standards.

## Core Invariants

- **The canonical file is the source of truth.** One immutable canonical-JSON file per Composition version. The index (`index/`) is a disposable derived view - never the system of record, and `.gitignore`d.
- **The SDK kernel stays host-independent.** `rm`, `aom`, `opt`, and `validate` must not depend on the filesystem, git, SQLite, HTTP/MCP, a CLI, a clock, or global state. The CDR is their first consumer; GitEHR is the intended first external consumer. See [rust-sdk.md](specs/rust-sdk.md).
- **Single binary, no runtime.** The shipped artefact depends only on the system `git`. No JVM, no database server ever ships. Archie and EHRbase are *test-time oracles only*, never a runtime dependency.
- **Validation at the door.** Every commit is validated (RM + Operational Template) before anything is written to git; non-conformant data must never reach the store. `--no-validate` is the only documented bypass.
- **Canonical serialisation is byte-stable.** Parse -> canonicalise round-trips without drift; equal Compositions serialise byte-for-byte equal. Do not break this - it underpins diffing, hashing, and git stability.
- **The four-layer licensing split is maintained** (code AGPL-3.0-or-later / anarchie's own prose CC-BY-SA-4.0 / bundled CKM artefacts under source-declared CC-BY-SA terms with the current starter derivatives at 3.0 / openEHR specs CC-BY-ND, not redistributed). See [specs/licensing.md](specs/licensing.md). Keep `reuse lint` green.
- **The walkthrough stays runnable.** `docs/walkthrough/` commands must work against the real binary; `examples/vitals.json` is guarded by a test to stay in step with the fixture. If you change CLI output, update the walkthrough in the same change.

## Workflow

- `s/build` - build the release binary.
- `s/test` - run the full test suite (forwards args, e.g. `s/test <name>`).
- `s/lint` - `cargo fmt --all --check` + `cargo clippy --all-targets -- -D warnings`.
- `s/docs` - serve the docs site locally (binds the first free port in 8000-8030).
- `s/install` - install/reinstall the binary from this checkout.
- `s/install-hooks` - opt in to the pre-commit hook that runs `s/lint`.

## Before Every Commit

```sh
s/lint          # cargo fmt --all --check + cargo clippy --all-targets -- -D warnings
s/test          # cargo test
reuse lint      # SPDX/REUSE compliance
```

CI enforces all of these plus a coverage job. Do not commit red. Use conventional commits (`feat(area):`, `fix:`, `docs:`, `ci:`, `chore:`, `refactor:`); the `-m` message is the commit subject.

## Approval Required

Ask before taking externally visible or hard-to-reverse actions: publishing a release or crates.io version, pushing a `vX.Y.Z` tag, force-pushing, deleting branches, changing repository secrets, or running anything against third-party systems. Committing to `main` is the normal flow for this early-stage project, but say what you are about to push.

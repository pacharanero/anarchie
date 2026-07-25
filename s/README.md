<!-- SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd -->
<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# `s/`

Convenience scripts for the repeated processes of working on anarchie. Each names one process and runs from anywhere in the checkout. `ls s/` is the verb list.

## `s/lint`

The formatting and lint checks CI enforces - `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`. Run before committing.

## `s/test`

The full test suite (`cargo test`). Forwards arguments: `s/test <name>` runs one test, `s/test --release` tests the release build.

## `s/aql-conformance`

Run the official openEHR AQL grammar against anarchie's conformance corpus. This requires Java, `javac`, `curl`, and `sha256sum`; it downloads all third-party artefacts into a temporary directory and leaves none in the checkout. CI runs it automatically.

## `s/ehrbase`

Manage the disposable local EHRbase interoperability oracle: `s/ehrbase up`, `s/ehrbase down`, `s/ehrbase logs`, or `s/ehrbase status`. `s/ehrbase test` rebuilds a clean oracle, loads the shared synthetic blood-pressure Composition into EHRbase and anarchie, compares AQL results, then tears the oracle down. It requires Docker Compose, binds only to localhost, and is for synthetic test data only. See [`tests/ehrbase/README.md`](../tests/ehrbase/README.md).

## `s/build`

A release build of the `anarchie` binary (`cargo build --release`).

## `s/install`

Install the local build onto your `PATH` (`cargo install --path .`). Forwards arguments, e.g. `s/install --locked`.

## `s/install-hooks`

Opt in to the tracked pre-commit hook in `.githooks/` that runs `s/lint`. One-off, per checkout.

## `s/docs`

Serve the documentation site locally with live reload (`zensical serve`).

## `s/version++`

The one release action: `s/version++ [patch|minor|major]` (default `patch`). Runs the CI checks, bumps the version, regenerates `CHANGELOG.md` (git-cliff), commits `chore(release): vX.Y.Z`, tags it, and pushes `main` + the tag. The tag triggers the cargo-dist release (prebuilt binaries, GitHub Release, Homebrew) and the crates.io publish. Commit feature work first; releases are cut from a clean `main`.

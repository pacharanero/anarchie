<!-- SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd -->
<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# `s/`

Convenience scripts for the repeated processes of working on anarchie. Each names one process and runs from anywhere in the checkout. `ls s/` is the verb list.

## `s/lint`

The formatting and lint checks CI enforces - `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`. Run before committing.

## `s/test`

The full test suite (`cargo test`). Forwards arguments: `s/test <name>` runs one test, `s/test --release` tests the release build.

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

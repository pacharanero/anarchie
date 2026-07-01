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

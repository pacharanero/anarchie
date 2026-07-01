// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Thin entry point for the `anarchie` binary. All behaviour lives in the
//! [`anarchie::cli`] module so it can be embedded in other tools.

fn main() -> std::process::ExitCode {
    anarchie::cli::run()
}

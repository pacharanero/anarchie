#!/bin/sh
# SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# anarchie installer.
#
#   curl -LsSf https://pacharanero.github.io/anarchie/install.sh | sh
#
# This is the interim installer: it builds anarchie from source with Cargo, so
# it needs a Rust toolchain (https://rustup.rs). A prebuilt-binary installer
# that needs no toolchain is on the roadmap (see the installation docs). The
# install *command* above is stable - only its implementation will change.

set -eu

REPO="https://github.com/pacharanero/anarchie"
CRATE="anarchie-cli" # the package that provides the `anarchie` binary

if ! command -v cargo >/dev/null 2>&1; then
	cat >&2 <<EOF
error: this installer currently builds anarchie from source and needs Rust/Cargo.

  1. install Rust:  https://rustup.rs
  2. re-run:        curl -LsSf https://pacharanero.github.io/anarchie/install.sh | sh
     or directly:   cargo install --git $REPO $CRATE --locked

A prebuilt-binary installer (no toolchain required) is on the roadmap.
EOF
	exit 1
fi

echo "Installing anarchie (building from source via cargo)..."
cargo install --git "$REPO" "$CRATE" --locked
echo "Done. Run 'anarchie --help' to get started."

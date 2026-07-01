// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! The service commands: `serve` (the openEHR REST API) and `mcp` (the stdio
//! MCP server for LLM agents). Both run until interrupted; `--format` does not
//! apply.

use anyhow::{Context, Result};

use super::open_deployment;

pub(crate) fn serve(addr: &str) -> Result<()> {
    let deployment = open_deployment()?;
    crate::serve::serve(deployment, addr).context("running REST server")
}

pub(crate) fn mcp() -> Result<()> {
    let deployment = open_deployment()?;
    crate::serve::run_mcp(deployment).context("running MCP server")
}

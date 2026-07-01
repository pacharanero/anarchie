// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! # anarchie-serve
//!
//! The service front ends for `anarchie`: a conformant-subset openEHR **REST
//! API** (`anarchie serve`) and a stdio **MCP server** (`anarchie mcp`). Both
//! are thin, stateless translations onto the store and query engine via the
//! shared [`ops`] layer - the outer layer of the onion. See `specs/rest-api.md`.

mod mcp;
mod ops;
mod rest;

pub use mcp::run as run_mcp;
pub use rest::serve;

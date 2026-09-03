// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! AQL (Archetype Query Language) for the MVP subset.
//!
//! Parsing belongs to the `openehr-query` SDK crate; [`lower`] narrows its
//! syntax tree to [`ast`], the model the executor runs.

mod ast;
mod lower;

pub use ast::*;
pub use lower::parse;

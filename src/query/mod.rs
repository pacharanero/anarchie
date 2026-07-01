// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! # anarchie-query
//!
//! The read model and query engine for `anarchie`: a path-extraction **index**
//! over the canonical Composition files, and an **AQL** engine that translates
//! a useful subset of Archetype Query Language into SQL over that index. The
//! index is derived and disposable - the files remain the system of record. See
//! `specs/query-engine.md`.

mod aql;
mod error;
mod execute;
mod index;
pub mod stored;

pub use aql::{parse, AqlQuery};
pub use error::{QueryError, Result};
pub use execute::{execute, Column, Params, ResultSet};
pub use index::Index;

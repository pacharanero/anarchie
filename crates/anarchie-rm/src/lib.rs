// SPDX-License-Identifier: AGPL-3.0-or-later
//! # anarchie-rm
//!
//! openEHR Reference Model (RM) types with [openEHR canonical JSON][canonical]
//! (de)serialisation via `serde`.
//!
//! This crate is the foundation of the `anarchie` flat-file CDR: every
//! Composition on disk is canonical JSON, so faithful, byte-stable
//! serialisation is the hard dependency everything else builds on.
//!
//! [canonical]: https://specifications.openehr.org/releases/RM/latest/

pub mod ty;

mod common;
mod composition;
mod data_structures;
mod data_values;
mod ehr;
mod history;
mod support;
mod text;

pub use common::*;
pub use composition::*;
pub use data_structures::*;
pub use data_values::*;
pub use ehr::*;
pub use history::*;
pub use support::*;
pub use text::*;

/// Errors arising from canonical JSON (de)serialisation.
#[derive(Debug, thiserror::Error)]
pub enum RmError {
    /// The bytes were not valid canonical JSON for the requested RM type.
    #[error("canonical JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Parse a value of an RM type from openEHR canonical JSON.
pub fn from_canonical_str<T: serde::de::DeserializeOwned>(json: &str) -> Result<T, RmError> {
    Ok(serde_json::from_str(json)?)
}

/// Serialise an RM value to canonical JSON.
///
/// The canonical form is deterministic, pretty-printed (two-space indent), and
/// terminated by a single newline so that files are diff-friendly under git and
/// the operation is idempotent: re-serialising a parsed canonical document
/// reproduces it byte-for-byte.
pub fn to_canonical_string<T: serde::Serialize>(value: &T) -> Result<String, RmError> {
    let mut out = serde_json::to_string_pretty(value)?;
    out.push('\n');
    Ok(out)
}

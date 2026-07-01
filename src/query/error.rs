// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error types for the query engine.

use thiserror::Error;

/// Errors arising from indexing or querying.
#[derive(Debug, Error)]
pub enum QueryError {
    /// The SQLite index could not be opened or written.
    #[error("index database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// A Composition could not be read from the store.
    #[error("store error: {0}")]
    Store(#[from] crate::store::StoreError),

    /// A Composition's JSON was malformed.
    #[error("composition JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The AQL text could not be tokenised or parsed.
    #[error("AQL parse error: {0}")]
    Parse(String),

    /// The parsed AQL referenced something the MVP translator cannot handle.
    #[error("unsupported AQL: {0}")]
    Unsupported(String),

    /// A `$`-parameter referenced in the query was not supplied.
    #[error("missing query parameter: ${0}")]
    MissingParameter(String),

    /// A stored query was requested by a name that is not registered.
    #[error("stored query `{0}` not found")]
    StoredQueryNotFound(String),

    /// I/O error reading or writing stored queries.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience alias for query-engine results.
pub type Result<T> = std::result::Result<T, QueryError>;

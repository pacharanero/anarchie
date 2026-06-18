// SPDX-License-Identifier: AGPL-3.0-or-later
//! Error types for the store.

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("git command `{command}` failed (exit {code}): {stderr}")]
    Git {
        command: String,
        code: i32,
        stderr: String,
    },

    #[error("could not run git: {0}")]
    GitSpawn(#[source] std::io::Error),

    #[error("not an anarchie deployment: no anarchie.toml found at or above {0}")]
    NotADeployment(PathBuf),

    #[error("deployment already exists at {0}")]
    AlreadyExists(PathBuf),

    #[error("EHR `{0}` not found")]
    EhrNotFound(String),

    #[error("composition `{0}` not found")]
    CompositionNotFound(String),

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("malformed clinical content: {0}")]
    Rm(#[from] anarchie_rm::RmError),

    #[error("composition failed validation with {} error(s)", .0.error_count())]
    Invalid(anarchie_validate::ValidationReport),

    #[error("template error: {0}")]
    Opt(#[from] anarchie_validate::OptError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML serialise error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("TOML parse error: {0}")]
    TomlDe(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, StoreError>;

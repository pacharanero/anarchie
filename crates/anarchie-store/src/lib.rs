// SPDX-License-Identifier: AGPL-3.0-or-later
//! # anarchie-store
//!
//! The git-backed flat-file store for `anarchie`. A deployment is a directory
//! of one-git-repository-per-EHR plus shared configuration and templates; a
//! CONTRIBUTION is a git commit. See `specs/on-disk-format.md` and
//! `specs/versioning-and-git.md`.

mod config;
mod deployment;
mod error;
mod git;
mod starter;
mod template;

pub use config::{DeploymentConfig, IndexConfig, TerminologyConfig};
pub use deployment::{
    now_iso8601, Audit, ChangeType, CommitOutcome, ContributionManifest, Deployment, EhrRepo,
    LogEntry,
};
pub use error::{Result, StoreError};
pub use git::Git;
pub use starter::starter_template_ids;

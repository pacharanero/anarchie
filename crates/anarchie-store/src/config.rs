// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deployment-level configuration, persisted as `anarchie.toml`.

use serde::{Deserialize, Serialize};

/// The deployment configuration file (`anarchie.toml`) at the deployment root.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// The creating-system identity, used in every `version_uid`.
    pub system_id: String,
    /// openEHR RM version this deployment stores.
    pub rm_version: String,
    #[serde(default)]
    pub terminology: Option<TerminologyConfig>,
    #[serde(default)]
    pub index: IndexConfig,
}

impl DeploymentConfig {
    pub fn new(system_id: impl Into<String>) -> Self {
        Self {
            system_id: system_id.into(),
            rm_version: "1.1.0".to_string(),
            terminology: None,
            index: IndexConfig::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminologyConfig {
    /// e.g. `"sct"` to shell out to the `sct` binary.
    pub backend: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sct_db: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexConfig {
    /// `"synchronous"` updates the index on write; `"lazy"` marks it dirty.
    pub freshness: String,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            freshness: "synchronous".to_string(),
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Operational Template registration: the schema, stored as data.
//!
//! Templates are registered into a deployment with `anarchie template add` and
//! kept under `templates/<template_id>.opt.json`, with `templates/index.json`
//! listing the registered ids. They are parsed once into the AOM constraint
//! model and consulted at commit time to validate the Compositions that claim
//! them. Like the EHR data itself, the schema lives in plain files.

use std::fs;
use std::path::PathBuf;

use anarchie_validate::Opt;

use crate::deployment::Deployment;
use crate::error::{Result, StoreError};

impl Deployment {
    /// The directory holding registered Operational Templates.
    pub fn templates_dir(&self) -> PathBuf {
        self.root().join("templates")
    }

    fn template_path(&self, template_id: &str) -> PathBuf {
        self.templates_dir().join(format!("{template_id}.opt.json"))
    }

    /// Register (or replace) an Operational Template, returning its id.
    pub fn add_template(&self, opt: &Opt) -> Result<String> {
        let dir = self.templates_dir();
        create_dir(&dir)?;
        let path = self.template_path(&opt.template_id);
        write_file(&path, &opt.to_json()?)?;
        self.rewrite_template_index()?;
        Ok(opt.template_id.clone())
    }

    /// List the registered template ids.
    pub fn list_templates(&self) -> Result<Vec<String>> {
        let dir = self.templates_dir();
        let mut ids = Vec::new();
        if !dir.exists() {
            return Ok(ids);
        }
        let entries = fs::read_dir(&dir).map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
        for entry in entries {
            let entry = entry.map_err(|source| StoreError::Io {
                path: dir.clone(),
                source,
            })?;
            if let Some(name) = entry.file_name().to_str() {
                if let Some(id) = name.strip_suffix(".opt.json") {
                    ids.push(id.to_string());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }

    /// Load a registered template by id, if present.
    pub fn get_template(&self, template_id: &str) -> Result<Option<Opt>> {
        let path = self.template_path(template_id);
        if !path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(&path).map_err(|source| StoreError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(Some(Opt::from_json(&json)?))
    }

    fn rewrite_template_index(&self) -> Result<()> {
        let ids = self.list_templates()?;
        let index = serde_json::json!({ "templates": ids });
        let mut body = serde_json::to_string_pretty(&index)?;
        body.push('\n');
        write_file(&self.templates_dir().join("index.json"), &body)
    }
}

fn create_dir(path: &std::path::Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn write_file(path: &std::path::Path, contents: &str) -> Result<()> {
    fs::write(path, contents).map_err(|source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}

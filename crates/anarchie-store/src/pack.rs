// SPDX-License-Identifier: AGPL-3.0-or-later
//! Archetype packs: installable sets of Operational Templates.
//!
//! The bundling mechanism behind the starter set ([`crate::starter`]) extends
//! naturally to *packs* - named, versioned sets of OPTs you can add to a
//! deployment, echoing `sct`'s codelist model. The MVP supports the bundled
//! `ips-core` pack (the starter set) and local directory packs (every
//! `*.opt.json` in a folder); a networked registry / `kam` integration is the
//! later step. See `specs/bundled-archetypes.md` and `specs/roadmap.md`.

use std::fs;
use std::path::Path;

use anarchie_validate::Opt;

use crate::deployment::Deployment;
use crate::error::{Result, StoreError};

/// The bundled packs available by name (those compiled into the binary).
pub fn bundled_packs() -> &'static [&'static str] {
    &["ips-core"]
}

impl Deployment {
    /// Install an archetype pack of Operational Templates: either a bundled
    /// pack by name (e.g. `ips-core`) or every `*.opt.json` in a local
    /// directory. Returns the installed template ids.
    pub fn install_pack(&self, source: &str) -> Result<Vec<String>> {
        match source {
            // `ips-core` is the bundled IPS-aligned starter set.
            "ips-core" | "starter" => self.install_starter_templates(),
            dir => self.install_pack_from_dir(Path::new(dir)),
        }
    }

    fn install_pack_from_dir(&self, dir: &Path) -> Result<Vec<String>> {
        if !dir.is_dir() {
            return Err(StoreError::Config(format!(
                "pack `{}` is neither a bundled pack name nor a directory",
                dir.display()
            )));
        }
        let read = fs::read_dir(dir).map_err(|source| StoreError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let mut paths: Vec<_> = read
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.to_str().is_some_and(|s| s.ends_with(".opt.json")))
            .collect();
        paths.sort();

        let mut installed = Vec::new();
        for path in paths {
            let json = fs::read_to_string(&path).map_err(|source| StoreError::Io {
                path: path.clone(),
                source,
            })?;
            let opt = Opt::from_json(&json)?;
            installed.push(self.add_template(&opt)?);
        }
        Ok(installed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeploymentConfig;

    #[test]
    fn installs_the_bundled_ips_core_pack() {
        let dir = tempfile::tempdir().unwrap();
        let dep = Deployment::init(dir.path(), DeploymentConfig::new("t")).unwrap();
        let installed = dep.install_pack("ips-core").unwrap();
        assert!(installed.contains(&"vital_signs_encounter.v1".to_string()));
    }

    #[test]
    fn installs_a_local_directory_pack() {
        let src = tempfile::tempdir().unwrap();
        std::fs::write(
            src.path().join("mini.opt.json"),
            r#"{"template_id":"mini.v1","concept":"openEHR-EHR-COMPOSITION.encounter.v1",
                "definition":{"type":"COMPLEX","rm_type":"COMPOSITION","node_id":"openEHR-EHR-COMPOSITION.encounter.v1"}}"#,
        )
        .unwrap();

        let dep_dir = tempfile::tempdir().unwrap();
        let dep = Deployment::init(dep_dir.path(), DeploymentConfig::new("t")).unwrap();
        let installed = dep.install_pack(src.path().to_str().unwrap()).unwrap();
        assert_eq!(installed, vec!["mini.v1".to_string()]);
        assert!(dep
            .list_templates()
            .unwrap()
            .contains(&"mini.v1".to_string()));
    }

    #[test]
    fn unknown_pack_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let dep = Deployment::init(dir.path(), DeploymentConfig::new("t")).unwrap();
        assert!(dep.install_pack("/no/such/pack").is_err());
    }
}

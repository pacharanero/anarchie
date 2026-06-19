// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bundled "batteries-included" starter Operational Templates.
//!
//! A reason openEHR adoption stalls is that a fresh CDR is *empty*: no models,
//! no templates, nothing you can store until you have authored an archetype. To
//! mirror SQLite's "it just works" ergonomics, `anarchie init` installs a small
//! curated set of Operational Templates by default, so a newcomer can store
//! real clinical data in the first five minutes (`anarchie init --minimal`
//! opts out). See `specs/bundled-archetypes.md`.
//!
//! The OPT JSON files live under `starter/templates/` and are embedded into the
//! binary with [`include_str!`], so the bundle ships with the single executable
//! and needs no companion files at runtime. The templates are derived from
//! openEHR International (CKM) Published archetypes, which are licensed CC-BY-SA
//! 3.0 and may be redistributed and derived from with attribution.

use std::fs;

use anarchie_validate::Opt;

use crate::deployment::Deployment;
use crate::error::{Result, StoreError};

/// CC-BY-SA 3.0 attribution and provenance for the bundled models, written
/// alongside the installed templates so the ShareAlike notice travels with the
/// (derivative) data. Segregated from the AGPL-licensed `anarchie` code.
const ATTRIBUTION: &str = include_str!("starter/templates/attribution.md");

/// One bundled template: its canonical OPT JSON, embedded at compile time.
struct StarterTemplate {
    /// The expected `template_id`, kept alongside the JSON for stable ordering
    /// and readability. The authoritative id is the one inside the JSON.
    id: &'static str,
    json: &'static str,
}

/// The Tier 1 "Core" starter set: the default `anarchie init` templates. These
/// span the universal vital-signs encounter plus the IPS-style record sections
/// a general-purpose CDR should store on day one.
const STARTER_TEMPLATES: &[StarterTemplate] = &[
    StarterTemplate {
        id: "vital_signs_encounter.v1",
        json: include_str!("starter/templates/vital-signs-encounter.opt.json"),
    },
    StarterTemplate {
        id: "problem_list.v1",
        json: include_str!("starter/templates/problem-list.opt.json"),
    },
    StarterTemplate {
        id: "adverse_reaction_list.v1",
        json: include_str!("starter/templates/adverse-reaction-list.opt.json"),
    },
];

/// The `template_id`s of the bundled starter templates, in install order.
pub fn starter_template_ids() -> Vec<&'static str> {
    STARTER_TEMPLATES.iter().map(|t| t.id).collect()
}

impl Deployment {
    /// Install the bundled starter templates into this deployment, returning the
    /// installed template ids in bundle order. Idempotent: re-installing
    /// overwrites any existing file of the same id (and rewrites the index). An
    /// `attribution.md` recording the CC-BY-SA 3.0 provenance is written
    /// alongside them; it is not itself a template (it lacks the `.opt.json`
    /// suffix, so `list_templates` ignores it).
    pub fn install_starter_templates(&self) -> Result<Vec<String>> {
        let mut installed = Vec::with_capacity(STARTER_TEMPLATES.len());
        for template in STARTER_TEMPLATES {
            let opt = Opt::from_json(template.json)?;
            installed.push(self.add_template(&opt)?);
        }
        let attribution_path = self.templates_dir().join("attribution.md");
        fs::write(&attribution_path, ATTRIBUTION).map_err(|source| StoreError::Io {
            path: attribution_path,
            source,
        })?;
        Ok(installed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DeploymentConfig;

    #[test]
    fn bundled_templates_are_valid_opt_json() {
        for template in STARTER_TEMPLATES {
            let opt = Opt::from_json(template.json).unwrap_or_else(|e| {
                panic!("starter template {} is not valid OPT: {e}", template.id)
            });
            assert_eq!(
                opt.template_id, template.id,
                "template_id inside {} disagrees with its bundle id",
                template.id
            );
        }
    }

    #[test]
    fn install_registers_every_starter_template() {
        let dir = tempfile::tempdir().expect("tempdir");
        let deployment =
            Deployment::init(dir.path(), DeploymentConfig::new("test.local")).expect("init");

        let installed = deployment.install_starter_templates().expect("install");
        assert_eq!(installed, starter_template_ids());

        let registered = deployment.list_templates().expect("list");
        for id in starter_template_ids() {
            assert!(registered.contains(&id.to_string()), "{id} not registered");
        }

        // The CC-BY-SA attribution lands with the data but is not a template.
        assert!(deployment.templates_dir().join("attribution.md").exists());
        assert!(!registered.iter().any(|id| id.contains("attribution")));
    }
}

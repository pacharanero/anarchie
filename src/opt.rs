// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! # anarchie-opt
//!
//! Loads a flattened **Operational Template** into the [`crate::aom`]
//! constraint model that the validator walks a Composition against.
//!
//! An [`Opt`] is `anarchie`'s native, already-flattened template form: a small
//! amount of identifying metadata plus a [`CComplexObject`](crate::aom::CComplexObject)
//! definition rooted at the template's top RM type (usually `COMPOSITION`).
//! Templates are registered as *data* (`anarchie template add`), parsed once
//! into this model, and walked at validation time - the file-first philosophy
//! applied to the schema as well as the data.
//!
//! Ingesting the standard OPT XML (and the web-template JSON) and lowering it
//! into this model is future work; see `specs/validation.md`.

use crate::aom::CComplexObject;
use serde::{Deserialize, Serialize};

/// Errors arising from loading an Operational Template.
#[derive(Debug, thiserror::Error)]
pub enum OptError {
    /// The bytes were not valid `anarchie` OPT JSON.
    #[error("template JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// The template's root constraint was not a `COMPOSITION`.
    #[error("template root must constrain a COMPOSITION, found {0}")]
    NotAComposition(String),
}

/// A flattened Operational Template in `anarchie`'s native form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Opt {
    /// The template identifier, e.g. `"blood_pressure"`. Matches the
    /// `archetype_details.template_id` a Composition claims.
    pub template_id: String,
    /// The root archetype concept id, e.g.
    /// `"openEHR-EHR-COMPOSITION.encounter.v1"`.
    pub concept: String,
    /// The constraint tree, rooted at the template's top RM object.
    pub definition: CComplexObject,
}

impl Opt {
    /// Parse an Operational Template from `anarchie` OPT JSON.
    pub fn from_json(json: &str) -> Result<Self, OptError> {
        let opt: Opt = serde_json::from_str(json)?;
        if opt.definition.rm_type != "COMPOSITION" {
            return Err(OptError::NotAComposition(opt.definition.rm_type.clone()));
        }
        Ok(opt)
    }

    /// Serialise the template to `anarchie` OPT JSON (pretty, newline-terminated).
    pub fn to_json(&self) -> Result<String, OptError> {
        let mut out = serde_json::to_string_pretty(self)?;
        out.push('\n');
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_minimal_template() {
        let json = r#"{
            "template_id": "blood_pressure",
            "concept": "openEHR-EHR-COMPOSITION.encounter.v1",
            "definition": {
                "type": "COMPLEX",
                "rm_type": "COMPOSITION",
                "node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
                "occurrences": { "lower": 1, "upper": 1 },
                "attributes": []
            }
        }"#;
        let opt = Opt::from_json(json).expect("parses");
        assert_eq!(opt.template_id, "blood_pressure");
        assert_eq!(opt.definition.rm_type, "COMPOSITION");
        let reparsed = Opt::from_json(&opt.to_json().unwrap()).unwrap();
        assert_eq!(opt, reparsed);
    }

    #[test]
    fn rejects_non_composition_root() {
        let json = r#"{
            "template_id": "x",
            "concept": "openEHR-EHR-OBSERVATION.blood_pressure.v2",
            "definition": {
                "type": "COMPLEX",
                "rm_type": "OBSERVATION",
                "node_id": "openEHR-EHR-OBSERVATION.blood_pressure.v2"
            }
        }"#;
        assert!(matches!(
            Opt::from_json(json),
            Err(OptError::NotAComposition(_))
        ));
    }
}

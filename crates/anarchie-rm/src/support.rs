// SPDX-License-Identifier: AGPL-3.0-or-later
//! Identifiers and references (`support.identification` in the RM).

use serde::{Deserialize, Serialize};

use crate::ty::{tags, Ty};

/// `OBJECT_ID` hierarchy: an abstract identifier with concrete subtypes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "_type")]
pub enum ObjectId {
    #[serde(rename = "HIER_OBJECT_ID")]
    HierObjectId { value: String },
    #[serde(rename = "OBJECT_VERSION_ID")]
    ObjectVersionId { value: String },
    #[serde(rename = "GENERIC_ID")]
    GenericId { value: String, scheme: String },
}

/// `UID_BASED_ID`: the subset of [`ObjectId`] usable as an object UID.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "_type")]
pub enum UidBasedId {
    #[serde(rename = "HIER_OBJECT_ID")]
    HierObjectId { value: String },
    #[serde(rename = "OBJECT_VERSION_ID")]
    ObjectVersionId { value: String },
}

/// `TERMINOLOGY_ID`: identifies a terminology such as `SNOMED-CT`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminologyId {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::TerminologyId>,
    pub value: String,
}

impl TerminologyId {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            ty: Ty::default(),
            value: value.into(),
        }
    }
}

/// `ARCHETYPE_ID`: identifies an archetype, e.g. `openEHR-EHR-OBSERVATION.x.v1`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchetypeId {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::ArchetypeId>,
    pub value: String,
}

impl ArchetypeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            ty: Ty::default(),
            value: value.into(),
        }
    }
}

/// `TEMPLATE_ID`: identifies an operational template.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateId {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::TemplateId>,
    pub value: String,
}

impl TemplateId {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            ty: Ty::default(),
            value: value.into(),
        }
    }
}

/// `CODE_PHRASE`: a coded term drawn from a terminology.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodePhrase {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::CodePhrase>,
    pub terminology_id: TerminologyId,
    pub code_string: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub preferred_term: Option<String>,
}

impl CodePhrase {
    pub fn new(terminology: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            ty: Ty::default(),
            terminology_id: TerminologyId::new(terminology),
            code_string: code.into(),
            preferred_term: None,
        }
    }
}

/// `PARTY_REF`: a reference to a party in an external demographic service.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyRef {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::PartyRef>,
    pub id: ObjectId,
    pub namespace: String,
    #[serde(rename = "type")]
    pub party_type: String,
}

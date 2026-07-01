// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Common RM types: archetyping, links, parties and participations.

use serde::{Deserialize, Serialize};

use crate::rm::data_values::{DvEhrUri, DvIdentifier};
use crate::rm::support::{ArchetypeId, PartyRef, TemplateId};
use crate::rm::text::{DvCodedText, Text};
use crate::rm::ty::{tags, Ty};

/// `ARCHETYPED`: the archetyping metadata attached to a root `LOCATABLE`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Archetyped {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::Archetyped>,
    pub archetype_id: ArchetypeId,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub template_id: Option<TemplateId>,
    pub rm_version: String,
}

/// `LINK`: a typed relationship to another `LOCATABLE`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::Link>,
    pub meaning: Text,
    #[serde(rename = "type")]
    pub link_type: Text,
    pub target: DvEhrUri,
}

/// `PARTY_IDENTIFIED`: a named, optionally identified party.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyIdentified {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub identifiers: Vec<DvIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub external_ref: Option<PartyRef>,
}

/// `PARTY_SELF`: the subject of the record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartySelf {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub external_ref: Option<PartyRef>,
}

/// `PARTY_RELATED`: a party in a stated relationship to the subject.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyRelated {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub identifiers: Vec<DvIdentifier>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub external_ref: Option<PartyRef>,
    pub relationship: DvCodedText,
}

/// `PARTY_PROXY`: a reference to a real-world party.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "_type")]
// Variant sizes differ by design: these are distinct RM classes, not a hot path.
#[allow(clippy::large_enum_variant)]
pub enum PartyProxy {
    #[serde(rename = "PARTY_SELF")]
    PartySelf(PartySelf),
    #[serde(rename = "PARTY_IDENTIFIED")]
    PartyIdentified(PartyIdentified),
    #[serde(rename = "PARTY_RELATED")]
    PartyRelated(PartyRelated),
}

impl PartyProxy {
    /// Convenience constructor for a simply-named `PARTY_IDENTIFIED`.
    pub fn named(name: impl Into<String>) -> Self {
        PartyProxy::PartyIdentified(PartyIdentified {
            name: Some(name.into()),
            identifiers: Vec::new(),
            external_ref: None,
        })
    }
}

/// `PARTICIPATION`: a party participating in an event in some function.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participation {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::Participation>,
    pub function: Text,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mode: Option<DvCodedText>,
    pub performer: PartyProxy,
}

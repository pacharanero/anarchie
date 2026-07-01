// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! `COMPOSITION` and the content hierarchy (`SECTION`, the `ENTRY` subtypes).

use serde::{Deserialize, Serialize};

use crate::rm::common::{Archetyped, Participation, PartyIdentified, PartyProxy};
use crate::rm::data_structures::ItemStructure;
use crate::rm::data_values::{DvDateTime, DvParsable};
use crate::rm::history::History;
use crate::rm::support::{CodePhrase, ObjectId, UidBasedId};
use crate::rm::text::{DvCodedText, DvText, Text};
use crate::rm::ty::{tags, Ty};

/// `LOCATABLE_REF`: a reference to a `LOCATABLE` within some EHR.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocatableRef {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::LocatableRef>,
    pub id: ObjectId,
    pub namespace: String,
    #[serde(rename = "type")]
    pub ref_type: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
}

/// `EVENT_CONTEXT`: the clinical context in which a composition was authored.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventContext {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::EventContext>,
    pub start_time: DvDateTime,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub end_time: Option<DvDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub location: Option<String>,
    pub setting: DvCodedText,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub other_context: Option<ItemStructure>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub health_care_facility: Option<PartyIdentified>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub participations: Vec<Participation>,
}

/// `ACTIVITY`: a planned activity within an `INSTRUCTION`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Activity {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::Activity>,
    pub name: Text,
    pub archetype_node_id: String,
    pub action_archetype_id: String,
    pub description: ItemStructure,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timing: Option<DvParsable>,
}

/// `ISM_TRANSITION`: the care-flow state transition recorded by an `ACTION`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsmTransition {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::IsmTransition>,
    pub current_state: DvCodedText,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub transition: Option<DvCodedText>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub careflow_step: Option<DvCodedText>,
}

/// `INSTRUCTION_DETAILS`: links an `ACTION` back to its `INSTRUCTION` activity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InstructionDetails {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::InstructionDetails>,
    pub instruction_id: LocatableRef,
    pub activity_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub wf_details: Option<ItemStructure>,
}

/// `SECTION`: a navigational grouping of content items.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub name: Text,
    pub archetype_node_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub items: Vec<ContentItem>,
}

/// `OBSERVATION`: a recorded observation (measurements, findings).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub name: Text,
    pub archetype_node_id: String,
    pub language: CodePhrase,
    pub encoding: CodePhrase,
    pub subject: PartyProxy,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<PartyProxy>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub other_participations: Vec<Participation>,
    pub data: History,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<History>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub protocol: Option<ItemStructure>,
}

/// `EVALUATION`: an opinion or assessment (diagnoses, risks).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Evaluation {
    pub name: Text,
    pub archetype_node_id: String,
    pub language: CodePhrase,
    pub encoding: CodePhrase,
    pub subject: PartyProxy,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<PartyProxy>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub other_participations: Vec<Participation>,
    pub data: ItemStructure,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub protocol: Option<ItemStructure>,
}

/// `INSTRUCTION`: an order or plan to be carried out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Instruction {
    pub name: Text,
    pub archetype_node_id: String,
    pub language: CodePhrase,
    pub encoding: CodePhrase,
    pub subject: PartyProxy,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<PartyProxy>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub other_participations: Vec<Participation>,
    pub narrative: DvText,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub expiry_time: Option<DvDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub wf_definition: Option<DvParsable>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub activities: Vec<Activity>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub protocol: Option<ItemStructure>,
}

/// `ACTION`: a record of something that was done.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub name: Text,
    pub archetype_node_id: String,
    pub language: CodePhrase,
    pub encoding: CodePhrase,
    pub subject: PartyProxy,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<PartyProxy>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub other_participations: Vec<Participation>,
    pub time: DvDateTime,
    pub description: ItemStructure,
    pub ism_transition: IsmTransition,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub instruction_details: Option<InstructionDetails>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub protocol: Option<ItemStructure>,
}

/// `ADMIN_ENTRY`: administrative information (admissions, transfers).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdminEntry {
    pub name: Text,
    pub archetype_node_id: String,
    pub language: CodePhrase,
    pub encoding: CodePhrase,
    pub subject: PartyProxy,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub provider: Option<PartyProxy>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub other_participations: Vec<Participation>,
    pub data: ItemStructure,
}

/// `CONTENT_ITEM`: the polymorphic content slot of a `COMPOSITION` or `SECTION`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
// Variant sizes differ by design: these are distinct RM classes, not a hot path.
#[allow(clippy::large_enum_variant)]
pub enum ContentItem {
    #[serde(rename = "SECTION")]
    Section(Section),
    #[serde(rename = "OBSERVATION")]
    Observation(Observation),
    #[serde(rename = "EVALUATION")]
    Evaluation(Evaluation),
    #[serde(rename = "INSTRUCTION")]
    Instruction(Instruction),
    #[serde(rename = "ACTION")]
    Action(Action),
    #[serde(rename = "ADMIN_ENTRY")]
    AdminEntry(AdminEntry),
}

/// `COMPOSITION`: the top-level committed unit of clinical content.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Composition {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::Composition>,
    pub name: Text,
    pub archetype_node_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub uid: Option<UidBasedId>,
    pub archetype_details: Archetyped,
    pub language: CodePhrase,
    pub territory: CodePhrase,
    pub category: DvCodedText,
    pub composer: PartyProxy,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub context: Option<EventContext>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub content: Vec<ContentItem>,
}

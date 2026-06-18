// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `EHR` container and its mutable `EHR_STATUS`.

use serde::{Deserialize, Serialize};

use crate::common::Archetyped;
use crate::data_structures::ItemStructure;
use crate::data_values::DvDateTime;
use crate::support::{HierObjectId, ObjectId};
use crate::text::Text;
use crate::ty::{tags, Ty};

/// `EHR`: the top-level record container for a single subject of care.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ehr {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::Ehr>,
    pub ehr_id: HierObjectId,
    pub system_id: HierObjectId,
    pub time_created: DvDateTime,
}

impl Ehr {
    pub fn new(
        ehr_id: impl Into<String>,
        system_id: impl Into<String>,
        time_created: impl Into<String>,
    ) -> Self {
        Self {
            ty: Ty::default(),
            ehr_id: HierObjectId::new(ehr_id),
            system_id: HierObjectId::new(system_id),
            time_created: DvDateTime {
                value: time_created.into(),
            },
        }
    }
}

/// `EHR_STATUS`: the mutable status object controlling queryability and
/// linking the EHR to its subject.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EhrStatus {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::EhrStatus>,
    pub name: Text,
    pub archetype_node_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub uid: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub archetype_details: Option<Archetyped>,
    pub is_queryable: bool,
    pub is_modifiable: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub other_details: Option<ItemStructure>,
}

impl EhrStatus {
    /// A minimal, queryable, modifiable default status.
    pub fn default_for(archetype_node_id: impl Into<String>) -> Self {
        Self {
            ty: Ty::default(),
            name: Text::plain("EHR Status"),
            archetype_node_id: archetype_node_id.into(),
            uid: None,
            archetype_details: None,
            is_queryable: true,
            is_modifiable: true,
            other_details: None,
        }
    }
}

// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! `HISTORY` and `EVENT` (the time-series structure used by `OBSERVATION`).

use serde::{Deserialize, Serialize};

use crate::rm::data_structures::ItemStructure;
use crate::rm::data_values::{DvDateTime, DvDuration};
use crate::rm::text::{DvCodedText, Text};
use crate::rm::ty::{tags, Ty};

/// `POINT_EVENT`: an instantaneous observation event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointEvent {
    pub name: Text,
    pub archetype_node_id: String,
    pub time: DvDateTime,
    pub data: ItemStructure,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<ItemStructure>,
}

/// `INTERVAL_EVENT`: an observation summarising a time interval.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IntervalEvent {
    pub name: Text,
    pub archetype_node_id: String,
    pub time: DvDateTime,
    pub data: ItemStructure,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub state: Option<ItemStructure>,
    pub width: DvDuration,
    pub math_function: DvCodedText,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sample_count: Option<i64>,
}

/// `EVENT`: either a [`PointEvent`] or an [`IntervalEvent`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
// Variant sizes differ by design: these are distinct RM classes, not a hot path.
#[allow(clippy::large_enum_variant)]
pub enum Event {
    #[serde(rename = "POINT_EVENT")]
    PointEvent(PointEvent),
    #[serde(rename = "INTERVAL_EVENT")]
    IntervalEvent(IntervalEvent),
}

/// `HISTORY`: an origin-anchored series of [`Event`]s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct History {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::History>,
    pub name: Text,
    pub archetype_node_id: String,
    pub origin: DvDateTime,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub period: Option<DvDuration>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub duration: Option<DvDuration>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub summary: Option<ItemStructure>,
    pub events: Vec<Event>,
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! `ITEM_STRUCTURE` and its building blocks (`CLUSTER`, `ELEMENT`).

use serde::{Deserialize, Serialize};

use crate::data_values::DataValue;
use crate::text::{DvCodedText, Text};

/// `ELEMENT`: a leaf node carrying a single [`DataValue`] (or a null flavour).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub name: Text,
    pub archetype_node_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub value: Option<DataValue>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub null_flavour: Option<DvCodedText>,
}

/// `CLUSTER`: a named grouping of [`Item`]s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cluster {
    pub name: Text,
    pub archetype_node_id: String,
    pub items: Vec<Item>,
}

/// `ITEM`: either a [`Cluster`] or an [`Element`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
// Variant sizes differ by design: these are distinct RM classes, not a hot path.
#[allow(clippy::large_enum_variant)]
pub enum Item {
    #[serde(rename = "CLUSTER")]
    Cluster(Cluster),
    #[serde(rename = "ELEMENT")]
    Element(Element),
}

/// `ITEM_TREE`: a tree of [`Item`]s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemTree {
    pub name: Text,
    pub archetype_node_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub items: Vec<Item>,
}

/// `ITEM_LIST`: a flat list of [`Element`]s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemList {
    pub name: Text,
    pub archetype_node_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub items: Vec<Element>,
}

/// `ITEM_SINGLE`: a single [`Element`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemSingle {
    pub name: Text,
    pub archetype_node_id: String,
    pub item: Element,
}

/// `ITEM_TABLE`: rows of [`Cluster`]s.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemTable {
    pub name: Text,
    pub archetype_node_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub rows: Vec<Cluster>,
}

/// `ITEM_STRUCTURE`: the polymorphic structured-data slot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
// Variant sizes differ by design: these are distinct RM classes, not a hot path.
#[allow(clippy::large_enum_variant)]
pub enum ItemStructure {
    #[serde(rename = "ITEM_TREE")]
    ItemTree(ItemTree),
    #[serde(rename = "ITEM_LIST")]
    ItemList(ItemList),
    #[serde(rename = "ITEM_SINGLE")]
    ItemSingle(ItemSingle),
    #[serde(rename = "ITEM_TABLE")]
    ItemTable(ItemTable),
}

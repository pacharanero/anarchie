// SPDX-License-Identifier: AGPL-3.0-or-later
//! Textual data values (`DV_TEXT`, `DV_CODED_TEXT`) and the `name` slot.

use serde::{Deserialize, Serialize};

use crate::rm::support::CodePhrase;
use crate::rm::ty::{tags, Ty};

/// `TERM_MAPPING`: a mapping from a term to a target terminology code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermMapping {
    #[serde(rename = "_type", default)]
    pub ty: Ty<tags::TermMapping>,
    #[serde(rename = "match")]
    pub match_op: String,
    pub target: CodePhrase,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub purpose: Option<Box<DvCodedText>>,
}

/// `DV_TEXT`: free text, optionally with mappings to coded terms.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvText {
    pub value: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mappings: Vec<TermMapping>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub formatting: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub language: Option<CodePhrase>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub encoding: Option<CodePhrase>,
}

impl DvText {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            mappings: Vec::new(),
            formatting: None,
            language: None,
            encoding: None,
        }
    }
}

/// `DV_CODED_TEXT`: text backed by a code from a terminology.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvCodedText {
    pub value: String,
    pub defining_code: CodePhrase,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mappings: Vec<TermMapping>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub formatting: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub language: Option<CodePhrase>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub encoding: Option<CodePhrase>,
}

impl DvCodedText {
    pub fn new(value: impl Into<String>, defining_code: CodePhrase) -> Self {
        Self {
            value: value.into(),
            defining_code,
            mappings: Vec::new(),
            formatting: None,
            language: None,
            encoding: None,
        }
    }
}

/// The `name` slot of a `LOCATABLE`: either `DV_TEXT` or `DV_CODED_TEXT`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "_type")]
pub enum Text {
    #[serde(rename = "DV_TEXT")]
    DvText(DvText),
    #[serde(rename = "DV_CODED_TEXT")]
    DvCodedText(DvCodedText),
}

impl Text {
    /// Convenience constructor for a plain-text name.
    pub fn plain(value: impl Into<String>) -> Self {
        Text::DvText(DvText::new(value))
    }

    /// The display value, regardless of which variant this is.
    pub fn value(&self) -> &str {
        match self {
            Text::DvText(t) => &t.value,
            Text::DvCodedText(t) => &t.value,
        }
    }
}

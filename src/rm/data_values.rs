// SPDX-License-Identifier: AGPL-3.0-or-later
//! Quantitative and basic data values, and the polymorphic [`DataValue`] slot.

use serde::{Deserialize, Serialize};

use crate::rm::support::CodePhrase;
use crate::rm::text::{DvCodedText, DvText};

/// `DV_BOOLEAN`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DvBoolean {
    pub value: bool,
}

/// `DV_IDENTIFIER`: an external identifier such as an NHS number.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvIdentifier {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub issuer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub assigner: Option<String>,
    pub id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none", default)]
    pub id_type: Option<String>,
}

/// `DV_URI`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvUri {
    pub value: String,
}

/// `DV_EHR_URI`: a URI into an EHR (the `ehr://` scheme).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvEhrUri {
    pub value: String,
}

/// `DV_QUANTITY`: a measured amount with units.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DvQuantity {
    pub magnitude: f64,
    pub units: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub precision: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub magnitude_status: Option<String>,
}

/// `DV_COUNT`: a dimensionless integer count.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvCount {
    pub magnitude: i64,
}

/// `DV_PROPORTION`: a ratio of two numbers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DvProportion {
    pub numerator: f64,
    pub denominator: f64,
    /// `proportion_kind`: 0=ratio, 1=unitary, 2=percent, 3=fraction, 4=integer fraction.
    #[serde(rename = "type")]
    pub proportion_kind: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub precision: Option<i64>,
}

/// `DV_ORDINAL`: a ranked symbolic value, e.g. a pain score.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvOrdinal {
    pub value: i64,
    pub symbol: DvCodedText,
}

/// `DV_SCALE`: like an ordinal but with a real-valued score.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DvScale {
    pub value: f64,
    pub symbol: DvCodedText,
}

/// `DV_DURATION`: an ISO 8601 duration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvDuration {
    pub value: String,
}

/// `DV_DATE_TIME`: an ISO 8601 date-time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvDateTime {
    pub value: String,
}

/// `DV_DATE`: an ISO 8601 date.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvDate {
    pub value: String,
}

/// `DV_TIME`: an ISO 8601 time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvTime {
    pub value: String,
}

/// `DV_MULTIMEDIA`: inline or referenced media content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvMultimedia {
    pub media_type: CodePhrase,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alternate_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub uri: Option<DvUri>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub size: Option<i64>,
}

/// `DV_PARSABLE`: a value in a named formalism (e.g. an embedded grammar).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvParsable {
    pub value: String,
    pub formalism: String,
}

/// `DATA_VALUE`: the polymorphic value carried by an `ELEMENT`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
pub enum DataValue {
    #[serde(rename = "DV_BOOLEAN")]
    DvBoolean(DvBoolean),
    #[serde(rename = "DV_TEXT")]
    DvText(DvText),
    #[serde(rename = "DV_CODED_TEXT")]
    DvCodedText(DvCodedText),
    #[serde(rename = "DV_IDENTIFIER")]
    DvIdentifier(DvIdentifier),
    #[serde(rename = "DV_URI")]
    DvUri(DvUri),
    #[serde(rename = "DV_EHR_URI")]
    DvEhrUri(DvEhrUri),
    #[serde(rename = "DV_QUANTITY")]
    DvQuantity(DvQuantity),
    #[serde(rename = "DV_COUNT")]
    DvCount(DvCount),
    #[serde(rename = "DV_PROPORTION")]
    DvProportion(DvProportion),
    #[serde(rename = "DV_ORDINAL")]
    DvOrdinal(DvOrdinal),
    #[serde(rename = "DV_SCALE")]
    DvScale(DvScale),
    #[serde(rename = "DV_DURATION")]
    DvDuration(DvDuration),
    #[serde(rename = "DV_DATE_TIME")]
    DvDateTime(DvDateTime),
    #[serde(rename = "DV_DATE")]
    DvDate(DvDate),
    #[serde(rename = "DV_TIME")]
    DvTime(DvTime),
    #[serde(rename = "DV_MULTIMEDIA")]
    DvMultimedia(DvMultimedia),
    #[serde(rename = "DV_PARSABLE")]
    DvParsable(DvParsable),
}

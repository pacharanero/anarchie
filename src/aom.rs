// SPDX-License-Identifier: AGPL-3.0-or-later
//! # anarchie-aom
//!
//! A pragmatic subset of the openEHR **Archetype Object Model** (AOM): the
//! constraint types that an Operational Template flattens into, expressed as
//! `serde`-deserialisable Rust types.
//!
//! `anarchie` validates Compositions against *flattened Operational Templates*,
//! never against raw ADL archetypes. Flattening (resolving specialisation and
//! filling slots) is the job of upstream tools (Archetype Designer, ADL
//! Workbench, Archie). By starting from a flattened template the constraint
//! model shrinks dramatically: there is no specialisation to resolve, no slots
//! to fill, just a tree of constraints to walk a Composition against.
//!
//! The JSON form here is `anarchie`'s own canonical constraint serialisation.
//! Ingesting the standard OPT XML / web-template JSON and lowering it into this
//! model is future work (see `specs/validation.md`); this model is the stable
//! target that the validator consumes.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

/// A multiplicity interval `{lower..upper}` used for occurrences, existence and
/// cardinality. An absent `upper` means unbounded (the ADL `*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiplicityInterval {
    #[serde(default)]
    pub lower: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper: Option<u32>,
}

impl MultiplicityInterval {
    /// `{0..*}` - optional and unbounded.
    pub const ANY: Self = Self {
        lower: 0,
        upper: None,
    };

    /// `{1..1}` - mandatory and single.
    pub const MANDATORY: Self = Self {
        lower: 1,
        upper: Some(1),
    };

    /// Whether the node this constrains must be present at least once.
    pub fn is_mandatory(&self) -> bool {
        self.lower >= 1
    }

    /// Whether `count` satisfies the interval.
    pub fn contains(&self, count: u32) -> bool {
        count >= self.lower && self.upper.map(|u| count <= u).unwrap_or(true)
    }

    /// A human-readable form such as `1..*` or `0..1`.
    pub fn display(&self) -> String {
        match self.upper {
            Some(u) => format!("{}..{}", self.lower, u),
            None => format!("{}..*", self.lower),
        }
    }
}

impl Default for MultiplicityInterval {
    fn default() -> Self {
        Self::ANY
    }
}

/// A cardinality constraint on a container attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cardinality {
    pub interval: MultiplicityInterval,
    #[serde(default)]
    pub is_ordered: bool,
    #[serde(default)]
    pub is_unique: bool,
}

/// A closed/open numeric interval, used for `C_DV_QUANTITY` magnitude ranges and
/// the like. An absent bound is open (unbounded) on that side.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Interval<T> {
    #[serde(default = "none", skip_serializing_if = "Option::is_none")]
    pub lower: Option<T>,
    #[serde(default = "none", skip_serializing_if = "Option::is_none")]
    pub upper: Option<T>,
    #[serde(default = "yes")]
    pub lower_included: bool,
    #[serde(default = "yes")]
    pub upper_included: bool,
}

fn none<T>() -> Option<T> {
    None
}

fn yes() -> bool {
    true
}

impl<T: PartialOrd + Copy> Interval<T> {
    /// Whether `value` falls within the interval, honouring inclusivity.
    pub fn contains(&self, value: T) -> bool {
        if let Some(lower) = self.lower {
            match value.partial_cmp(&lower) {
                Some(Ordering::Less) | None => return false,
                Some(Ordering::Equal) if !self.lower_included => return false,
                _ => {}
            }
        }
        if let Some(upper) = self.upper {
            match value.partial_cmp(&upper) {
                Some(Ordering::Greater) | None => return false,
                Some(Ordering::Equal) if !self.upper_included => return false,
                _ => {}
            }
        }
        true
    }
}

/// The polymorphic constraint on an RM object (`C_OBJECT`).
///
/// Complex objects (`COMPOSITION`, `OBSERVATION`, `ELEMENT`, `CLUSTER`, ...) are
/// matched to RM nodes by their `archetype_node_id`. Leaf constraints
/// (`C_DV_QUANTITY` and friends) apply to the data value carried by an
/// attribute (typically `ELEMENT.value`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CObject {
    /// A constraint on a complex RM object with constrained attributes.
    #[serde(rename = "COMPLEX")]
    Complex(CComplexObject),
    /// A constraint on a `DV_QUANTITY` (units and magnitude range).
    #[serde(rename = "DV_QUANTITY")]
    DvQuantity(CDvQuantity),
    /// A constraint on a `CODE_PHRASE` (the allowed coded values).
    #[serde(rename = "CODE_PHRASE")]
    CodePhrase(CCodePhrase),
    /// A constraint on a `DV_TEXT` value (pattern or enumerated list).
    #[serde(rename = "C_STRING")]
    String(CString),
    /// A constraint on a `DV_ORDINAL` (the allowed ranked values).
    #[serde(rename = "DV_ORDINAL")]
    DvOrdinal(CDvOrdinal),
}

impl CObject {
    /// The `archetype_node_id` this object matches, if it is a complex node.
    pub fn node_id(&self) -> Option<&str> {
        match self {
            CObject::Complex(c) => Some(&c.node_id),
            _ => None,
        }
    }

    /// Whether this is a leaf (data-value) constraint rather than a complex node.
    pub fn is_leaf(&self) -> bool {
        !matches!(self, CObject::Complex(_))
    }
}

/// A constraint on a complex RM object (`C_COMPLEX_OBJECT`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CComplexObject {
    /// The RM class name this constrains, e.g. `"OBSERVATION"`.
    pub rm_type: String,
    /// The `archetype_node_id` an RM node must carry to match this constraint
    /// (an archetype id like `openEHR-EHR-OBSERVATION.blood_pressure.v2` at a
    /// root, or an `atNNNN` code within an archetype).
    pub node_id: String,
    #[serde(default)]
    pub occurrences: MultiplicityInterval,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<CAttribute>,
}

/// A constraint on an RM attribute (`C_ATTRIBUTE`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CAttribute {
    /// The RM attribute name, e.g. `"data"`, `"items"`, `"value"`.
    pub rm_attribute: String,
    #[serde(default)]
    pub existence: MultiplicityInterval,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cardinality: Option<Cardinality>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<CObject>,
}

/// `C_DV_QUANTITY`: a disjunction of permitted (units, magnitude-range) items.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CDvQuantity {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub list: Vec<CQuantityItem>,
}

/// One permitted units value within a [`CDvQuantity`], with an optional
/// magnitude range and precision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CQuantityItem {
    pub units: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub magnitude: Option<Interval<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub precision: Option<Interval<i64>>,
}

/// `C_CODE_PHRASE`: the permitted coded values for a `CODE_PHRASE`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CCodePhrase {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminology: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub codes: Vec<String>,
}

/// `C_STRING`: a pattern or enumerated list constraint on a string value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CString {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub list: Vec<String>,
}

/// `C_DV_ORDINAL`: the permitted ranked values for a `DV_ORDINAL`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CDvOrdinal {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiplicity_contains_and_mandatory() {
        let m = MultiplicityInterval {
            lower: 1,
            upper: Some(2),
        };
        assert!(m.is_mandatory());
        assert!(!m.contains(0));
        assert!(m.contains(1));
        assert!(m.contains(2));
        assert!(!m.contains(3));
        assert_eq!(m.display(), "1..2");
        assert_eq!(MultiplicityInterval::ANY.display(), "0..*");
    }

    #[test]
    fn interval_inclusivity() {
        let closed = Interval {
            lower: Some(0.0),
            upper: Some(300.0),
            lower_included: true,
            upper_included: true,
        };
        assert!(closed.contains(0.0));
        assert!(closed.contains(300.0));
        assert!(!closed.contains(-0.1));
        assert!(!closed.contains(300.1));

        let open_upper = Interval {
            lower: Some(0.0),
            upper: Some(1.0),
            lower_included: true,
            upper_included: false,
        };
        assert!(!open_upper.contains(1.0));
        assert!(open_upper.contains(0.999));
    }
}

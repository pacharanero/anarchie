// SPDX-License-Identifier: AGPL-3.0-or-later
//! Operational Template validation.
//!
//! This layer answers "is this *this kind of* openEHR?" - does the Composition
//! match the specific template it claims? It walks the canonical JSON of the
//! Composition guided by the flattened constraint tree of an [`Opt`], checking
//! occurrences, attribute existence and cardinality, and the leaf data-value
//! constraints (`C_DV_QUANTITY` units and magnitude ranges, `C_CODE_PHRASE`
//! value sets, `C_STRING` enumerations, `C_DV_ORDINAL` values).
//!
//! Walking the JSON rather than the typed RM tree keeps this engine small and
//! uniform: the AOM names RM attributes as strings, which map directly onto
//! canonical JSON object keys, and `archetype_node_id` matching is a field
//! lookup.

use crate::aom::{CAttribute, CComplexObject, CObject};
use crate::opt::Opt;
use serde_json::Value;

use crate::validate::report::ValidationReport;

/// Validate a Composition (as canonical JSON) against an Operational Template.
pub fn validate_opt(composition: &Value, opt: &Opt) -> ValidationReport {
    let mut report = ValidationReport::new();

    let claimed = composition
        .get("archetype_details")
        .and_then(|a| a.get("template_id"))
        .and_then(|t| t.get("value"))
        .and_then(Value::as_str);
    if let Some(claimed) = claimed {
        if claimed != opt.template_id {
            report.warning(
                "/",
                "OPT:template_id",
                format!(
                    "composition claims template \"{claimed}\" but is being validated against \"{}\"",
                    opt.template_id
                ),
            );
        }
    }

    let root_node = composition
        .get("archetype_node_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    if root_node != opt.definition.node_id {
        report.warning(
            "/",
            "OPT:root_archetype",
            format!(
                "composition root archetype_node_id \"{root_node}\" does not match template root \"{}\"",
                opt.definition.node_id
            ),
        );
    }

    walk_complex(composition, &opt.definition, "", &mut report);
    report
}

fn walk_complex(node: &Value, c: &CComplexObject, path: &str, report: &mut ValidationReport) {
    for attr in &c.attributes {
        walk_attribute(node, attr, path, report);
    }
}

fn walk_attribute(node: &Value, attr: &CAttribute, path: &str, report: &mut ValidationReport) {
    let attr_path = format!("{path}/{}", attr.rm_attribute);
    let child_value = node.get(&attr.rm_attribute);
    let rm_children = as_children(child_value);

    if attr.existence.is_mandatory() && rm_children.is_empty() {
        report.error(
            &attr_path,
            "OPT:existence",
            format!(
                "mandatory attribute \"{}\" (existence {}) is absent",
                attr.rm_attribute,
                attr.existence.display()
            ),
        );
        return;
    }

    if let Some(cardinality) = &attr.cardinality {
        let count = rm_children.len() as u32;
        if !cardinality.interval.contains(count) {
            report.error(
                &attr_path,
                "OPT:cardinality",
                format!(
                    "attribute \"{}\" has {count} items, outside cardinality {}",
                    attr.rm_attribute,
                    cardinality.interval.display()
                ),
            );
        }
    }

    for constraint in &attr.children {
        match constraint {
            CObject::Complex(cc) => {
                let matched: Vec<&Value> = rm_children
                    .iter()
                    .copied()
                    .filter(|child| node_id_of(child) == Some(cc.node_id.as_str()))
                    .collect();
                let count = matched.len() as u32;
                if !cc.occurrences.contains(count) {
                    report.error(
                        format!("{attr_path}[{}]", cc.node_id),
                        "OPT:occurrences",
                        format!(
                            "node \"{}\" occurs {count} times, outside occurrences {}",
                            cc.node_id,
                            cc.occurrences.display()
                        ),
                    );
                }
                for child in matched {
                    let child_path = format!("{attr_path}[{}]", cc.node_id);
                    walk_complex(child, cc, &child_path, report);
                }
            }
            leaf => {
                for child in &rm_children {
                    apply_leaf(child, leaf, &attr_path, report);
                }
            }
        }
    }
}

fn apply_leaf(value: &Value, constraint: &CObject, path: &str, report: &mut ValidationReport) {
    match constraint {
        CObject::DvQuantity(cq) => {
            if value.get("_type").and_then(Value::as_str) != Some("DV_QUANTITY") {
                return;
            }
            let units = value.get("units").and_then(Value::as_str).unwrap_or("");
            let magnitude = value.get("magnitude").and_then(Value::as_f64);
            if cq.list.is_empty() {
                return;
            }
            match cq.list.iter().find(|item| item.units == units) {
                None => {
                    let allowed: Vec<&str> = cq.list.iter().map(|i| i.units.as_str()).collect();
                    report.error(
                        path,
                        "C_DV_QUANTITY",
                        format!(
                            "units \"{units}\" not permitted; allowed: {}",
                            allowed.join(", ")
                        ),
                    );
                }
                Some(item) => {
                    if let (Some(range), Some(mag)) = (&item.magnitude, magnitude) {
                        if !range.contains(mag) {
                            report.error(
                                format!("{path}/magnitude"),
                                "C_DV_QUANTITY",
                                format!(
                                    "magnitude {mag} outside permitted range for units \"{units}\""
                                ),
                            );
                        }
                    }
                }
            }
        }
        CObject::CodePhrase(cc) => {
            let code = coded_value(value);
            if let Some((terminology, code_string)) = code {
                if let Some(expected) = &cc.terminology {
                    if expected != terminology {
                        report.error(
                            path,
                            "C_CODE_PHRASE",
                            format!(
                                "terminology \"{terminology}\" does not match required \"{expected}\""
                            ),
                        );
                    }
                }
                if !cc.codes.is_empty() && !cc.codes.iter().any(|c| c == code_string) {
                    report.error(
                        path,
                        "C_CODE_PHRASE",
                        format!(
                            "code \"{code_string}\" not in permitted set: {}",
                            cc.codes.join(", ")
                        ),
                    );
                }
            }
        }
        CObject::String(cs) => {
            if value.get("_type").and_then(Value::as_str) != Some("DV_TEXT") {
                return;
            }
            let text = value.get("value").and_then(Value::as_str).unwrap_or("");
            if !cs.list.is_empty() && !cs.list.iter().any(|s| s == text) {
                report.error(
                    path,
                    "C_STRING",
                    format!(
                        "value \"{text}\" not in permitted set: {}",
                        cs.list.join(", ")
                    ),
                );
            }
        }
        CObject::DvOrdinal(co) => {
            if value.get("_type").and_then(Value::as_str) != Some("DV_ORDINAL") {
                return;
            }
            if let Some(v) = value.get("value").and_then(Value::as_i64) {
                if !co.values.is_empty() && !co.values.contains(&v) {
                    report.error(
                        path,
                        "C_DV_ORDINAL",
                        format!("ordinal value {v} not in permitted set"),
                    );
                }
            }
        }
        CObject::Complex(_) => {}
    }
}

/// Extract `(terminology, code_string)` from either a `DV_CODED_TEXT` (via its
/// `defining_code`) or a bare `CODE_PHRASE`.
fn coded_value(value: &Value) -> Option<(&str, &str)> {
    let code = match value.get("_type").and_then(Value::as_str) {
        Some("DV_CODED_TEXT") => value.get("defining_code")?,
        _ if value.get("code_string").is_some() => value,
        _ => return None,
    };
    let terminology = code
        .get("terminology_id")
        .and_then(|t| t.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let code_string = code
        .get("code_string")
        .and_then(Value::as_str)
        .unwrap_or("");
    Some((terminology, code_string))
}

/// The candidate RM child objects carried by an attribute value: the elements
/// of an array, a single object, or nothing.
fn as_children(value: Option<&Value>) -> Vec<&Value> {
    match value {
        Some(Value::Array(items)) => items.iter().filter(|v| v.is_object()).collect(),
        Some(v @ Value::Object(_)) => vec![v],
        _ => Vec::new(),
    }
}

fn node_id_of(value: &Value) -> Option<&str> {
    value.get("archetype_node_id").and_then(Value::as_str)
}

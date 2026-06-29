// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests for RM and OPT validation against the shared
//! blood-pressure fixture and a hand-authored Operational Template.

use anarchie::rm::{from_canonical_str, Composition};
use anarchie::validate::{validate, validate_opt, validate_rm, Opt, Severity};
use serde_json::Value;

const COMPOSITION_JSON: &str = include_str!("fixtures/blood-pressure-composition.json");
const OPT_JSON: &str = include_str!("fixtures/vital-signs-encounter.opt.json");

fn composition() -> Composition {
    from_canonical_str(COMPOSITION_JSON).expect("fixture parses")
}

fn opt() -> Opt {
    Opt::from_json(OPT_JSON).expect("OPT parses")
}

#[test]
fn the_fixture_is_rm_valid() {
    let report = validate_rm(&composition());
    assert!(
        report.valid,
        "unexpected violations: {:?}",
        report.violations
    );
    assert_eq!(report.error_count(), 0);
}

#[test]
fn the_fixture_conforms_to_its_template() {
    let report = validate(&composition(), Some(&opt()));
    assert!(
        report.valid,
        "unexpected violations: {:?}",
        report.violations
    );
}

#[test]
fn an_element_with_neither_value_nor_null_is_rejected() {
    let mut value: Value = serde_json::from_str(COMPOSITION_JSON).unwrap();
    // Strip the systolic element's value so it has neither value nor null.
    let element = &mut value["content"][0]["data"]["events"][0]["data"]["items"][0];
    element.as_object_mut().unwrap().remove("value");

    let composition: Composition = serde_json::from_value(value).unwrap();
    let report = validate_rm(&composition);
    assert!(!report.valid);
    assert!(report
        .violations
        .iter()
        .any(|v| v.constraint == "RM:element_value_xor_null"));
}

#[test]
fn an_empty_code_phrase_is_rejected() {
    let mut value: Value = serde_json::from_str(COMPOSITION_JSON).unwrap();
    value["language"]["code_string"] = Value::String(String::new());
    let composition: Composition = serde_json::from_value(value).unwrap();

    let report = validate_rm(&composition);
    assert!(!report.valid);
    assert!(report
        .violations
        .iter()
        .any(|v| v.constraint == "RM:CODE_PHRASE_code"));
}

#[test]
fn an_out_of_range_magnitude_breaches_the_template() {
    let mut value: Value = serde_json::from_str(COMPOSITION_JSON).unwrap();
    // Systolic of 5000 mmHg is well outside the permitted 0..1000.
    value["content"][0]["data"]["events"][0]["data"]["items"][0]["value"]["magnitude"] =
        serde_json::json!(5000.0);

    let report = validate_opt(&value, &opt());
    assert!(!report.valid);
    let violation = report
        .violations
        .iter()
        .find(|v| v.constraint == "C_DV_QUANTITY")
        .expect("a C_DV_QUANTITY violation");
    assert_eq!(violation.severity, Severity::Error);
    assert!(violation.rm_path.ends_with("/magnitude"));
}

#[test]
fn disallowed_units_breach_the_template() {
    let mut value: Value = serde_json::from_str(COMPOSITION_JSON).unwrap();
    value["content"][0]["data"]["events"][0]["data"]["items"][0]["value"]["units"] =
        Value::String("kPa".to_string());

    let report = validate_opt(&value, &opt());
    assert!(!report.valid);
    assert!(report
        .violations
        .iter()
        .any(|v| v.constraint == "C_DV_QUANTITY" && v.message.contains("not permitted")));
}

#[test]
fn a_missing_mandatory_observation_breaches_occurrences() {
    let mut value: Value = serde_json::from_str(COMPOSITION_JSON).unwrap();
    // Remove all content: the mandatory blood-pressure observation is now absent.
    value["content"] = Value::Array(Vec::new());

    let report = validate_opt(&value, &opt());
    assert!(!report.valid);
    assert!(report
        .violations
        .iter()
        .any(|v| v.constraint == "OPT:existence" || v.constraint == "OPT:cardinality"));
}

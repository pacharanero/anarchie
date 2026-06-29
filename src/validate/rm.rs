// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reference Model structural validation.
//!
//! This layer answers "is this openEHR at all?" - independent of any template.
//! `serde` already rejects JSON that is the wrong *shape* (missing mandatory
//! attributes, wrong types) when it deserialises into [`Composition`]. This
//! walker catches the semantic RM rules that survive deserialisation: an
//! `ELEMENT` must carry a value xor a null flavour, a `CODE_PHRASE` must name a
//! terminology and a code, a `DV_QUANTITY` must have units, and so on.
//!
//! It is always-on and needs no template, so even a deployment with no
//! registered templates rejects structurally malformed Compositions at commit.

use crate::rm::{
    Action, AdminEntry, Cluster, CodePhrase, Composition, ContentItem, DataValue, Element,
    Evaluation, Event, Instruction, Item, ItemStructure, Observation, Section,
};

use crate::validate::report::ValidationReport;

const MAGNITUDE_STATUS: [&str; 6] = ["=", "<", ">", "<=", ">=", "~"];

/// Validate a Composition against the Reference Model. Always available.
pub fn validate_rm(composition: &Composition) -> ValidationReport {
    let mut report = ValidationReport::new();
    let root = "/";

    if composition.archetype_details.rm_version.trim().is_empty() {
        report.error(
            root,
            "RM:rm_version_present",
            "archetype_details.rm_version must not be empty",
        );
    }
    if composition.archetype_node_id.trim().is_empty() {
        report.error(
            root,
            "RM:archetype_node_id_present",
            "COMPOSITION.archetype_node_id must not be empty",
        );
    } else if composition.archetype_node_id != composition.archetype_details.archetype_id.value {
        report.warning(
            root,
            "RM:root_archetype_consistency",
            format!(
                "archetype_node_id \"{}\" differs from archetype_details.archetype_id \"{}\"",
                composition.archetype_node_id, composition.archetype_details.archetype_id.value
            ),
        );
    }

    check_code_phrase(&composition.language, root, "language", &mut report);
    check_code_phrase(&composition.territory, root, "territory", &mut report);
    check_code_phrase(
        &composition.category.defining_code,
        root,
        "category/defining_code",
        &mut report,
    );

    for item in &composition.content {
        let path = format!("/content[{}]", content_node_id(item));
        validate_content_item(item, &path, &mut report);
    }

    report
}

fn validate_content_item(item: &ContentItem, path: &str, report: &mut ValidationReport) {
    match item {
        ContentItem::Section(section) => validate_section(section, path, report),
        ContentItem::Observation(obs) => validate_observation(obs, path, report),
        ContentItem::Evaluation(ev) => validate_evaluation(ev, path, report),
        ContentItem::Instruction(instr) => validate_instruction(instr, path, report),
        ContentItem::Action(action) => validate_action(action, path, report),
        ContentItem::AdminEntry(admin) => validate_admin_entry(admin, path, report),
    }
}

fn validate_section(section: &Section, path: &str, report: &mut ValidationReport) {
    require_node_id(&section.archetype_node_id, path, "SECTION", report);
    for child in &section.items {
        let child_path = format!("{path}/items[{}]", content_node_id(child));
        validate_content_item(child, &child_path, report);
    }
}

fn validate_observation(obs: &Observation, path: &str, report: &mut ValidationReport) {
    require_node_id(&obs.archetype_node_id, path, "OBSERVATION", report);
    check_code_phrase(&obs.language, path, "language", report);
    check_code_phrase(&obs.encoding, path, "encoding", report);
    for (i, event) in obs.data.events.iter().enumerate() {
        let (node_id, data) = match event {
            Event::PointEvent(e) => (&e.archetype_node_id, &e.data),
            Event::IntervalEvent(e) => (&e.archetype_node_id, &e.data),
        };
        let event_path = format!(
            "{path}/data[{}]/events[{}]",
            obs.data.archetype_node_id, node_id
        );
        let _ = i;
        validate_item_structure(data, &event_path, report);
    }
}

fn validate_evaluation(ev: &Evaluation, path: &str, report: &mut ValidationReport) {
    require_node_id(&ev.archetype_node_id, path, "EVALUATION", report);
    check_code_phrase(&ev.language, path, "language", report);
    check_code_phrase(&ev.encoding, path, "encoding", report);
    validate_item_structure(&ev.data, &format!("{path}/data"), report);
}

fn validate_instruction(instr: &Instruction, path: &str, report: &mut ValidationReport) {
    require_node_id(&instr.archetype_node_id, path, "INSTRUCTION", report);
    check_code_phrase(&instr.language, path, "language", report);
    check_code_phrase(&instr.encoding, path, "encoding", report);
    for activity in &instr.activities {
        validate_item_structure(
            &activity.description,
            &format!(
                "{path}/activities[{}]/description",
                activity.archetype_node_id
            ),
            report,
        );
    }
}

fn validate_action(action: &Action, path: &str, report: &mut ValidationReport) {
    require_node_id(&action.archetype_node_id, path, "ACTION", report);
    check_code_phrase(&action.language, path, "language", report);
    check_code_phrase(&action.encoding, path, "encoding", report);
    check_code_phrase(
        &action.ism_transition.current_state.defining_code,
        path,
        "ism_transition/current_state/defining_code",
        report,
    );
    validate_item_structure(&action.description, &format!("{path}/description"), report);
}

fn validate_admin_entry(admin: &AdminEntry, path: &str, report: &mut ValidationReport) {
    require_node_id(&admin.archetype_node_id, path, "ADMIN_ENTRY", report);
    check_code_phrase(&admin.language, path, "language", report);
    check_code_phrase(&admin.encoding, path, "encoding", report);
    validate_item_structure(&admin.data, &format!("{path}/data"), report);
}

fn validate_item_structure(structure: &ItemStructure, path: &str, report: &mut ValidationReport) {
    match structure {
        ItemStructure::ItemTree(tree) => {
            for item in &tree.items {
                validate_item(item, path, report);
            }
        }
        ItemStructure::ItemList(list) => {
            for element in &list.items {
                let element_path = format!("{path}/items[{}]", element.archetype_node_id);
                validate_element(element, &element_path, report);
            }
        }
        ItemStructure::ItemSingle(single) => {
            let element_path = format!("{path}/item[{}]", single.item.archetype_node_id);
            validate_element(&single.item, &element_path, report);
        }
        ItemStructure::ItemTable(table) => {
            for row in &table.rows {
                validate_cluster(row, path, report);
            }
        }
    }
}

fn validate_item(item: &Item, path: &str, report: &mut ValidationReport) {
    match item {
        Item::Cluster(cluster) => validate_cluster(cluster, path, report),
        Item::Element(element) => {
            let element_path = format!("{path}/items[{}]", element.archetype_node_id);
            validate_element(element, &element_path, report);
        }
    }
}

fn validate_cluster(cluster: &Cluster, path: &str, report: &mut ValidationReport) {
    require_node_id(&cluster.archetype_node_id, path, "CLUSTER", report);
    let cluster_path = format!("{path}/items[{}]", cluster.archetype_node_id);
    for item in &cluster.items {
        validate_item(item, &cluster_path, report);
    }
}

fn validate_element(element: &Element, path: &str, report: &mut ValidationReport) {
    require_node_id(&element.archetype_node_id, path, "ELEMENT", report);
    match (&element.value, &element.null_flavour) {
        (Some(_), Some(_)) => report.error(
            path,
            "RM:element_value_xor_null",
            "ELEMENT must not carry both a value and a null_flavour",
        ),
        (None, None) => report.error(
            path,
            "RM:element_value_xor_null",
            "ELEMENT must carry either a value or a null_flavour",
        ),
        (Some(value), None) => validate_data_value(value, path, report),
        (None, Some(null)) => check_code_phrase(
            &null.defining_code,
            path,
            "null_flavour/defining_code",
            report,
        ),
    }
}

fn validate_data_value(value: &DataValue, path: &str, report: &mut ValidationReport) {
    let value_path = format!("{path}/value");
    match value {
        DataValue::DvQuantity(q) => {
            if q.units.trim().is_empty() {
                report.error(
                    &value_path,
                    "RM:DV_QUANTITY_units",
                    "DV_QUANTITY.units must not be empty",
                );
            }
            if let Some(status) = &q.magnitude_status {
                if !MAGNITUDE_STATUS.contains(&status.as_str()) {
                    report.error(
                        &value_path,
                        "RM:DV_QUANTITY_magnitude_status",
                        format!(
                            "DV_QUANTITY.magnitude_status \"{status}\" is not one of =, <, >, <=, >=, ~"
                        ),
                    );
                }
            }
        }
        DataValue::DvCodedText(c) => {
            check_code_phrase(&c.defining_code, path, "value/defining_code", report);
        }
        DataValue::DvProportion(p) => {
            if !(0..=4).contains(&p.proportion_kind) {
                report.error(
                    &value_path,
                    "RM:DV_PROPORTION_kind",
                    format!("DV_PROPORTION.type {} is not in 0..=4", p.proportion_kind),
                );
            }
            if p.denominator == 0.0 {
                report.error(
                    &value_path,
                    "RM:DV_PROPORTION_denominator",
                    "DV_PROPORTION.denominator must not be zero",
                );
            }
        }
        DataValue::DvOrdinal(o) => {
            check_code_phrase(
                &o.symbol.defining_code,
                path,
                "value/symbol/defining_code",
                report,
            );
        }
        DataValue::DvScale(s) => {
            check_code_phrase(
                &s.symbol.defining_code,
                path,
                "value/symbol/defining_code",
                report,
            );
        }
        _ => {}
    }
}

fn check_code_phrase(code: &CodePhrase, path: &str, attr: &str, report: &mut ValidationReport) {
    if code.terminology_id.value.trim().is_empty() {
        report.error(
            format!("{path}/{attr}"),
            "RM:CODE_PHRASE_terminology",
            "CODE_PHRASE.terminology_id must not be empty",
        );
    }
    if code.code_string.trim().is_empty() {
        report.error(
            format!("{path}/{attr}"),
            "RM:CODE_PHRASE_code",
            "CODE_PHRASE.code_string must not be empty",
        );
    }
}

fn require_node_id(node_id: &str, path: &str, rm_class: &str, report: &mut ValidationReport) {
    if node_id.trim().is_empty() {
        report.error(
            path,
            "RM:archetype_node_id_present",
            format!("{rm_class}.archetype_node_id must not be empty"),
        );
    }
}

fn content_node_id(item: &ContentItem) -> &str {
    match item {
        ContentItem::Section(s) => &s.archetype_node_id,
        ContentItem::Observation(o) => &o.archetype_node_id,
        ContentItem::Evaluation(e) => &e.archetype_node_id,
        ContentItem::Instruction(i) => &i.archetype_node_id,
        ContentItem::Action(a) => &a.archetype_node_id,
        ContentItem::AdminEntry(a) => &a.archetype_node_id,
    }
}

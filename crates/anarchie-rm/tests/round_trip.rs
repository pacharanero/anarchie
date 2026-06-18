// SPDX-License-Identifier: AGPL-3.0-or-later
//! Canonical-JSON round-trip and idempotency tests for `anarchie-rm`.

use anarchie_rm::{
    Composition, ContentItem, DataValue, Item, ItemStructure, PartyProxy, Text,
};

const BLOOD_PRESSURE: &str =
    include_str!("fixtures/blood-pressure-composition.json");

fn parse(json: &str) -> Composition {
    anarchie_rm::from_canonical_str(json).expect("fixture should parse as COMPOSITION")
}

fn canonical(composition: &Composition) -> String {
    anarchie_rm::to_canonical_string(composition).expect("serialisation should succeed")
}

#[test]
fn parses_expected_clinical_content() {
    let composition = parse(BLOOD_PRESSURE);

    assert_eq!(composition.name.value(), "Blood pressure");
    assert_eq!(
        composition.archetype_details.archetype_id.value,
        "openEHR-EHR-COMPOSITION.encounter.v1"
    );
    assert_eq!(composition.archetype_details.rm_version, "1.1.0");
    assert_eq!(composition.territory.code_string, "GB");

    match &composition.composer {
        PartyProxy::PartyIdentified(p) => {
            assert_eq!(p.name.as_deref(), Some("Dr Ada Lovelace"))
        }
        other => panic!("unexpected composer variant: {other:?}"),
    }

    assert_eq!(composition.content.len(), 1);
    let observation = match &composition.content[0] {
        ContentItem::Observation(obs) => obs,
        other => panic!("expected an OBSERVATION, found {other:?}"),
    };

    let event = &observation.data.events[0];
    let data = match event {
        anarchie_rm::Event::PointEvent(e) => &e.data,
        anarchie_rm::Event::IntervalEvent(e) => &e.data,
    };
    let tree = match data {
        ItemStructure::ItemTree(t) => t,
        other => panic!("expected an ITEM_TREE, found {other:?}"),
    };
    assert_eq!(tree.items.len(), 2);

    let systolic = match &tree.items[0] {
        Item::Element(el) => el,
        other => panic!("expected an ELEMENT, found {other:?}"),
    };
    assert!(matches!(&systolic.name, Text::DvText(_)));
    match systolic.value.as_ref().expect("systolic has a value") {
        DataValue::DvQuantity(q) => {
            assert_eq!(q.magnitude, 128.0);
            assert_eq!(q.units, "mm[Hg]");
        }
        other => panic!("expected a DV_QUANTITY, found {other:?}"),
    }
}

#[test]
fn round_trip_preserves_structure() {
    let original = parse(BLOOD_PRESSURE);
    let serialised = canonical(&original);
    let reparsed = parse(&serialised);
    assert_eq!(
        original, reparsed,
        "parsing our own canonical output must reproduce the same value"
    );
}

#[test]
fn canonical_serialisation_is_idempotent() {
    // The defining property of the canonical form: once a document has been
    // run through it, re-serialising never changes another byte.
    let once = canonical(&parse(BLOOD_PRESSURE));
    let twice = canonical(&parse(&once));
    assert_eq!(once, twice, "canonical serialisation must be byte-stable");
}

#[test]
fn canonical_output_is_pretty_and_newline_terminated() {
    let serialised = canonical(&parse(BLOOD_PRESSURE));
    assert!(serialised.ends_with("}\n"), "canonical files end with a newline");
    assert!(
        serialised.contains("\n  \"_type\": \"COMPOSITION\""),
        "canonical files are pretty-printed with a leading _type"
    );
}

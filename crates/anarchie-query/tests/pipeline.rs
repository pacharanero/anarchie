// SPDX-License-Identifier: AGPL-3.0-or-later
//! End-to-end Phase 4 pipeline: a real deployment → git commit → index → AQL.
//! Exercises the freshness check and the deployment-level `Index::build`, which
//! the in-crate unit tests (in-memory) do not.
#![recursion_limit = "256"]

use anarchie_query::{execute, Index, Params};
use anarchie_rm::Composition;
use anarchie_store::{Audit, ChangeType, Deployment, DeploymentConfig};
use serde_json::json;

const BP: &str = "openEHR-EHR-OBSERVATION.blood_pressure.v2";

fn bp_composition(systolic: f64) -> Composition {
    let value = json!({
        "_type": "COMPOSITION",
        "name": { "_type": "DV_TEXT", "value": "Blood pressure" },
        "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-COMPOSITION.encounter.v1" },
            "rm_version": "1.1.0"
        },
        "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
        "territory": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" }, "code_string": "GB" },
        "category": { "_type": "DV_CODED_TEXT", "value": "event", "defining_code": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" }, "code_string": "433" } },
        "composer": { "_type": "PARTY_IDENTIFIED", "name": "Dr Ada Lovelace" },
        "content": [{
            "_type": "OBSERVATION",
            "name": { "_type": "DV_TEXT", "value": "Blood pressure" },
            "archetype_node_id": BP,
            "language": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" }, "code_string": "en" },
            "encoding": { "_type": "CODE_PHRASE", "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "IANA_character-sets" }, "code_string": "UTF-8" },
            "subject": { "_type": "PARTY_SELF" },
            "data": { "_type": "HISTORY", "name": { "_type": "DV_TEXT", "value": "History" }, "archetype_node_id": "at0001", "origin": { "_type": "DV_DATE_TIME", "value": "2025-06-01T09:15:00Z" }, "events": [{
                "_type": "POINT_EVENT", "name": { "_type": "DV_TEXT", "value": "Any event" }, "archetype_node_id": "at0006",
                "time": { "_type": "DV_DATE_TIME", "value": "2025-06-01T09:15:00Z" },
                "data": { "_type": "ITEM_TREE", "name": { "_type": "DV_TEXT", "value": "blood pressure" }, "archetype_node_id": "at0003", "items": [{
                    "_type": "ELEMENT", "name": { "_type": "DV_TEXT", "value": "Systolic" }, "archetype_node_id": "at0004",
                    "value": { "_type": "DV_QUANTITY", "magnitude": systolic, "units": "mm[Hg]" }
                }]}
            }]}
        }]
    });
    anarchie_rm::from_canonical_str(&value.to_string()).expect("valid composition")
}

#[test]
fn deployment_index_and_query_end_to_end() {
    let tmp = tempfile::tempdir().unwrap();
    let deployment =
        Deployment::init(tmp.path(), DeploymentConfig::new("test.local")).expect("init");
    let audit = Audit::now("tester", "t@example.org", ChangeType::Creation, "BP");

    let repo = deployment.create_ehr(&audit).expect("ehr");
    repo.commit_composition_unchecked(bp_composition(160.0), None, &audit)
        .expect("commit");

    let db = tmp.path().join("index").join("aql.db");
    let mut index = Index::open(&db).expect("open index");
    assert_eq!(index.build(&deployment, false).expect("build"), 1);
    // Freshness: re-building without changes re-indexes nothing.
    assert_eq!(index.build(&deployment, false).expect("rebuild"), 0);

    let aql = format!(
        "SELECT o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic \
         FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o[{BP}] \
         WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude > 140"
    );
    let result = execute(&index, &aql, &Params::new()).expect("query");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0][0], json!(160.0));
}

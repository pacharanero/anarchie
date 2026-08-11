// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use anarchie::knowledge::{ArtifactKind, DependencyIssueKind, Inventory};
use assert_cmd::Command;
use predicates::prelude::*;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ckm-inventory")
}

#[test]
fn inventories_ckm_metadata_and_dependency_evidence_deterministically() {
    let first = Inventory::scan(&fixture()).expect("inventory fixture");
    let second = Inventory::scan(&fixture()).expect("inventory fixture again");
    assert_eq!(first, second);

    assert_eq!(first.summary.artifacts, 4);
    assert_eq!(first.summary.archetypes, 3);
    assert_eq!(first.summary.published_archetypes, 2);
    assert_eq!(first.summary.templates, 1);
    assert_eq!(first.summary.termsets, 0);
    assert_eq!(first.summary.licenses["CC-BY-SA-3.0"], 1);
    assert_eq!(first.summary.licenses["CC-BY-SA-4.0"], 3);
    assert!(first.summary.parse_issues.is_empty());

    let child = first
        .artifacts
        .iter()
        .find(|artifact| artifact.id.as_deref() == Some("openEHR-EHR-CLUSTER.child.v1"))
        .expect("child archetype");
    assert_eq!(child.kind, ArtifactKind::Archetype);
    assert_eq!(child.adl_version.as_deref(), Some("1.4"));
    assert_eq!(child.revision.as_deref(), Some("0.0.1-alpha"));
    assert_eq!(child.languages, ["en"]);
    assert_eq!(child.sha256.len(), 64);
    assert_eq!(
        child.hard_dependencies,
        [
            "openEHR-EHR-CLUSTER.absent.v1",
            "openEHR-EHR-CLUSTER.parent.v1"
        ]
    );
    assert_eq!(
        child.slot_constraints,
        ["/openEHR-EHR-CLUSTER\\.device(-[a-zA-Z0-9_]+)*\\.v1/"]
    );

    assert_eq!(
        first
            .dependency_issues
            .iter()
            .filter(|issue| issue.kind == DependencyIssueKind::MissingHardDependency)
            .count(),
        2
    );
    assert_eq!(first.summary.dependency_issues["duplicate_artifact_id"], 2);
    assert_eq!(
        first.summary.dependency_issues["ambiguous_hard_dependency"],
        1
    );
    let remote = first
        .artifacts
        .iter()
        .find(|artifact| artifact.origin == "remote/example")
        .expect("remote source provenance");
    assert_eq!(remote.revision.as_deref(), Some("1.0.1"));
    let mismatch = first
        .dependency_issues
        .iter()
        .find(|issue| issue.kind == DependencyIssueKind::MajorVersionMismatch)
        .expect("major-version mismatch");
    assert_eq!(mismatch.dependency, "openEHR-EHR-CLUSTER.parent.v2");
    assert_eq!(mismatch.candidates, ["openEHR-EHR-CLUSTER.parent.v1"]);
}

#[test]
fn knowledge_inventory_cli_emits_text_and_json() {
    Command::cargo_bin("anarchie")
        .expect("binary")
        .args(["knowledge", "inventory"])
        .arg(fixture())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Artifacts: 4")
                .and(predicate::str::contains("Archetypes: 3 (2 published)"))
                .and(predicate::str::contains("major_version_mismatch: 1")),
        );

    let output = Command::cargo_bin("anarchie")
        .expect("binary")
        .args(["--format", "json", "knowledge", "inventory"])
        .arg(fixture())
        .output()
        .expect("run inventory");
    assert!(output.status.success());
    let inventory: Inventory = serde_json::from_slice(&output.stdout).expect("inventory JSON");
    assert_eq!(inventory.summary.artifacts, 4);
    assert_eq!(inventory.dependency_issues.len(), 6);
}

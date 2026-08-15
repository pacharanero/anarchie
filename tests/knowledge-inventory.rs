// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;

use anarchie::knowledge::{
    build_international_package, ArtifactKind, DecisionReason, DependencyIssueKind, Inventory,
    KnowledgeLock, KnowledgeManifest, KnowledgeStatus, KnowledgeStatusState, PublicationError,
    ResolutionIssueKind,
};
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ckm-inventory")
}

fn manifest(name: &str) -> PathBuf {
    fixture().join(name)
}

fn deployment() -> TempDir {
    let temporary = tempfile::tempdir().expect("temporary deployment directory");
    anarchie::store::Deployment::init(
        temporary.path(),
        anarchie::store::DeploymentConfig::new("knowledge.test"),
    )
    .expect("initialise deployment");
    temporary
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

#[test]
fn resolves_policy_and_hard_dependency_closure_deterministically() {
    let inventory = Inventory::scan(&fixture()).expect("inventory fixture");
    let manifest = KnowledgeManifest::from_path(&manifest("knowledge-success.toml"))
        .expect("knowledge manifest");
    let first = manifest.resolve(&inventory).expect("resolve manifest");
    let second = manifest
        .resolve(&inventory)
        .expect("resolve manifest again");

    assert_eq!(first, second);
    assert!(first.is_success());
    assert_eq!(first.lock.artefacts.len(), 2);
    assert_eq!(
        first
            .lock
            .artefacts
            .iter()
            .map(|artefact| artefact.id.as_str())
            .collect::<Vec<_>>(),
        [
            "openEHR-EHR-CLUSTER.child.v1",
            "openEHR-EHR-CLUSTER.parent.v1"
        ]
    );
    assert_eq!(first.issues.len(), 1);
    assert_eq!(
        first.issues[0].kind,
        ResolutionIssueKind::MissingHardDependency
    );
    assert!(first.issues[0].allowed);
    assert_eq!(
        first.explain("openEHR-EHR-CLUSTER.parent.v1")[0].reason,
        DecisionReason::HardDependency
    );
    assert_eq!(
        first.lock.to_toml().expect("render lock"),
        second.lock.to_toml().expect("render lock")
    );
}

#[test]
fn publishes_a_deterministic_closed_ckm_source_package_with_evidence() {
    let output = tempfile::tempdir().expect("temporary package output");
    let first_archive = output.path().join("first.tar.zst");
    let second_archive = output.path().join("second.tar.zst");
    let mut manifest = KnowledgeManifest::from_path(&manifest("knowledge-success.toml"))
        .expect("knowledge manifest");
    manifest.artefacts.include = vec!["openEHR-EHR-CLUSTER.parent.v1".to_string()];
    manifest.policy.allow_missing_hard_dependencies = false;

    let first = build_international_package(&fixture(), &manifest, "2026.8.1", &first_archive)
        .expect("publish package");
    let second = build_international_package(&fixture(), &manifest, "2026.8.1", &second_archive)
        .expect("publish package again");

    assert_eq!(first, second);
    assert_eq!(first.included_artefacts, 1);
    assert_eq!(first.excluded_artefacts, 3);
    assert_eq!(first.allowed_issues, 0);
    assert_eq!(
        std::fs::read(&first_archive).expect("read first archive"),
        std::fs::read(&second_archive).expect("read second archive")
    );
    let package =
        anarchie::knowledge::PackageArchive::verify(&first_archive).expect("verify package");
    assert_eq!(package.manifest.name, "ckm-international");
    assert_eq!(package.manifest.version, "2026.8.1");
    assert!(package
        .files
        .iter()
        .any(|file| file.path == "provenance/knowledge.lock"));
    assert!(package
        .files
        .iter()
        .any(|file| file.path == "provenance/inclusion-report.json"));
}

#[test]
fn refuses_to_publish_a_ckm_package_with_blocking_resolution_issues() {
    let output = tempfile::tempdir().expect("temporary package output");
    let archive = output.path().join("blocked.tar.zst");
    let manifest = KnowledgeManifest::from_path(&manifest("knowledge-failure.toml"))
        .expect("knowledge manifest");

    assert!(matches!(
        build_international_package(&fixture(), &manifest, "2026.8.1", &archive),
        Err(PublicationError::ResolutionBlocked(_))
    ));
    assert!(!archive.exists());
}

#[test]
fn refuses_major_version_substitution_and_does_not_write_a_lock() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let lock = temporary.path().join("knowledge.lock");
    std::fs::write(&lock, "existing lock\n").expect("write existing lock");

    Command::cargo_bin("anarchie")
        .expect("binary")
        .args(["knowledge", "resolve"])
        .arg(fixture())
        .args(["--manifest"])
        .arg(manifest("knowledge-failure.toml"))
        .args(["--lock"])
        .arg(&lock)
        .assert()
        .failure()
        .stdout(predicate::str::contains("Blocking issues: 1"))
        .stderr(predicate::str::contains("lock not written"));

    assert_eq!(
        std::fs::read_to_string(&lock).expect("read preserved lock"),
        "existing lock\n"
    );
}

#[test]
fn resolve_writes_a_stable_lock_and_why_explains_selection() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let lock = temporary.path().join("knowledge.lock");

    let resolve = || {
        Command::cargo_bin("anarchie")
            .expect("binary")
            .args(["knowledge", "resolve"])
            .arg(fixture())
            .args(["--manifest"])
            .arg(manifest("knowledge-success.toml"))
            .args(["--lock"])
            .arg(&lock)
            .assert()
            .success()
            .stdout(predicate::str::contains("Selected artefacts: 2"));
    };
    resolve();
    let first = std::fs::read_to_string(&lock).expect("read generated lock");
    resolve();
    let second = std::fs::read_to_string(&lock).expect("read replaced lock");
    assert_eq!(first, second);
    assert!(first.contains("inventory-sha256"));
    assert!(first.contains("openEHR-EHR-CLUSTER.child.v1"));

    Command::cargo_bin("anarchie")
        .expect("binary")
        .args(["knowledge", "why", "openEHR-EHR-CLUSTER.parent.v1"])
        .arg(fixture())
        .args(["--manifest"])
        .arg(manifest("knowledge-success.toml"))
        .assert()
        .success()
        .stdout(
            predicate::str::contains("included").and(predicate::str::contains("HardDependency")),
        );
}

#[test]
fn implicit_selection_blocks_duplicate_ids_across_allowed_origins() {
    let inventory = Inventory::scan(&fixture()).expect("inventory fixture");
    let mut manifest = KnowledgeManifest::from_path(&manifest("knowledge-success.toml"))
        .expect("knowledge manifest");
    manifest.artefacts.include.clear();
    manifest.source.origins = vec!["local".to_string(), "remote".to_string()];

    let resolution = manifest.resolve(&inventory).expect("resolve manifest");
    assert!(!resolution.is_success());
    assert!(resolution
        .issues
        .iter()
        .any(|issue| issue.kind == ResolutionIssueKind::DuplicateArtefactId));
}

#[test]
fn a_dependency_excluded_by_origin_is_not_tolerated_as_missing() {
    let mut inventory = Inventory::scan(&fixture()).expect("inventory fixture");
    inventory
        .artifacts
        .retain(|artefact| artefact.path != "local/archetypes/openEHR-EHR-CLUSTER.parent.v1.adl");
    let manifest = KnowledgeManifest::from_path(&manifest("knowledge-success.toml"))
        .expect("knowledge manifest");

    let resolution = manifest.resolve(&inventory).expect("resolve manifest");
    assert!(!resolution.is_success());
    assert!(resolution.issues.iter().any(|issue| {
        issue.kind == ResolutionIssueKind::PolicyExcludedHardDependency && !issue.allowed
    }));
}

#[test]
fn deployment_resolve_writes_default_lock_and_status_tracks_freshness() {
    let deployment = deployment();
    let manifest_path = deployment.path().join("knowledge.toml");
    std::fs::write(
        &manifest_path,
        std::fs::read_to_string(manifest("knowledge-success.toml")).unwrap(),
    )
    .expect("write deployment manifest");
    let lock = deployment.path().join("knowledge.lock");

    Command::cargo_bin("anarchie")
        .expect("binary")
        .current_dir(deployment.path())
        .args(["knowledge", "status"])
        .arg(fixture())
        .assert()
        .success()
        .stdout(predicate::str::contains("State: unresolved"));

    Command::cargo_bin("anarchie")
        .expect("binary")
        .current_dir(deployment.path())
        .args(["knowledge", "resolve"])
        .arg(fixture())
        .assert()
        .success()
        .stdout(predicate::str::contains("Lock: ").and(predicate::str::contains("knowledge.lock")));
    assert!(lock.exists());

    Command::cargo_bin("anarchie")
        .expect("binary")
        .current_dir(deployment.path())
        .args(["knowledge", "status"])
        .arg(fixture())
        .assert()
        .success()
        .stdout(predicate::str::contains("State: current"));

    std::fs::write(
        &manifest_path,
        "version = 1\n\n[knowledge]\nname = \"changed-policy\"\n",
    )
    .expect("change deployment manifest");
    Command::cargo_bin("anarchie")
        .expect("binary")
        .current_dir(deployment.path())
        .args(["knowledge", "status"])
        .arg(fixture())
        .assert()
        .success()
        .stdout(predicate::str::contains("State: stale"));
}

#[test]
fn status_compares_manifest_source_and_inventory_evidence() {
    let inventory = Inventory::scan(&fixture()).expect("inventory fixture");
    let manifest = KnowledgeManifest::from_path(&manifest("knowledge-success.toml"))
        .expect("knowledge manifest");
    let resolution = manifest.resolve(&inventory).expect("resolve manifest");
    let lock = resolution.lock;

    assert_eq!(
        KnowledgeStatus::inspect(&manifest, Some(&lock), &inventory)
            .expect("current status")
            .state,
        KnowledgeStatusState::Current
    );
    assert_eq!(
        KnowledgeStatus::inspect(&manifest, None, &inventory)
            .expect("unresolved status")
            .state,
        KnowledgeStatusState::Unresolved
    );
    let mut changed_inventory = inventory.clone();
    changed_inventory.artifacts[0].sha256 = "0".repeat(64);
    assert_eq!(
        KnowledgeStatus::inspect(&manifest, Some(&lock), &changed_inventory)
            .expect("stale source status")
            .state,
        KnowledgeStatusState::Stale
    );

    let temporary = tempfile::tempdir().expect("temporary directory");
    let lock_path = temporary.path().join("knowledge.lock");
    lock.write_atomic(&lock_path).expect("write lock");
    assert_eq!(
        KnowledgeLock::from_path(&lock_path)
            .expect("read lock")
            .knowledge
            .manifest_sha256,
        manifest.digest().expect("manifest digest")
    );
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! End-to-end tests for the git-backed store: init, create EHR, commit
//! versions, read head and history, and diff.

use std::path::PathBuf;

use anarchie_rm::Composition;
use anarchie_store::{Audit, ChangeType, Deployment, DeploymentConfig};

fn fixture() -> Composition {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("anarchie-rm")
        .join("tests")
        .join("fixtures")
        .join("blood-pressure-composition.json");
    let json = std::fs::read_to_string(&path).expect("read fixture");
    anarchie_rm::from_canonical_str(&json).expect("parse fixture")
}

fn audit(change_type: ChangeType, description: &str) -> Audit {
    Audit::now("Dr A. Smith", "a.smith@example.org", change_type, description)
}

#[test]
fn init_creates_the_expected_skeleton() {
    let tmp = tempfile::tempdir().unwrap();
    let deployment =
        Deployment::init(tmp.path(), DeploymentConfig::new("anarchie.test")).unwrap();

    assert!(tmp.path().join("anarchie.toml").exists());
    assert!(tmp.path().join("templates").join("index.json").exists());
    assert!(tmp.path().join("index").join(".gitignore").exists());
    assert!(tmp.path().join("ehrs").is_dir());
    assert_eq!(deployment.config().system_id, "anarchie.test");
}

#[test]
fn init_refuses_to_overwrite_an_existing_deployment() {
    let tmp = tempfile::tempdir().unwrap();
    Deployment::init(tmp.path(), DeploymentConfig::new("anarchie.test")).unwrap();
    let err = Deployment::init(tmp.path(), DeploymentConfig::new("anarchie.test"));
    assert!(err.is_err());
}

#[test]
fn open_finds_the_deployment_from_a_subdirectory() {
    let tmp = tempfile::tempdir().unwrap();
    Deployment::init(tmp.path(), DeploymentConfig::new("anarchie.test")).unwrap();
    let nested = tmp.path().join("ehrs");
    let reopened = Deployment::open(&nested).unwrap();
    assert_eq!(reopened.root(), tmp.path());
}

#[test]
fn commit_assigns_a_version_uid_and_writes_head_to_the_working_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let deployment =
        Deployment::init(tmp.path(), DeploymentConfig::new("anarchie.test")).unwrap();
    let repo = deployment.create_ehr(&audit(ChangeType::Creation, "Create EHR")).unwrap();

    let outcome = repo
        .commit_composition(fixture(), None, &audit(ChangeType::Creation, "Admission vitals"))
        .unwrap();

    assert_eq!(
        outcome.version_uid,
        format!("{}::anarchie.test::1", outcome.object_id)
    );

    let head = repo.cat_head(&outcome.object_id).unwrap();
    assert!(head.contains(&outcome.version_uid));
    assert!(head.ends_with('\n'));
}

#[test]
fn second_commit_increments_the_version_tree_id() {
    let tmp = tempfile::tempdir().unwrap();
    let deployment =
        Deployment::init(tmp.path(), DeploymentConfig::new("anarchie.test")).unwrap();
    let repo = deployment.create_ehr(&audit(ChangeType::Creation, "Create EHR")).unwrap();

    let first = repo
        .commit_composition(fixture(), None, &audit(ChangeType::Creation, "v1"))
        .unwrap();
    let second = repo
        .commit_composition(
            fixture(),
            Some(first.object_id.clone()),
            &audit(ChangeType::Modification, "v2"),
        )
        .unwrap();

    assert_eq!(first.object_id, second.object_id);
    assert!(second.version_uid.ends_with("::2"));

    let history = repo.log(&first.object_id).unwrap();
    assert_eq!(history.len(), 2);
    // Newest first.
    assert!(history[0].version_uid.ends_with("::2"));
    assert!(history[1].version_uid.ends_with("::1"));
}

#[test]
fn cat_version_reconstructs_an_older_version_from_git() {
    let tmp = tempfile::tempdir().unwrap();
    let deployment =
        Deployment::init(tmp.path(), DeploymentConfig::new("anarchie.test")).unwrap();
    let repo = deployment.create_ehr(&audit(ChangeType::Creation, "Create EHR")).unwrap();

    let first = repo
        .commit_composition(fixture(), None, &audit(ChangeType::Creation, "v1"))
        .unwrap();
    repo.commit_composition(
        fixture(),
        Some(first.object_id.clone()),
        &audit(ChangeType::Modification, "v2"),
    )
    .unwrap();

    let v1 = repo.cat_version(&first.version_uid).unwrap();
    assert!(v1.contains("::anarchie.test::1"));
    assert!(!v1.contains("::anarchie.test::2"));
}

#[test]
fn diff_between_versions_shows_the_version_uid_change() {
    let tmp = tempfile::tempdir().unwrap();
    let deployment =
        Deployment::init(tmp.path(), DeploymentConfig::new("anarchie.test")).unwrap();
    let repo = deployment.create_ehr(&audit(ChangeType::Creation, "Create EHR")).unwrap();

    let first = repo
        .commit_composition(fixture(), None, &audit(ChangeType::Creation, "v1"))
        .unwrap();
    repo.commit_composition(
        fixture(),
        Some(first.object_id.clone()),
        &audit(ChangeType::Modification, "v2"),
    )
    .unwrap();

    let diff = repo.diff(&first.object_id, 1, 2).unwrap();
    assert!(diff.contains("::anarchie.test::1"));
    assert!(diff.contains("::anarchie.test::2"));
}

#[test]
fn list_ehrs_and_compositions_reflect_what_was_created() {
    let tmp = tempfile::tempdir().unwrap();
    let deployment =
        Deployment::init(tmp.path(), DeploymentConfig::new("anarchie.test")).unwrap();
    let repo = deployment.create_ehr(&audit(ChangeType::Creation, "Create EHR")).unwrap();
    let outcome = repo
        .commit_composition(fixture(), None, &audit(ChangeType::Creation, "v1"))
        .unwrap();

    let ehrs = deployment.list_ehrs().unwrap();
    assert_eq!(ehrs, vec![repo.ehr_id().to_string()]);

    let comps = repo.list_compositions().unwrap();
    assert_eq!(comps, vec![outcome.object_id]);
}

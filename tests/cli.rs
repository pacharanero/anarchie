// SPDX-License-Identifier: AGPL-3.0-or-later
//! End-to-end tests of the `anarchie` binary itself - argument parsing, command
//! dispatch, stdout, and exit codes. The library is covered by the other test
//! files; these lock the user-facing CLI surface by running the real binary.

use std::path::PathBuf;

use assert_cmd::Command;
use predicates::prelude::*;

/// The canonical blood-pressure fixture (validates against
/// `vital_signs_encounter.v1`).
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("blood-pressure-composition.json")
}

fn anarchie() -> Command {
    Command::cargo_bin("anarchie").expect("the `anarchie` binary builds")
}

/// `anarchie init` in a fresh temp dir, returning the dir handle.
fn init_deployment() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    anarchie()
        .current_dir(&dir)
        .args(["init", "--system-id", "test.local"])
        .assert()
        .success();
    dir
}

#[test]
fn version_prints_name_and_version() {
    anarchie()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("anarchie 0.1.0"));
}

#[test]
fn help_lists_the_commands() {
    anarchie().arg("--help").assert().success().stdout(
        predicate::str::contains("commit")
            .and(predicate::str::contains("validate"))
            .and(predicate::str::contains("serve"))
            .and(predicate::str::contains("mcp")),
    );
}

#[test]
fn no_subcommand_is_a_usage_error() {
    anarchie().assert().failure().code(2);
}

#[test]
fn info_summarises_a_composition() {
    anarchie()
        .arg("info")
        .arg(fixture())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Composition: Blood pressure")
                .and(predicate::str::contains("vital_signs_encounter.v1"))
                .and(predicate::str::contains("entries:       1")),
        );
}

#[test]
fn init_seeds_the_ips_starter_templates() {
    let dir = init_deployment();
    anarchie()
        .current_dir(&dir)
        .args(["template", "list"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("vital_signs_encounter.v1")
                .and(predicate::str::contains("medication_list.v1"))
                .and(predicate::str::contains("problem_list.v1")),
        );
}

#[test]
fn init_minimal_seeds_no_templates() {
    let dir = tempfile::tempdir().unwrap();
    anarchie()
        .current_dir(&dir)
        .args(["init", "--minimal", "--system-id", "test.local"])
        .assert()
        .success()
        .stdout(predicate::str::contains("none (--minimal)"));
}

#[test]
fn pack_list_shows_the_bundled_ips_core() {
    anarchie()
        .args(["pack", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ips-core"));
}

#[test]
fn commit_lifecycle_log_cat_and_fsck() {
    let dir = init_deployment();

    // `ehr new` prints the new EHR id.
    let out = anarchie()
        .current_dir(&dir)
        .args(["ehr", "new"])
        .assert()
        .success();
    let ehr = String::from_utf8_lossy(&out.get_output().stdout)
        .trim()
        .to_string();
    assert_eq!(ehr.len(), 36, "ehr new should print a UUID, got {ehr:?}");

    // Commit with a message.
    let commit = anarchie()
        .current_dir(&dir)
        .args(["commit", &ehr])
        .arg(fixture())
        .args(["-m", "Admission vitals"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Committed"));
    let commit_out = String::from_utf8_lossy(&commit.get_output().stdout).to_string();
    let object_id = commit_out
        .lines()
        .find_map(|l| l.trim().strip_prefix("object_id:"))
        .map(|s| s.trim().to_string())
        .expect("commit output carries an object_id line");

    // The `-m` message is the commit subject, so it surfaces in `log`.
    anarchie()
        .current_dir(&dir)
        .args(["log", &ehr, &object_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("Admission vitals"));

    // `cat` reads the head back as canonical JSON.
    anarchie()
        .current_dir(&dir)
        .args(["cat", &ehr, &object_id])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"_type\": \"COMPOSITION\""));

    // `fsck` reports a clean store.
    anarchie()
        .current_dir(&dir)
        .arg("fsck")
        .assert()
        .success()
        .stdout(predicate::str::contains("Store is clean."));
}

#[test]
fn commit_rejects_an_out_of_range_value() {
    let dir = init_deployment();
    let out = anarchie()
        .current_dir(&dir)
        .args(["ehr", "new"])
        .assert()
        .success();
    let ehr = String::from_utf8_lossy(&out.get_output().stdout)
        .trim()
        .to_string();

    // Make the systolic value (128) wildly out of range.
    let bad = std::fs::read_to_string(fixture())
        .unwrap()
        .replace("128.0", "9000.0");
    let bad_path = dir.path().join("bad.json");
    std::fs::write(&bad_path, bad).unwrap();

    anarchie()
        .current_dir(&dir)
        .args(["commit", &ehr])
        .arg(&bad_path)
        .assert()
        .failure()
        .stdout(predicate::str::contains("outside permitted range"));
}

#[test]
fn index_then_aql_counts_the_compositions() {
    let dir = init_deployment();
    let out = anarchie()
        .current_dir(&dir)
        .args(["ehr", "new"])
        .assert()
        .success();
    let ehr = String::from_utf8_lossy(&out.get_output().stdout)
        .trim()
        .to_string();
    anarchie()
        .current_dir(&dir)
        .args(["commit", &ehr])
        .arg(fixture())
        .assert()
        .success();

    anarchie()
        .current_dir(&dir)
        .arg("index")
        .assert()
        .success()
        .stdout(predicate::str::contains("Indexed 1 composition"));

    anarchie()
        .current_dir(&dir)
        .args(["aql", "SELECT COUNT(*) FROM COMPOSITION c"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"rows\"").and(predicate::str::contains('1')));
}

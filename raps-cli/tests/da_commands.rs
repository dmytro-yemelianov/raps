//! Integration tests for Design Automation commands
//!
//! Tests CLI argument parsing, help output, and error handling for DA commands.

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_da_help() {
    raps()
        .args(["da", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Design Automation"))
        .stdout(predicate::str::contains("engines"))
        .stdout(predicate::str::contains("appbundles"))
        .stdout(predicate::str::contains("activities"))
        .stdout(predicate::str::contains("run"));
}

#[test]
fn test_da_engines_help() {
    raps()
        .args(["da", "engines", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List available engines"));
}

#[test]
fn test_da_appbundles_help() {
    raps()
        .args(["da", "appbundles", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List app bundles"));
}

#[test]
fn test_da_appbundle_create_help() {
    raps()
        .args(["da", "appbundle-create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Create an app bundle"))
        .stdout(predicate::str::contains("--id"))
        .stdout(predicate::str::contains("--engine"));
}

#[test]
fn test_da_appbundle_delete_help() {
    raps()
        .args(["da", "appbundle-delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Delete an app bundle"))
        .stdout(predicate::str::contains("ID"));
}

#[test]
fn test_da_activities_help() {
    raps()
        .args(["da", "activities", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List activities"));
}

#[test]
fn test_da_activity_create_help() {
    raps()
        .args(["da", "activity-create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Create an activity"))
        .stdout(predicate::str::contains("--file"))
        .stdout(predicate::str::contains("--id"))
        .stdout(predicate::str::contains("--engine"));
}

#[test]
fn test_da_activity_delete_help() {
    raps()
        .args(["da", "activity-delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Delete an activity"))
        .stdout(predicate::str::contains("ID"));
}

#[test]
fn test_da_run_help() {
    raps()
        .args(["da", "run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Submit a work item"))
        .stdout(predicate::str::contains("ACTIVITY"))
        .stdout(predicate::str::contains("--input"))
        .stdout(predicate::str::contains("--output"));
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_da_appbundle_delete_requires_id() {
    raps()
        .args(["da", "appbundle-delete"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ID").or(predicate::str::contains("required")));
}

#[test]
fn test_da_activity_delete_requires_id() {
    raps()
        .args(["da", "activity-delete"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ID").or(predicate::str::contains("required")));
}

#[test]
fn test_da_run_requires_activity() {
    raps()
        .args(["da", "run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ACTIVITY").or(predicate::str::contains("required")));
}

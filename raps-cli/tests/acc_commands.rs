//! Integration tests for ACC commands
//!
//! Tests CLI argument parsing, help output, and error handling for ACC commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_acc_help() {
    raps()
        .args(["acc", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("asset"))
        .stdout(predicate::str::contains("submittal"))
        .stdout(predicate::str::contains("checklist"));
}

#[test]
fn test_acc_asset_help() {
    raps()
        .args(["acc", "asset", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn test_acc_submittal_help() {
    raps()
        .args(["acc", "submittal", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn test_acc_checklist_help() {
    raps()
        .args(["acc", "checklist", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"));
}

// ==================== Missing Args Tests ====================

#[test]
fn test_acc_no_subcommand() {
    raps()
        .args(["acc"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommand"));
}

#[test]
fn test_acc_asset_list_missing_project() {
    raps()
        .args(["acc", "asset", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("PROJECT_ID"));
}

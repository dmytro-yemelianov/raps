//! Integration tests for issue commands
//!
//! Tests CLI argument parsing, help output, and error handling for issue commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_issue_help() {
    raps()
        .args(["issue", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("types"))
        .stdout(predicate::str::contains("comment"));
}

#[test]
fn test_issue_list_help() {
    raps()
        .args(["issue", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PROJECT_ID"))
        .stdout(predicate::str::contains("--status"))
        .stdout(predicate::str::contains("--since"));
}

#[test]
fn test_issue_create_help() {
    raps()
        .args(["issue", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PROJECT_ID"))
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--description"));
}

#[test]
fn test_issue_update_help() {
    raps()
        .args(["issue", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PROJECT_ID"))
        .stdout(predicate::str::contains("ISSUE_ID"))
        .stdout(predicate::str::contains("--status"))
        .stdout(predicate::str::contains("--title"));
}

// ==================== Missing Args Tests ====================

#[test]
fn test_issue_list_missing_project() {
    raps()
        .args(["issue", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("PROJECT_ID"));
}

#[test]
fn test_issue_create_missing_project() {
    raps()
        .args(["issue", "create"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("PROJECT_ID"));
}

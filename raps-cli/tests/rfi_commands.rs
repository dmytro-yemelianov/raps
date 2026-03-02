//! Integration tests for RFI commands
//!
//! Tests CLI argument parsing, help output, and error handling for RFI commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_rfi_help() {
    raps()
        .args(["rfi", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"));
}

#[test]
fn test_rfi_list_help() {
    raps()
        .args(["rfi", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--status"))
        .stdout(predicate::str::contains("--since"));
}

#[test]
fn test_rfi_create_help() {
    raps()
        .args(["rfi", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--question"))
        .stdout(predicate::str::contains("--priority"))
        .stdout(predicate::str::contains("--due-date"));
}

#[test]
fn test_rfi_update_help() {
    raps()
        .args(["rfi", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--status"));
}

#[test]
fn test_rfi_get_help() {
    raps().args(["rfi", "get", "--help"]).assert().success();
}

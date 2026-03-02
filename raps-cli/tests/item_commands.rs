//! Integration tests for item commands
//!
//! Tests CLI argument parsing, help output, and error handling for item commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_item_help() {
    raps()
        .args(["item", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("info"))
        .stdout(predicate::str::contains("versions"))
        .stdout(predicate::str::contains("create-from-oss"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("rename"));
}

#[test]
fn test_item_info_help() {
    raps()
        .args(["item", "info", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PROJECT_ID").or(predicate::str::contains("project_id")))
        .stdout(predicate::str::contains("ITEM_ID").or(predicate::str::contains("item_id")));
}

#[test]
fn test_item_create_from_oss_help() {
    raps()
        .args(["item", "create-from-oss", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--name"))
        .stdout(predicate::str::contains("--object-id"));
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_item_no_subcommand() {
    raps()
        .arg("item")
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommand"));
}

#[test]
fn test_item_info_requires_project_id() {
    raps()
        .args(["item", "info"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("PROJECT_ID").or(predicate::str::contains("required")));
}

#[test]
fn test_item_info_requires_item_id() {
    raps()
        .args(["item", "info", "some-project"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ITEM_ID").or(predicate::str::contains("required")));
}

#[test]
fn test_item_create_from_oss_requires_flags() {
    raps()
        .args(["item", "create-from-oss", "proj-id", "folder-id"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--name").or(predicate::str::contains("required")));
}

// ==================== No-Credentials Tests ====================

#[test]
fn test_item_info_no_credentials() {
    raps()
        .args(["item", "info", "proj-id", "item-id", "--non-interactive"])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .failure();
}

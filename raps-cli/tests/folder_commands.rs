//! Integration tests for folder commands
//!
//! Tests CLI argument parsing, help output, and error handling for folder commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_folder_help() {
    raps()
        .args(["folder", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("rename"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("rights"));
}

#[test]
fn test_folder_create_help() {
    raps()
        .args(["folder", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--name"));
}

#[test]
fn test_folder_delete_help() {
    raps()
        .args(["folder", "delete", "--help"])
        .assert()
        .success();
}

#[test]
fn test_folder_rights_help() {
    raps()
        .args(["folder", "rights", "--help"])
        .assert()
        .success();
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_folder_no_subcommand() {
    raps()
        .arg("folder")
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommand"));
}

#[test]
fn test_folder_create_non_interactive_requires_name() {
    raps()
        .args([
            "folder",
            "create",
            "proj-id",
            "parent-id",
            "--non-interactive",
        ])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .failure();
}

#[test]
fn test_folder_rename_non_interactive_requires_name() {
    raps()
        .args([
            "folder",
            "rename",
            "proj-id",
            "folder-id",
            "--non-interactive",
        ])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .failure();
}

// ==================== No-Credentials Tests ====================

#[test]
fn test_folder_list_no_credentials() {
    raps()
        .args([
            "folder",
            "list",
            "proj-id",
            "folder-id",
            "--non-interactive",
        ])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .failure();
}

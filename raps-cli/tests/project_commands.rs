//! Integration tests for project commands
//!
//! Tests CLI argument parsing, help output, and error handling for project commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_project_help() {
    raps()
        .args(["project", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("project"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("info"));
}

#[test]
fn test_project_list_help() {
    raps()
        .args(["project", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("HUB_ID").or(predicate::str::contains("hub_id")));
}

#[test]
fn test_project_info_help() {
    raps()
        .args(["project", "info", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("HUB_ID").or(predicate::str::contains("hub_id")))
        .stdout(predicate::str::contains("PROJECT_ID").or(predicate::str::contains("project_id")));
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_project_no_subcommand() {
    raps()
        .arg("project")
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommand"));
}

#[test]
fn test_project_list_non_interactive_no_hub() {
    raps()
        .args(["project", "list", "--non-interactive"])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .failure();
}

#[test]
fn test_project_info_non_interactive_no_args() {
    raps()
        .args(["project", "info", "--non-interactive"])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .failure();
}

// ==================== No-Credentials Tests ====================

#[test]
fn test_project_list_no_credentials() {
    raps()
        .args(["project", "list", "fake-hub", "--non-interactive"])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .failure();
}

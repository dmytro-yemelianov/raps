//! Integration tests for hub commands
//!
//! Tests CLI argument parsing, help output, and error handling for hub commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_hub_help() {
    raps()
        .args(["hub", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hub"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("info"));
}

#[test]
fn test_hub_list_help() {
    raps()
        .args(["hub", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List"));
}

#[test]
fn test_hub_info_help() {
    raps()
        .args(["hub", "info", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("HUB_ID").or(predicate::str::contains("hub_id")));
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_hub_info_requires_hub_id() {
    raps()
        .args(["hub", "info"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("HUB_ID").or(predicate::str::contains("required")));
}

#[test]
fn test_hub_no_subcommand() {
    raps()
        .arg("hub")
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommand"));
}

// ==================== No-Credentials Tests ====================

#[test]
fn test_hub_list_no_credentials() {
    raps()
        .args(["hub", "list", "--non-interactive"])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .failure();
}

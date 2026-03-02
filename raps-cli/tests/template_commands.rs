//! Integration tests for template commands
//!
//! Tests CLI argument parsing, help output, and error handling for template commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_template_help() {
    raps()
        .args(["template", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("info"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("archive"));
}

#[test]
fn test_template_create_help() {
    raps()
        .args(["template", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--name"))
        .stdout(predicate::str::contains("--account").or(predicate::str::contains("-a")));
}

#[test]
fn test_template_info_help() {
    raps()
        .args(["template", "info", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("TEMPLATE_ID").or(predicate::str::contains("template_id")),
        );
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_template_no_subcommand() {
    raps()
        .arg("template")
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommand"));
}

#[test]
fn test_template_info_requires_template_id() {
    raps()
        .args(["template", "info"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("TEMPLATE_ID").or(predicate::str::contains("required")));
}

#[test]
fn test_template_create_requires_name() {
    raps()
        .args(["template", "create"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--name").or(predicate::str::contains("required")));
}

// ==================== No-Credentials Tests ====================

#[test]
fn test_template_list_requires_account() {
    raps()
        .args(["template", "list"])
        .env_remove("APS_ACCOUNT_ID")
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .failure();
}

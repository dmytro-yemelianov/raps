//! Integration tests for inspect commands
//!
//! Tests CLI argument parsing, help output, and error handling for inspect commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_inspect_help() {
    raps()
        .args(["inspect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("zip"));
}

#[test]
fn test_inspect_zip_help() {
    raps()
        .args(["inspect", "zip", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("BUCKET").or(predicate::str::contains("bucket")))
        .stdout(predicate::str::contains("OBJECT").or(predicate::str::contains("object")));
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_inspect_no_subcommand() {
    raps()
        .arg("inspect")
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommand"));
}

// ==================== No-Credentials Tests ====================

#[test]
fn test_inspect_zip_no_credentials() {
    raps()
        .args([
            "inspect",
            "zip",
            "fake-bucket",
            "fake-obj",
            "--non-interactive",
        ])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .failure();
}

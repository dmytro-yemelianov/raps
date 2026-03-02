//! Integration tests for job commands
//!
//! Tests CLI argument parsing, help output, and error handling for job commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_job_help() {
    raps()
        .args(["job", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("cancel"));
}

#[test]
fn test_job_status_help() {
    raps()
        .args(["job", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID").or(predicate::str::contains("id")))
        .stdout(predicate::str::contains("--wait"))
        .stdout(predicate::str::contains("--poll-secs"));
}

#[test]
fn test_job_list_help() {
    raps()
        .args(["job", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--state"));
}

#[test]
fn test_job_cancel_help() {
    raps()
        .args(["job", "cancel", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MACHINE_ID").or(predicate::str::contains("machine_id")));
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_job_status_requires_id() {
    raps()
        .args(["job", "status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ID").or(predicate::str::contains("required")));
}

#[test]
fn test_job_cancel_requires_machine_id() {
    raps()
        .args(["job", "cancel"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("MACHINE_ID").or(predicate::str::contains("required")));
}

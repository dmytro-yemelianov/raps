//! Integration tests for report commands
//!
//! Tests CLI argument parsing, help output, and error handling for report commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_report_help() {
    raps()
        .args(["report", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("rfi-summary"))
        .stdout(predicate::str::contains("issues-summary"))
        .stdout(predicate::str::contains("submittals-summary"))
        .stdout(predicate::str::contains("checklists-summary"))
        .stdout(predicate::str::contains("assets-summary"));
}

#[test]
fn test_report_rfi_summary_help() {
    raps()
        .args(["report", "rfi-summary", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--account"))
        .stdout(predicate::str::contains("--filter"))
        .stdout(predicate::str::contains("--status"))
        .stdout(predicate::str::contains("--since"));
}

#[test]
fn test_report_issues_summary_help() {
    raps()
        .args(["report", "issues-summary", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--account"))
        .stdout(predicate::str::contains("--filter"))
        .stdout(predicate::str::contains("--status"));
}

#[test]
fn test_report_submittals_summary_help() {
    raps()
        .args(["report", "submittals-summary", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--account"))
        .stdout(predicate::str::contains("--filter"));
}

#[test]
fn test_report_checklists_summary_help() {
    raps()
        .args(["report", "checklists-summary", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--account"))
        .stdout(predicate::str::contains("--filter"));
}

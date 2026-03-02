//! Integration tests for admin commands
//!
//! Tests CLI argument parsing, help output, and error handling for admin commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_admin_help() {
    raps()
        .args(["admin", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("user"))
        .stdout(predicate::str::contains("folder"))
        .stdout(predicate::str::contains("project"))
        .stdout(predicate::str::contains("operation"))
        .stdout(predicate::str::contains("company-list"));
}

#[test]
fn test_admin_user_help() {
    raps()
        .args(["admin", "user", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("remove"));
}

#[test]
fn test_admin_user_list_help() {
    raps()
        .args(["admin", "user", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--account"))
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--role"));
}

#[test]
fn test_admin_folder_help() {
    raps()
        .args(["admin", "folder", "--help"])
        .assert()
        .success();
}

#[test]
fn test_admin_project_help() {
    raps()
        .args(["admin", "project", "--help"])
        .assert()
        .success();
}

#[test]
fn test_admin_operation_help() {
    raps()
        .args(["admin", "operation", "--help"])
        .assert()
        .success();
}

#[test]
fn test_admin_company_list_help() {
    raps()
        .args(["admin", "company-list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--account"));
}

// ==================== Missing Args Tests ====================

#[test]
fn test_admin_user_add_missing_args() {
    raps().args(["admin", "user", "add"]).assert().failure();
}

#[test]
fn test_admin_user_remove_missing_args() {
    raps().args(["admin", "user", "remove"]).assert().failure();
}

#[test]
fn test_admin_no_subcommand() {
    raps()
        .args(["admin"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommand"));
}

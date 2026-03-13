//! Scenario tests for `admin operation` subcommands.
//!
//! All commands use local file-based StateManager (no APS credentials needed).
//! HOME is redirected to a tempdir for isolation so existing state files on the
//! developer's machine don't influence results.

use predicates::prelude::*;

fn raps_with_isolated_home() -> (assert_cmd::Command, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    cmd.env("HOME", tmp.path())
        .env("XDG_DATA_HOME", tmp.path().join("data"))
        .env("LOCALAPPDATA", tmp.path().join("appdata"))
        // suppress APS credential lookups
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .env_remove("APS_BASE_URL");
    (cmd, tmp)
}

// ==================== list (empty state) ====================

/// `admin operation list --output table` with no stored operations says "No operations found."
#[test]
fn test_admin_operation_list_empty_table_succeeds() {
    let (mut cmd, _tmp) = raps_with_isolated_home();
    cmd.args(["admin", "operation", "list", "--output", "table"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No operations found"));
}

/// `admin operation list --output json` returns an empty JSON array.
#[test]
fn test_admin_operation_list_empty_json() {
    let (mut cmd, _tmp) = raps_with_isolated_home();
    cmd.args(["admin", "operation", "list", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

/// `admin operation list --status completed --output json` with no ops still returns [].
#[test]
fn test_admin_operation_list_filter_status_empty() {
    let (mut cmd, _tmp) = raps_with_isolated_home();
    cmd.args([
        "admin",
        "operation",
        "list",
        "--status",
        "completed",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("[]"));
}

/// `admin operation list --limit 5 --output yaml` succeeds (exercises limit path).
#[test]
fn test_admin_operation_list_with_limit_yaml() {
    let (mut cmd, _tmp) = raps_with_isolated_home();
    cmd.args([
        "admin",
        "operation",
        "list",
        "--limit",
        "5",
        "--output",
        "yaml",
    ])
    .assert()
    .success();
}

// ==================== status (no operation) ====================

/// `admin operation status` with no operations fails with "No operations found".
#[test]
fn test_admin_operation_status_no_ops_fails() {
    let (mut cmd, _tmp) = raps_with_isolated_home();
    cmd.args(["admin", "operation", "status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No operations found"));
}

/// `admin operation status --operation-id <uuid>` with unknown ID fails gracefully.
#[test]
fn test_admin_operation_status_unknown_id_fails() {
    let (mut cmd, _tmp) = raps_with_isolated_home();
    cmd.args([
        "admin",
        "operation",
        "status",
        "--operation-id",
        "00000000-0000-0000-0000-000000000000",
    ])
    .assert()
    .failure()
    .code(predicate::ne(101));
}

// ==================== resume (no operation) ====================

/// `admin operation resume` with no resumable operation fails gracefully.
#[test]
fn test_admin_operation_resume_no_ops_fails() {
    let (mut cmd, _tmp) = raps_with_isolated_home();
    cmd.args(["admin", "operation", "resume"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No resumable operation found"));
}

/// `admin operation resume --operation-id <uuid>` with unknown ID fails gracefully.
#[test]
fn test_admin_operation_resume_unknown_id_fails() {
    let (mut cmd, _tmp) = raps_with_isolated_home();
    cmd.args([
        "admin",
        "operation",
        "resume",
        "--operation-id",
        "00000000-0000-0000-0000-000000000001",
    ])
    .assert()
    .failure()
    .code(predicate::ne(101));
}

// ==================== cancel (no operation) ====================

/// `admin operation cancel` with no active operation fails gracefully.
#[test]
fn test_admin_operation_cancel_no_ops_fails() {
    let (mut cmd, _tmp) = raps_with_isolated_home();
    cmd.args(["admin", "operation", "cancel"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No active operation found"));
}

/// `admin operation cancel --operation-id <uuid> --yes` with unknown ID fails gracefully.
#[test]
fn test_admin_operation_cancel_unknown_id_fails() {
    let (mut cmd, _tmp) = raps_with_isolated_home();
    cmd.args([
        "admin",
        "operation",
        "cancel",
        "--operation-id",
        "00000000-0000-0000-0000-000000000002",
        "--yes",
    ])
    .assert()
    .failure()
    .code(predicate::ne(101));
}

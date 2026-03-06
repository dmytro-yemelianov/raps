//! Scenario tests for `admin folder rights` command.
//!
//! The mock server has projects `proj-001` and `proj-002` but no "Project Files"
//! folder seeded for those projects. As a result, all projects are skipped
//! (not failed) when folder type is "project-files".

use crate::test_utils::start_cli_test;
use predicates::prelude::*;

/// Smoke: help output lists the level flag.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_folder_rights_help_lists_level_flag() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["admin", "folder", "rights", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--level"));
}

/// Smoke: help output shows folder flag.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_folder_rights_help_lists_folder_flag() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["admin", "folder", "rights", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--folder"));
}

/// Smoke: help output shows dry-run flag.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_folder_rights_help_lists_dry_run_flag() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["admin", "folder", "rights", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--dry-run"));
}

/// Smoke: missing email argument causes failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_folder_rights_missing_email_fails() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["admin", "folder", "rights", "--level", "view-only"])
        .assert()
        .failure();
}

/// Smoke: missing --level argument causes failure.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_folder_rights_missing_level_fails() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["admin", "folder", "rights", "alice@example.com"])
        .assert()
        .failure();
}

/// Scenario: rights with project-files folder type; projects are skipped because
/// no "Project Files" folder is seeded for proj-001/proj-002 in the mock.
/// JSON output contains operation_id.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_folder_rights_project_files_skipped_json() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token");

    cmd.args([
        "admin",
        "folder",
        "rights",
        "alice@example.com",
        "--account",
        "mock-account-001",
        "--level",
        "view-only",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("operation_id"));
}

/// Scenario: rights with custom folder id passes through directly.
/// JSON output contains total field.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_folder_rights_custom_folder_json() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token");

    cmd.args([
        "admin",
        "folder",
        "rights",
        "alice@example.com",
        "--account",
        "mock-account-001",
        "--level",
        "view-download",
        "--folder",
        "urn:adsk.wipprod:fs.folder:co.mock-top-folder-001",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("total"));
}

/// Scenario: dry-run flag sets dry_run mode; command succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_folder_rights_dry_run() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token");

    cmd.args([
        "admin",
        "folder",
        "rights",
        "alice@example.com",
        "--account",
        "mock-account-001",
        "--level",
        "folder-control",
        "--dry-run",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("total"));
}

/// Scenario: plans folder type — all projects skipped since no plans folder seeded.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_folder_rights_plans_type_json() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token");

    cmd.args([
        "admin",
        "folder",
        "rights",
        "alice@example.com",
        "--account",
        "mock-account-001",
        "--level",
        "upload-only",
        "--folder",
        "plans",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("operation_id"));
}

/// Scenario: view-download-upload level produces valid JSON output.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_folder_rights_view_download_upload_level() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token");

    cmd.args([
        "admin",
        "folder",
        "rights",
        "bob@example.com",
        "--account",
        "mock-account-001",
        "--level",
        "view-download-upload",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("operation_id"));
}

/// Scenario: view-download-upload-edit level produces valid JSON output.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_folder_rights_view_download_upload_edit_level() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token");

    cmd.args([
        "admin",
        "folder",
        "rights",
        "alice@example.com",
        "--account",
        "mock-account-001",
        "--level",
        "view-download-upload-edit",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("operation_id"));
}

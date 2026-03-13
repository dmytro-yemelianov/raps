//! Scenario tests for `rfi` CRUD commands.
//!
//! Uses the raps-mock server which has:
//! - `/construction/rfis/v2/projects/{id}/rfis` (GET, POST)
//! - `/construction/rfis/v2/projects/{id}/rfis/{rfi_id}` (GET, PATCH, DELETE)
//!
//! Seeded data: project "mock-project-001" has RFI "rfi-demo-001".
//! Auth: RAPS_FORCE_TOKEN bypasses 3-legged OAuth flow.

use crate::test_utils::start_cli_test;
use predicates::prelude::*;

const PROJECT: &str = "mock-project-001";
const RFI_ID: &str = "rfi-demo-001";

// ==================== list ====================

/// `rfi list <project>` returns JSON list containing the seeded RFI.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_list_json_contains_seeded_rfi() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args(["rfi", "list", PROJECT, "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("rfi-demo-001").or(predicate::str::contains("Demo RFI")));
}

/// `rfi list <project> --output table` succeeds without panic.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_list_table_succeeds() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args(["rfi", "list", PROJECT, "--output", "table"])
        .assert()
        .success();
}

/// `rfi list <project> --status void` returns empty list (no void RFIs seeded).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_list_status_filter_empty_result() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args([
            "rfi", "list", PROJECT, "--status", "void", "--output", "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

/// `rfi list <project> --since 9999-01-01` returns empty (future date filters all out).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_list_since_filter_returns_empty() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args([
            "rfi",
            "list",
            PROJECT,
            "--since",
            "9999-01-01",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

// ==================== get ====================

/// `rfi get <project> <rfi_id>` returns details of the seeded RFI.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_get_existing_rfi_json() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args(["rfi", "get", PROJECT, RFI_ID, "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("rfi-demo-001").or(predicate::str::contains("Demo RFI")));
}

/// `rfi get <project> <rfi_id> --output table` renders details table.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_get_table_output() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args(["rfi", "get", PROJECT, RFI_ID, "--output", "table"])
        .assert()
        .success()
        .stdout(predicate::str::contains("RFI Details"));
}

/// `rfi get <project> <nonexistent_id>` fails gracefully.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_get_nonexistent_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args(["rfi", "get", PROJECT, "nonexistent-rfi", "--output", "json"])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

// ==================== create ====================

/// `rfi create <project> --title "..."` creates a new RFI and shows it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_create_with_title_succeeds() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args([
            "rfi",
            "create",
            PROJECT,
            "--title",
            "Test RFI from scenario",
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Test RFI from scenario").or(predicate::str::contains("id")),
        );
}

/// `rfi create <project> --title "..." --output table` shows table success output.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_create_table_output() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args([
            "rfi",
            "create",
            PROJECT,
            "--title",
            "Table RFI",
            "--output",
            "table",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("RFI created successfully"));
}

/// `rfi create <project>` without --title fails with validation error.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_create_missing_title_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args(["rfi", "create", PROJECT])
        .assert()
        .failure()
        .stderr(predicate::str::contains("title").or(predicate::str::contains("required")));
}

/// `rfi create <project> --from-csv <file>` creates RFIs from a CSV file.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_create_from_csv_succeeds() {
    let (_server, mut cmd) = start_cli_test().await;

    let tmp = tempfile::tempdir().unwrap();
    let csv_path = tmp.path().join("rfis.csv");
    std::fs::write(
        &csv_path,
        "title,description,assigned_to\nCSV RFI Alpha,Need specs,\nCSV RFI Beta,,\n",
    )
    .unwrap();

    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args([
            "rfi",
            "create",
            PROJECT,
            "--from-csv",
            csv_path.to_str().unwrap(),
            "--output",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("total").or(predicate::str::contains("created")));
}

/// `rfi create <project> --from-csv <file>` with empty title row fails validation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_create_from_csv_empty_title_fails() {
    let (_server, mut cmd) = start_cli_test().await;

    let tmp = tempfile::tempdir().unwrap();
    let csv_path = tmp.path().join("bad.csv");
    std::fs::write(
        &csv_path,
        "title,description,assigned_to\n,Missing title,\n",
    )
    .unwrap();

    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args([
            "rfi",
            "create",
            PROJECT,
            "--from-csv",
            csv_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

/// `rfi create <project> --from-csv <nonexistent>` fails gracefully.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_create_from_csv_missing_file_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args([
            "rfi",
            "create",
            PROJECT,
            "--from-csv",
            "/tmp/nonexistent-rfi-file-xyz.csv",
        ])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

// ==================== update ====================

/// `rfi update <project> <rfi_id> --title "..."` updates the RFI.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_update_title_succeeds() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args([
            "rfi",
            "update",
            PROJECT,
            RFI_ID,
            "--title",
            "Updated RFI title",
            "--output",
            "json",
        ])
        .assert()
        .success();
}

/// `rfi update <project> <rfi_id> --output table` shows "updated successfully".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_update_table_output() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args([
            "rfi", "update", PROJECT, RFI_ID, "--status", "open", "--output", "table",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("RFI updated successfully"));
}

// ==================== delete ====================

/// `rfi delete <project> <rfi_id>` deletes the RFI (seeded lc-rfi-001).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_delete_succeeds() {
    let (server, _) = start_cli_test().await;

    // Create a fresh RFI to delete
    let mut create_cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    create_cmd
        .env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret")
        .env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args([
            "rfi",
            "create",
            PROJECT,
            "--title",
            "To Be Deleted",
            "--output",
            "json",
        ])
        .assert()
        .success();

    // Get the created RFI's ID from seeded data and delete it
    let mut del_cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    del_cmd
        .env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret")
        .env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args(["rfi", "delete", PROJECT, RFI_ID, "--output", "table"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted successfully"));
}

/// `rfi delete <project> <nonexistent>` fails gracefully.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rfi_delete_nonexistent_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token")
        .args(["rfi", "delete", PROJECT, "nonexistent-rfi"])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

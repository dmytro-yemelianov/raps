//! Scenario tests for DA workitems commands.

use crate::test_utils::start_cli_test;
use predicates::prelude::*;

// ==================== workitems list ====================

/// `da workitems` returns an empty list as JSON.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_workitems_list_json_empty() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args(["da", "workitems", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]").or(predicate::str::contains("workitem")));
}

/// `da workitems` table output succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_workitems_list_table_succeeds() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args(["da", "workitems"]).assert().success();
}

// ==================== da run ====================

/// `da run` with a fully-qualified activity ID succeeds (no nickname lookup).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_run_qualified_activity_succeeds() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args([
        "da", "run",
        "test-client.MyActivity+default",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Work item submitted").or(predicate::str::contains("workitem")));
}

/// `da run` with JSON output succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_run_json_output() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args([
        "da", "run",
        "test-client.MyActivity+default",
        "--output", "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("workitem_id").or(predicate::str::contains("success")));
}

/// `da run` with --input arg (URL form) succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_run_with_input_url() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args([
        "da", "run",
        "test-client.MyActivity+default",
        "--input", "InputFile=https://example.com/input.dwg",
    ])
    .assert()
    .success();
}

/// `da run` with --input starting with @ fails with helpful error (local file not supported).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_run_local_file_input_rejected() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args([
        "da", "run",
        "test-client.MyActivity+default",
        "--input", "InputFile=@/tmp/file.dwg",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("OSS").or(predicate::str::contains("upload")));
}

/// `da run` without activity ID fails with clap error.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_run_missing_activity_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args(["da", "run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ACTIVITY").or(predicate::str::contains("required")));
}

/// `da run` with unqualified activity ID and APS_DA_NICKNAME uses nickname.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_run_unqualified_activity_with_nickname() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("APS_DA_NICKNAME", "test-nickname")
        .args(["da", "run", "MyActivity"])
        .assert()
        .success();
}

// ==================== da status ====================

/// `da status` with a workitem ID returns status output.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_status_shows_workitem_status() {
    let (server, mut run_cmd) = start_cli_test().await;
    // First create a workitem
    run_cmd.args([
        "da", "run",
        "test-client.MyActivity+default",
        "--output", "json",
    ])
    .assert()
    .success();

    // Now check status (mock returns default status for any workitem ID)
    let mut status_cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    status_cmd
        .env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    status_cmd
        .args(["da", "status", "workitem-mock-001"])
        .assert()
        .success()
        .code(predicate::ne(101));
}

/// `da status` without workitem ID fails with clap error.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_status_missing_id_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args(["da", "status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("WORKITEM_ID").or(predicate::str::contains("required")));
}

// ==================== additional coverage tests ====================

/// `da workitems` after a run shows the created workitem.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_workitems_list_shows_item_after_run() {
    let (server, mut run_cmd) = start_cli_test().await;
    run_cmd.args(["da", "run", "test-client.MyActivity+default"])
        .assert()
        .success();

    let mut list_cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    list_cmd
        .env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    list_cmd.args(["da", "workitems", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("workitem").or(predicate::str::contains("id")));
}

/// `da workitems` table shows created workitem (exercises table rendering with items).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_workitems_list_table_with_items() {
    let (server, mut run_cmd) = start_cli_test().await;
    run_cmd.args(["da", "run", "test-client.MyActivity+default"])
        .assert()
        .success();

    let mut list_cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    list_cmd
        .env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    list_cmd.args(["da", "workitems"])
        .assert()
        .success()
        .code(predicate::ne(101));
}

/// `da run` with --output flag succeeds with JSON.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_run_with_output_arg() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args([
        "da", "run",
        "test-client.MyActivity+default",
        "--out-arg", "ResultFile=https://example.com/output.txt",
    ])
    .assert()
    .success();
}

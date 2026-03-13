//! Scenario tests for DA appbundles commands.
//!
//! Tests list, create, and delete app bundle operations against the mock server.
//! The DA API uses 2-legged (client credentials) auth — no RAPS_FORCE_TOKEN needed.

use crate::test_utils::start_cli_test;
use predicates::prelude::*;

/// `da appbundles` returns an empty list as JSON when no bundles exist.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_appbundles_list_json_empty() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["da", "appbundles", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

/// `da appbundles` table output succeeds (no panic even when list is empty).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_appbundles_list_table_empty_succeeds() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["da", "appbundles"]).assert().success();
}

/// `da appbundle-create` with explicit id and engine creates a bundle successfully.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_appbundle_create_with_id_and_engine() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args([
        "da",
        "appbundle-create",
        "--id",
        "MyBundle",
        "--engine",
        "Autodesk.AutoCAD+24",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("App bundle created"));
}

/// `da appbundle-create` shows the bundle ID in output.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_appbundle_create_shows_bundle_id() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args([
        "da",
        "appbundle-create",
        "--id",
        "RevitBundle",
        "--engine",
        "Autodesk.Revit+2025",
        "--description",
        "Test bundle",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("RevitBundle"));
}

/// `da appbundle-create` shows the engine in output.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_appbundle_create_shows_engine() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args([
        "da",
        "appbundle-create",
        "--id",
        "AcadBundle",
        "--engine",
        "Autodesk.AutoCAD+24",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("Autodesk.AutoCAD+24"));
}

/// After creating a bundle, `da appbundles` lists it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_appbundles_list_shows_created_bundle() {
    let (server, mut create_cmd) = start_cli_test().await;

    // Create a bundle
    create_cmd
        .args([
            "da",
            "appbundle-create",
            "--id",
            "ListedBundle",
            "--engine",
            "Autodesk.Revit+2025",
        ])
        .assert()
        .success();

    // List bundles using the same server
    let mut list_cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    list_cmd.env("APS_BASE_URL", &server.url);
    list_cmd.env("APS_CLIENT_ID", "test-client");
    list_cmd.env("APS_CLIENT_SECRET", "test-secret");
    list_cmd
        .args(["da", "appbundles", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ListedBundle"));
}

/// `da appbundle-delete` with a known bundle ID succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_appbundle_delete_succeeds() {
    let (server, mut create_cmd) = start_cli_test().await;

    // First create a bundle
    create_cmd
        .args([
            "da",
            "appbundle-create",
            "--id",
            "DeleteMe",
            "--engine",
            "Autodesk.AutoCAD+24",
        ])
        .assert()
        .success();

    // Then delete it using the same server
    let mut del_cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    del_cmd.env("APS_BASE_URL", &server.url);
    del_cmd.env("APS_CLIENT_ID", "test-client");
    del_cmd.env("APS_CLIENT_SECRET", "test-secret");
    del_cmd
        .args(["da", "appbundle-delete", "DeleteMe"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted"));
}

/// `da appbundle-delete` on a non-existent bundle still succeeds (mock returns 204).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_appbundle_delete_nonexistent_succeeds() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["da", "appbundle-delete", "NonExistentBundle"])
        .assert()
        .success();
}

/// `da appbundles` with `--output yaml` succeeds and produces YAML output.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_appbundles_list_yaml_output() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["da", "appbundles", "--output", "yaml"])
        .assert()
        .success();
}

/// `da appbundle-create` shows upload URL hint in output.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_appbundle_create_shows_upload_url() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args([
        "da",
        "appbundle-create",
        "--id",
        "UploadBundle",
        "--engine",
        "Autodesk.AutoCAD+24",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("mock-s3-upload"));
}

/// `da appbundle-create` also creates a 'default' alias.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_appbundle_create_creates_default_alias() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args([
        "da",
        "appbundle-create",
        "--id",
        "AliasBundle",
        "--engine",
        "Autodesk.Revit+2025",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("default"));
}

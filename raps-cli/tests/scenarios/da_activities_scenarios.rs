//! Scenario tests for DA activities commands.
//!
//! APS_DA_NICKNAME is set in all activity-create tests so effective_nickname()
//! returns without calling /forgeapps/me (which the mock does not serve).

use crate::test_utils::start_cli_test;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use std::io::Write;

// ==================== activities list ====================

/// `da activities` returns an empty list as JSON.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activities_list_json_empty() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args(["da", "activities", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

/// `da activities` table output succeeds with empty list.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activities_list_table_empty_succeeds() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args(["da", "activities"]).assert().success();
}

/// `da activities --output yaml` succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activities_list_yaml_output() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args(["da", "activities", "--output", "yaml"])
        .assert()
        .success();
}

/// After creating an activity, `da activities` lists it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activities_list_shows_created_activity() {
    let (server, mut create_cmd) = start_cli_test().await;
    create_cmd
        .env("APS_DA_NICKNAME", "test-nickname")
        .args([
            "da", "activity-create",
            "--id", "ListedActivity",
            "--engine", "Autodesk.AutoCAD+24",
            "--command", "$(engine.path)/accoreconsole.exe",
        ])
        .assert()
        .success();

    let mut list_cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    list_cmd
        .env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    list_cmd
        .args(["da", "activities", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ListedActivity"));
}

// ==================== activity-create (from args) ====================

/// `da activity-create` with --id, --engine, --command succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activity_create_from_args_succeeds() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("APS_DA_NICKNAME", "test-nickname")
        .args([
            "da", "activity-create",
            "--id", "MyActivity",
            "--engine", "Autodesk.Revit+2025",
            "--command", "$(engine.path)/revitcoreconsole.exe /i $(args.InputFile.path)",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("MyActivity"));
}

/// `da activity-create` output contains the engine.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activity_create_output_contains_engine() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("APS_DA_NICKNAME", "test-nickname")
        .args([
            "da", "activity-create",
            "--id", "EngineActivity",
            "--engine", "Autodesk.AutoCAD+24",
            "--command", "$(engine.path)/accoreconsole.exe",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Autodesk.AutoCAD+24"));
}

/// `da activity-create` with --description succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activity_create_with_description() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("APS_DA_NICKNAME", "test-nickname")
        .args([
            "da", "activity-create",
            "--id", "DescActivity",
            "--engine", "Autodesk.Revit+2025",
            "--command", "$(engine.path)/revitcoreconsole.exe",
            "--description", "Test activity",
        ])
        .assert()
        .success();
}

/// `da activity-create` with --appbundle qualifies the bundle name with nickname.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activity_create_with_appbundle() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("APS_DA_NICKNAME", "test-nickname")
        .args([
            "da", "activity-create",
            "--id", "BundleActivity",
            "--engine", "Autodesk.AutoCAD+24",
            "--command", "$(engine.path)/accoreconsole.exe",
            "--appbundle", "MyBundle",
        ])
        .assert()
        .success();
}

/// `da activity-create` without --id fails with clap error.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activity_create_missing_id_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args([
        "da", "activity-create",
        "--engine", "Autodesk.AutoCAD+24",
        "--command", "$(engine.path)/accoreconsole.exe",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("id").or(predicate::str::contains("required")));
}

/// `da activity-create` without --engine fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activity_create_missing_engine_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args([
        "da", "activity-create",
        "--id", "MyActivity",
        "--command", "$(engine.path)/accoreconsole.exe",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("engine").or(predicate::str::contains("required")));
}

/// `da activity-create` without --command fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activity_create_missing_command_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args([
        "da", "activity-create",
        "--id", "MyActivity",
        "--engine", "Autodesk.AutoCAD+24",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("command").or(predicate::str::contains("required")));
}

// ==================== activity-create (from file) ====================

/// `da activity-create --file` with a valid JSON file succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activity_create_from_json_file() {
    let mut f = NamedTempFile::with_suffix(".json").unwrap();
    write!(f, r#"{{
        "id": "FileActivity",
        "engine": "Autodesk.AutoCAD+24",
        "commandLine": ["$(engine.path)/accoreconsole.exe"],
        "appBundles": [],
        "parameters": {{}}
    }}"#)
    .unwrap();

    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("APS_DA_NICKNAME", "test-nickname")
        .args(["da", "activity-create", "--file", f.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("FileActivity").or(predicate::str::contains("created")));
}

/// `da activity-create --file` with missing id field fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activity_create_from_file_missing_id_fails() {
    let mut f = NamedTempFile::with_suffix(".json").unwrap();
    write!(f, r#"{{
        "id": "",
        "engine": "Autodesk.AutoCAD+24",
        "commandLine": ["$(engine.path)/accoreconsole.exe"],
        "appBundles": [],
        "parameters": {{}}
    }}"#)
    .unwrap();

    let (_server, mut cmd) = start_cli_test().await;
    cmd.args(["da", "activity-create", "--file", f.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("id"));
}

/// `da activity-create --file` with non-existent file fails gracefully.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activity_create_from_nonexistent_file_fails() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args([
        "da", "activity-create",
        "--file", "/tmp/nonexistent-activity-file-99999.json",
    ])
    .assert()
    .failure()
    .code(predicate::ne(101));
}

// ==================== activity-delete ====================

/// `da activity-delete` on a known activity succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activity_delete_succeeds() {
    let (server, mut create_cmd) = start_cli_test().await;
    create_cmd
        .env("APS_DA_NICKNAME", "test-nickname")
        .args([
            "da", "activity-create",
            "--id", "DeleteMe",
            "--engine", "Autodesk.AutoCAD+24",
            "--command", "$(engine.path)/accoreconsole.exe",
        ])
        .assert()
        .success();

    let mut del_cmd = assert_cmd::Command::cargo_bin("raps").unwrap();
    del_cmd
        .env("APS_BASE_URL", &server.url)
        .env("APS_CLIENT_ID", "test-client")
        .env("APS_CLIENT_SECRET", "test-secret");
    del_cmd
        .args(["da", "activity-delete", "DeleteMe"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted"));
}

/// `da activity-delete` on a non-existent activity succeeds (mock returns 204).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_da_activity_delete_nonexistent_succeeds() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.args(["da", "activity-delete", "NonExistentActivity"])
        .assert()
        .success();
}

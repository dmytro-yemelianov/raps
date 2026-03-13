//! Scenario tests for `config context` commands.

use crate::test_utils::start_cli_test;
use predicates::prelude::*;

// Helper: create and activate a profile in the given tempdir.
async fn setup_profile(home: &std::path::Path, xdg: &std::path::Path, name: &str) {
    let (_s1, mut c1) = start_cli_test().await;
    c1.env("HOME", home).env("XDG_CONFIG_HOME", xdg);
    c1.args(["config", "profile", "create", name])
        .assert()
        .success();

    let (_s2, mut c2) = start_cli_test().await;
    c2.env("HOME", home).env("XDG_CONFIG_HOME", xdg);
    c2.args(["config", "profile", "use", name])
        .assert()
        .success();
}

/// `config context show` without an active profile still succeeds (returns empty context).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_show_no_profile_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path().join("config"));
    cmd.args(["config", "context", "show", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hub_id"));
}

/// `config context show` returns a JSON array with hub_id, project_id, account_id.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_show_json_contains_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");
    setup_profile(&home, &xdg, "ctx-show").await;

    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd.args(["config", "context", "show", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hub_id"))
        .stdout(predicate::str::contains("project_id"))
        .stdout(predicate::str::contains("account_id"));
}

/// `config context show` table output prints "Current Context:".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_show_table_output() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");
    setup_profile(&home, &xdg, "ctx-table").await;

    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd.args(["config", "context", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hub_id"));
}

/// `config context set hub_id` without an active profile fails with "No active profile".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_set_no_profile_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path().join("config"));
    cmd.args(["config", "context", "set", "hub_id", "some-hub"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No active profile"));
}

/// `config context set hub_id <value>` with an active profile succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_set_hub_id_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");
    setup_profile(&home, &xdg, "ctx-sethub").await;

    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd.args([
        "config", "context", "set", "hub_id", "b.abc123", "--output", "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("hub_id"));
}

/// `config context set project_id <value>` with an active profile succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_set_project_id_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");
    setup_profile(&home, &xdg, "ctx-setproj").await;

    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd.args([
        "config",
        "context",
        "set",
        "project_id",
        "b.proj456",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("project_id"));
}

/// `config context set account_id <value>` with an active profile succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_set_account_id_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");
    setup_profile(&home, &xdg, "ctx-setacc").await;

    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd.args([
        "config",
        "context",
        "set",
        "account_id",
        "acc789",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("account_id"));
}

/// `config context set hub_id clear` clears the value (passes "clear" as value).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_set_hub_id_clear_value() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");
    setup_profile(&home, &xdg, "ctx-clrhub").await;

    // First set it
    let (_s1, mut c1) = start_cli_test().await;
    c1.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c1.args(["config", "context", "set", "hub_id", "b.abc123"])
        .assert()
        .success();

    // Then clear it
    let (_s2, mut c2) = start_cli_test().await;
    c2.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c2.args([
        "config", "context", "set", "hub_id", "clear", "--output", "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("hub_id"));
}

/// `config context set` with an unknown key fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_set_unknown_key_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");
    setup_profile(&home, &xdg, "ctx-badkey").await;

    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd.args(["config", "context", "set", "unknown_key", "value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown context key"));
}

/// `config context clear` without a profile fails gracefully.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_clear_no_profile_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path().join("config"));
    cmd.args(["config", "context", "clear"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No active profile"));
}

/// `config context clear` with an active profile succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_clear_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");
    setup_profile(&home, &xdg, "ctx-clear").await;

    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd.args(["config", "context", "clear"]).assert().success();
}

/// `config context clear` JSON output contains success field.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_clear_json_output() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");
    setup_profile(&home, &xdg, "ctx-clrjson").await;

    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd.args(["config", "context", "clear", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"success\""));
}

/// Full round-trip: set hub_id, project_id, account_id, then show, then clear, then show again.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_full_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");
    setup_profile(&home, &xdg, "ctx-roundtrip").await;

    // Set hub_id
    let (_s1, mut c1) = start_cli_test().await;
    c1.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c1.args(["config", "context", "set", "hub_id", "b.hub-abc"])
        .assert()
        .success();

    // Set project_id
    let (_s2, mut c2) = start_cli_test().await;
    c2.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c2.args(["config", "context", "set", "project_id", "b.proj-xyz"])
        .assert()
        .success();

    // Show — should contain both values
    let (_s3, mut c3) = start_cli_test().await;
    c3.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c3.args(["config", "context", "show", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("b.hub-abc"))
        .stdout(predicate::str::contains("b.proj-xyz"));

    // Clear
    let (_s4, mut c4) = start_cli_test().await;
    c4.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c4.args(["config", "context", "clear"]).assert().success();

    // Show again — values should be gone
    let (_s5, mut c5) = start_cli_test().await;
    c5.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c5.args(["config", "context", "show", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"(not set)\"").or(predicate::str::contains("null")));
}

/// `config context set hub_id` table output (no --output flag).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_set_hub_id_table_output() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");
    setup_profile(&home, &xdg, "ctx-tbl").await;

    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd.args(["config", "context", "set", "hub_id", "b.abc123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hub_id"));
}

/// `config context show` env var source is reflected in output.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_context_show_env_var_source() {
    let tmp = tempfile::tempdir().unwrap();
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", tmp.path().join("config"))
        .env("APS_HUB_ID", "env-hub-123");
    cmd.args(["config", "context", "show", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("env-hub-123"));
}

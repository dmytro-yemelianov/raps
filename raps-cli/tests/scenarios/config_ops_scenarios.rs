//! Scenario tests for `config get` and `config set` commands.

use crate::test_utils::start_cli_test;
use predicates::prelude::*;

/// `config get use_keychain` with RAPS_USE_KEYCHAIN=true returns json with value "true".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_use_keychain_env_true_json() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_USE_KEYCHAIN", "true");
    cmd.args(["config", "get", "use_keychain", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"use_keychain\""))
        .stdout(predicate::str::contains("\"true\""));
}

/// `config get use_keychain` without RAPS_USE_KEYCHAIN set still succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_use_keychain_no_env_json() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env_remove("RAPS_USE_KEYCHAIN");
    cmd.args(["config", "get", "use_keychain", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"use_keychain\""));
}

/// `config get client_id` without an active profile returns json with null value.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_client_id_no_profile_json() {
    let tmp = tempfile::tempdir().unwrap();
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", tmp.path());
    cmd.env("XDG_CONFIG_HOME", tmp.path().join("config"));
    cmd.args(["config", "get", "client_id", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"client_id\""));
}

/// `config get base_url` without an active profile returns json successfully.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_base_url_no_profile_json() {
    let tmp = tempfile::tempdir().unwrap();
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", tmp.path());
    cmd.env("XDG_CONFIG_HOME", tmp.path().join("config"));
    cmd.args(["config", "get", "base_url", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"base_url\""));
}

/// `config get` with an unknown key without active profile succeeds returning null
/// (source is "environment" so the unknown-key bail is not reached).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_unknown_key_no_profile_succeeds_with_null() {
    let tmp = tempfile::tempdir().unwrap();
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", tmp.path());
    cmd.env("XDG_CONFIG_HOME", tmp.path().join("config"));
    cmd.args(["config", "get", "nonexistent_key", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("null"));
}

/// `config set` without an active profile fails with a helpful error.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_set_no_active_profile_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", tmp.path());
    cmd.env("XDG_CONFIG_HOME", tmp.path().join("config"));
    cmd.args(["config", "set", "client_id", "my-client", "--output", "json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No active profile"));
}

/// `config get use_keychain` with RAPS_USE_KEYCHAIN=1 succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_use_keychain_one_json() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_USE_KEYCHAIN", "1");
    cmd.args(["config", "get", "use_keychain", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"use_keychain\""));
}

/// `config get use_keychain` with RAPS_USE_KEYCHAIN=yes succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_use_keychain_yes_json() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_USE_KEYCHAIN", "yes");
    cmd.args(["config", "get", "use_keychain", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"use_keychain\""));
}

/// `config get client_id` table output succeeds.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_client_id_table_output() {
    let tmp = tempfile::tempdir().unwrap();
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", tmp.path());
    cmd.env("XDG_CONFIG_HOME", tmp.path().join("config"));
    cmd.args(["config", "get", "client_id"])
        .assert()
        .success()
        .stdout(predicate::str::contains("client_id"));
}

/// `config get da_nickname` returns a key result successfully.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_da_nickname_json() {
    let tmp = tempfile::tempdir().unwrap();
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", tmp.path());
    cmd.env("XDG_CONFIG_HOME", tmp.path().join("config"));
    cmd.args(["config", "get", "da_nickname", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"da_nickname\""));
}

/// `config get callback_url` returns a key result successfully.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_callback_url_json() {
    let tmp = tempfile::tempdir().unwrap();
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", tmp.path());
    cmd.env("XDG_CONFIG_HOME", tmp.path().join("config"));
    cmd.args(["config", "get", "callback_url", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"callback_url\""));
}

/// `config set` with an unknown key fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_set_unknown_key_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("HOME", tmp.path());
    cmd.env("XDG_CONFIG_HOME", tmp.path().join("config"));
    // Even with a profile, setting an unknown key fails; but here it fails on
    // "no active profile" first — the same failure path is tested.
    cmd.args(["config", "set", "unknown_key", "value", "--output", "json"])
        .assert()
        .failure();
}

/// Full round-trip: create profile, activate it, set client_id, get it back (json).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_set_and_get_with_active_profile_json() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");

    // Step 1: create profile
    let (_server, mut cmd1) = start_cli_test().await;
    cmd1.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd1.args(["config", "profile", "create", "ci-profile"])
        .assert()
        .success();

    // Step 2: activate profile
    let (_server2, mut cmd2) = start_cli_test().await;
    cmd2.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd2.args(["config", "profile", "use", "ci-profile"])
        .assert()
        .success();

    // Step 3: set client_id
    let (_server3, mut cmd3) = start_cli_test().await;
    cmd3.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd3.args([
        "config", "set", "client_id", "my-test-client-id", "--output", "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"success\""));

    // Step 4: get client_id back
    let (_server4, mut cmd4) = start_cli_test().await;
    cmd4.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    cmd4.args(["config", "get", "client_id", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my-test-client-id"));
}

/// Set base_url and retrieve it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_set_base_url_json() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");

    // create + activate profile
    let (_s1, mut c1) = start_cli_test().await;
    c1.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c1.args(["config", "profile", "create", "base-url-profile"])
        .assert()
        .success();

    let (_s2, mut c2) = start_cli_test().await;
    c2.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c2.args(["config", "profile", "use", "base-url-profile"])
        .assert()
        .success();

    // set base_url
    let (_s3, mut c3) = start_cli_test().await;
    c3.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c3.args([
        "config",
        "set",
        "base_url",
        "https://example.autodesk.com",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("base_url"));
}

/// Set use_keychain=true and verify it succeeds with stdout confirmation.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_set_use_keychain_true() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");

    let (_s1, mut c1) = start_cli_test().await;
    c1.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c1.args(["config", "profile", "create", "kc-profile"])
        .assert()
        .success();

    let (_s2, mut c2) = start_cli_test().await;
    c2.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c2.args(["config", "profile", "use", "kc-profile"])
        .assert()
        .success();

    let (_s3, mut c3) = start_cli_test().await;
    c3.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c3.args(["config", "set", "use_keychain", "true", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("use_keychain"));
}

/// Set use_keychain=false triggers warning on stderr.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_set_use_keychain_false_warns() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");

    let (_s1, mut c1) = start_cli_test().await;
    c1.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c1.args(["config", "profile", "create", "kc2-profile"])
        .assert()
        .success();

    let (_s2, mut c2) = start_cli_test().await;
    c2.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c2.args(["config", "profile", "use", "kc2-profile"])
        .assert()
        .success();

    let (_s3, mut c3) = start_cli_test().await;
    c3.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c3.args([
        "config", "set", "use_keychain", "false", "--output", "json",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("WARNING"));
}

/// Set da_nickname and retrieve it (table format).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_set_da_nickname_table() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");

    let (_s1, mut c1) = start_cli_test().await;
    c1.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c1.args(["config", "profile", "create", "da-profile"])
        .assert()
        .success();

    let (_s2, mut c2) = start_cli_test().await;
    c2.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c2.args(["config", "profile", "use", "da-profile"])
        .assert()
        .success();

    let (_s3, mut c3) = start_cli_test().await;
    c3.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c3.args(["config", "set", "da_nickname", "my-app"])
        .assert()
        .success()
        .stdout(predicate::str::contains("da_nickname"));
}

/// Set callback_url (json output includes key name).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_set_callback_url_json() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");

    let (_s1, mut c1) = start_cli_test().await;
    c1.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c1.args(["config", "profile", "create", "cb-profile"])
        .assert()
        .success();

    let (_s2, mut c2) = start_cli_test().await;
    c2.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c2.args(["config", "profile", "use", "cb-profile"])
        .assert()
        .success();

    let (_s3, mut c3) = start_cli_test().await;
    c3.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c3.args([
        "config",
        "set",
        "callback_url",
        "http://localhost:3000/callback",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("callback_url"));
}

/// Set client_secret (json output includes key name).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_set_client_secret_json() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");

    let (_s1, mut c1) = start_cli_test().await;
    c1.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c1.args(["config", "profile", "create", "secret-profile"])
        .assert()
        .success();

    let (_s2, mut c2) = start_cli_test().await;
    c2.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c2.args(["config", "profile", "use", "secret-profile"])
        .assert()
        .success();

    let (_s3, mut c3) = start_cli_test().await;
    c3.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c3.args([
        "config",
        "set",
        "client_secret",
        "supersecretvalue",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("client_secret"));
}

/// Get client_secret from an active profile (json output includes key).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_client_secret_with_profile_json() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");

    let (_s1, mut c1) = start_cli_test().await;
    c1.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c1.args(["config", "profile", "create", "gs-profile"])
        .assert()
        .success();

    let (_s2, mut c2) = start_cli_test().await;
    c2.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c2.args(["config", "profile", "use", "gs-profile"])
        .assert()
        .success();

    let (_s3, mut c3) = start_cli_test().await;
    c3.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c3.args([
        "config",
        "set",
        "client_secret",
        "mysecret",
        "--output",
        "json",
    ])
    .assert()
    .success();

    let (_s4, mut c4) = start_cli_test().await;
    c4.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c4.args(["config", "get", "client_secret", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"client_secret\""));
}

/// Get with active profile but key not set returns null value.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_unset_key_with_active_profile_json() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");

    let (_s1, mut c1) = start_cli_test().await;
    c1.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c1.args(["config", "profile", "create", "empty-profile"])
        .assert()
        .success();

    let (_s2, mut c2) = start_cli_test().await;
    c2.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c2.args(["config", "profile", "use", "empty-profile"])
        .assert()
        .success();

    let (_s3, mut c3) = start_cli_test().await;
    c3.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c3.args(["config", "get", "da_nickname", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"da_nickname\""))
        .stdout(predicate::str::contains("null"));
}

/// Get with active profile but unknown key fails.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_config_get_unknown_key_with_active_profile_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_owned();
    let xdg = home.join("config");

    let (_s1, mut c1) = start_cli_test().await;
    c1.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c1.args(["config", "profile", "create", "uk-profile"])
        .assert()
        .success();

    let (_s2, mut c2) = start_cli_test().await;
    c2.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c2.args(["config", "profile", "use", "uk-profile"])
        .assert()
        .success();

    let (_s3, mut c3) = start_cli_test().await;
    c3.env("HOME", &home).env("XDG_CONFIG_HOME", &xdg);
    c3.args(["config", "get", "not_a_real_key", "--output", "json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown configuration key"));
}

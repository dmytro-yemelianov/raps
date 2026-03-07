//! Integration tests for job commands
//!
//! Tests CLI argument parsing, help output, and error handling for job commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_job_help() {
    raps()
        .args(["job", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("cancel"));
}

#[test]
fn test_job_status_help() {
    raps()
        .args(["job", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID").or(predicate::str::contains("id")))
        .stdout(predicate::str::contains("--wait"))
        .stdout(predicate::str::contains("--poll-secs"));
}

#[test]
fn test_job_list_help() {
    raps()
        .args(["job", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--state"));
}

#[test]
fn test_job_cancel_help() {
    raps()
        .args(["job", "cancel", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MACHINE_ID").or(predicate::str::contains("machine_id")));
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_job_status_requires_id() {
    raps()
        .args(["job", "status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("ID").or(predicate::str::contains("required")));
}

#[test]
fn test_job_cancel_requires_machine_id() {
    raps()
        .args(["job", "cancel"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("MACHINE_ID").or(predicate::str::contains("required")));
}

// ==================== No-credentials / Dispatch Coverage Tests ====================

#[test]
fn test_job_status_no_serverless_config_fails_gracefully() {
    raps()
        .env_remove("FLY_APP")
        .env_remove("FLY_API_TOKEN")
        .args(["job", "status", "machine-abc123"])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

#[test]
fn test_job_list_no_serverless_config_fails_gracefully() {
    raps()
        .env_remove("FLY_APP")
        .env_remove("FLY_API_TOKEN")
        .args(["job", "list"])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

#[test]
fn test_job_cancel_no_serverless_config_fails_gracefully() {
    raps()
        .env_remove("FLY_APP")
        .env_remove("FLY_API_TOKEN")
        .args(["job", "cancel", "machine-abc123"])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

#[test]
fn test_job_list_state_filter_accepted() {
    raps()
        .env_remove("FLY_APP")
        .env_remove("FLY_API_TOKEN")
        .args(["job", "list", "--state", "started"])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

// ==================== Fake-credentials coverage (exercises handler entry past from_config) ====================

#[test]
fn test_job_status_with_fake_creds_reaches_api_call() {
    // from_config() succeeds with non-empty fly_app + fly_token;
    // handler then fails at the HTTP call (unreachable host) — not a panic.
    raps()
        .env("FLY_APP", "test-app")
        .env("FLY_API_TOKEN", "fake-token")
        .args(["job", "status", "machine-abc123"])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

#[test]
fn test_job_list_with_fake_creds_reaches_api_call() {
    raps()
        .env("FLY_APP", "test-app")
        .env("FLY_API_TOKEN", "fake-token")
        .args(["job", "list"])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

#[test]
fn test_job_list_with_state_filter_and_fake_creds() {
    raps()
        .env("FLY_APP", "test-app")
        .env("FLY_API_TOKEN", "fake-token")
        .args(["job", "list", "--state", "started"])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

#[test]
fn test_job_cancel_with_fake_creds_reaches_api_call() {
    raps()
        .env("FLY_APP", "test-app")
        .env("FLY_API_TOKEN", "fake-token")
        .args(["job", "cancel", "machine-abc123"])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

#[test]
fn test_job_status_output_json_with_fake_creds() {
    raps()
        .env("FLY_APP", "test-app")
        .env("FLY_API_TOKEN", "fake-token")
        .args(["job", "status", "machine-abc123", "--output", "json"])
        .assert()
        .failure()
        .code(predicate::ne(101));
}

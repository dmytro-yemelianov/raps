//! Integration tests for webhook commands
//!
//! Tests CLI argument parsing, help output, and error handling for webhook commands.

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_webhook_help() {
    raps()
        .args(["webhook", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("webhook"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("events"));
}

#[test]
fn test_webhook_list_help() {
    raps()
        .args(["webhook", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List all webhooks"));
}

#[test]
fn test_webhook_create_help() {
    raps()
        .args(["webhook", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Create a new webhook"))
        .stdout(predicate::str::contains("--url"))
        .stdout(predicate::str::contains("--event"));
}

#[test]
fn test_webhook_delete_help() {
    raps()
        .args(["webhook", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Delete a webhook"))
        .stdout(predicate::str::contains("HOOK_ID"))
        .stdout(predicate::str::contains("--event"));
}

#[test]
fn test_webhook_events_help() {
    raps()
        .args(["webhook", "events", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List available webhook events"));
}

#[test]
fn test_webhook_test_help() {
    raps()
        .args(["webhook", "test", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Test webhook"))
        .stdout(predicate::str::contains("URL"));
}


// ==================== Argument Validation Tests ====================

#[test]
fn test_webhook_delete_requires_hook_id() {
    raps()
        .args(["webhook", "delete"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("HOOK_ID").or(predicate::str::contains("required")));
}

#[test]
fn test_webhook_delete_requires_event() {
    raps()
        .args(["webhook", "delete", "hook-123"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--event").or(predicate::str::contains("required")));
}

#[test]
fn test_webhook_test_requires_url() {
    raps()
        .args(["webhook", "test"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("URL").or(predicate::str::contains("required")));
}

// ==================== Events List (No credentials needed) ====================

#[test]
fn test_webhook_events_works_without_credentials() {
    // Events command lists static data, doesn't need credentials
    raps()
        .args(["webhook", "events"])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .success()
        .stdout(predicate::str::contains("dm.").or(predicate::str::contains("data")));
}

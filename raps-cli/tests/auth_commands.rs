//! Integration tests for auth commands
//!
//! Tests CLI argument parsing, help output, and error handling for auth commands.

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_auth_help() {
    raps()
        .args(["auth", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Authentication"))
        .stdout(predicate::str::contains("test"))
        .stdout(predicate::str::contains("login"))
        .stdout(predicate::str::contains("logout"))
        .stdout(predicate::str::contains("status"));
}

#[test]
fn test_auth_test_help() {
    raps()
        .args(["auth", "test", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Test 2-legged"));
}

#[test]
fn test_auth_login_help() {
    raps()
        .args(["auth", "login", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Login with 3-legged"))
        .stdout(predicate::str::contains("--default"))
        .stdout(predicate::str::contains("--device"))
        .stdout(predicate::str::contains("--token"));
}

#[test]
fn test_auth_logout_help() {
    raps()
        .args(["auth", "logout", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Logout"));
}

#[test]
fn test_auth_status_help() {
    raps()
        .args(["auth", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status"));
}

#[test]
fn test_auth_whoami_help() {
    raps()
        .args(["auth", "whoami", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("user profile"));
}

#[test]
fn test_auth_inspect_help() {
    raps()
        .args(["auth", "inspect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("token"))
        .stdout(predicate::str::contains("--warn-expiry-seconds"));
}

// ==================== Logout Without Stored Token ====================

#[test]
fn test_auth_logout_no_token() {
    // Logout should succeed even without a stored token
    raps()
        .args(["auth", "logout"])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .success();
}

// ==================== Status Command ====================

#[test]
fn test_auth_status_shows_three_legged_status() {
    // Status command shows 3-legged auth status
    raps()
        .args(["auth", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("three_legged").or(predicate::str::contains("logged_in")));
}

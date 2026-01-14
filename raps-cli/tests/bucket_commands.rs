//! Integration tests for bucket commands
//!
//! Tests CLI argument parsing, help output, and error handling for bucket commands.

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_bucket_help() {
    raps()
        .args(["bucket", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bucket"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("info"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn test_bucket_create_help() {
    raps()
        .args(["bucket", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Create"))
        .stdout(predicate::str::contains("--key"))
        .stdout(predicate::str::contains("--policy"))
        .stdout(predicate::str::contains("--region"));
}

#[test]
fn test_bucket_list_help() {
    raps()
        .args(["bucket", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List"));
}

#[test]
fn test_bucket_info_help() {
    raps()
        .args(["bucket", "info", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bucket"))
        .stdout(predicate::str::contains("BUCKET_KEY"));
}

#[test]
fn test_bucket_delete_help() {
    raps()
        .args(["bucket", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Delete"))
        .stdout(predicate::str::contains("--yes"));
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_bucket_info_requires_bucket_key() {
    raps()
        .args(["bucket", "info"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("BUCKET_KEY").or(predicate::str::contains("required")));
}

#[test]
fn test_bucket_create_invalid_policy() {
    // Invalid policy should cause an error
    raps()
        .args([
            "bucket",
            "create",
            "--key",
            "test",
            "--policy",
            "invalid-policy-xyz",
        ])
        .assert()
        .failure();
}

#[test]
fn test_bucket_create_invalid_region() {
    // Invalid region should cause an error
    raps()
        .args([
            "bucket",
            "create",
            "--key",
            "test",
            "--region",
            "invalid-region-xyz",
        ])
        .assert()
        .failure();
}

// ==================== Output Format Flag Tests ====================

#[test]
fn test_bucket_output_format_json_accepted() {
    // Verify --output json flag is accepted (command may succeed or fail based on credentials)
    let result = raps()
        .args(["bucket", "list", "--output", "json"])
        .output()
        .unwrap();

    // Either succeeds with JSON output or fails gracefully
    if result.status.success() {
        let stdout = String::from_utf8_lossy(&result.stdout);
        assert!(stdout.contains("[") || stdout.contains("{"));
    }
}

#[test]
fn test_bucket_output_format_yaml_accepted() {
    // Verify --output yaml flag is accepted
    let result = raps()
        .args(["bucket", "list", "--output", "yaml"])
        .output()
        .unwrap();

    // Command is valid regardless of credentials
    // Just check the flag doesn't cause a parse error
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(!stderr.contains("error: invalid value 'yaml'"));
}

#[test]
fn test_bucket_output_format_table_accepted() {
    // Verify --output table flag is accepted
    let result = raps()
        .args(["bucket", "list", "--output", "table"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(!stderr.contains("error: invalid value 'table'"));
}

#[test]
fn test_bucket_output_format_invalid() {
    // Invalid output format should be rejected
    raps()
        .args(["bucket", "list", "--output", "invalid-format"])
        .assert()
        .failure();
}

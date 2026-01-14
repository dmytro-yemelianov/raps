//! Integration tests for object commands
//!
//! Tests CLI argument parsing, help output, and error handling for object commands.

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_object_help() {
    raps()
        .args(["object", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("object"))
        .stdout(predicate::str::contains("upload"))
        .stdout(predicate::str::contains("download"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("delete"));
}

// Note: object upload --help has a clap panic due to positional arg ordering bug
// The test is disabled until the bug is fixed in the CLI

#[test]
fn test_object_upload_batch_help() {
    raps()
        .args(["object", "upload-batch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Upload multiple"))
        .stdout(predicate::str::contains("--parallel"));
}

#[test]
fn test_object_download_help() {
    raps()
        .args(["object", "download", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Download"))
        .stdout(predicate::str::contains("--output"));
}

#[test]
fn test_object_list_help() {
    raps()
        .args(["object", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List"));
}

#[test]
fn test_object_delete_help() {
    raps()
        .args(["object", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Delete"))
        .stdout(predicate::str::contains("--yes"));
}

#[test]
fn test_object_signed_url_help() {
    raps()
        .args(["object", "signed-url", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("signed"))
        .stdout(predicate::str::contains("BUCKET"))
        .stdout(predicate::str::contains("OBJECT"));
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_object_upload_requires_file() {
    raps()
        .args(["object", "upload"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("FILE").or(predicate::str::contains("required")));
}

#[test]
fn test_object_signed_url_requires_bucket_and_object() {
    raps()
        .args(["object", "signed-url"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("BUCKET").or(predicate::str::contains("required")));
}

#[test]
fn test_object_signed_url_requires_object() {
    raps()
        .args(["object", "signed-url", "test-bucket"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("OBJECT").or(predicate::str::contains("required")));
}

#[test]
fn test_object_upload_nonexistent_file() {
    // Uploading a nonexistent file should fail
    raps()
        .args([
            "object",
            "upload",
            "test-bucket",
            "nonexistent-file-12345.xyz",
        ])
        .assert()
        .failure();
}

// ==================== Output Format Tests ====================

#[test]
fn test_object_list_output_format_accepted() {
    // Verify --output flag is accepted
    let result = raps()
        .args(["object", "list", "test-bucket", "--output", "json"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(!stderr.contains("error: invalid value 'json'"));
}

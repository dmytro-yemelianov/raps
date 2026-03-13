// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Unit tests for the `raps sync` command.
//!
//! Tests cover CLI argument parsing (flags accepted / rejected by clap),
//! the dry-run path with an empty source directory, and the error path
//! when the local directory does not exist.

use assert_cmd::Command;
use predicates::prelude::*;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ---------------------------------------------------------------------------
// Help / flag acceptance
// ---------------------------------------------------------------------------

#[test]
fn test_sync_help_exits_zero() {
    raps().args(["sync", "--help"]).assert().success();
}

#[test]
fn test_sync_help_lists_flags() {
    raps()
        .args(["sync", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--delete"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--checksum"));
}

#[test]
fn test_sync_delete_flag_is_accepted_by_clap() {
    // Passing --delete with a nonexistent dir fails at runtime, not at argument
    // parsing — so the error message must NOT say "unexpected argument".
    let out = raps()
        .args([
            "sync",
            "/nonexistent-dir-raps-test",
            "my-bucket",
            "--delete",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "clap rejected --delete: {stderr}"
    );
    assert!(
        !stderr.contains("error: unrecognized"),
        "clap rejected --delete: {stderr}"
    );
}

#[test]
fn test_sync_dry_run_flag_is_accepted_by_clap() {
    let out = raps()
        .args([
            "sync",
            "/nonexistent-dir-raps-test",
            "my-bucket",
            "--dry-run",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "clap rejected --dry-run: {stderr}"
    );
}

#[test]
fn test_sync_checksum_flag_is_accepted_by_clap() {
    let out = raps()
        .args([
            "sync",
            "/nonexistent-dir-raps-test",
            "my-bucket",
            "--checksum",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "clap rejected --checksum: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Runtime error: local directory does not exist
// ---------------------------------------------------------------------------

#[test]
fn test_sync_error_when_local_dir_missing() {
    raps()
        .args(["sync", "/nonexistent-dir-raps-test-xyz", "my-bucket"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found")
                .or(predicate::str::contains("No such file"))
                .or(predicate::str::contains("nonexistent")),
        );
}

// ---------------------------------------------------------------------------
// Dry-run with an empty local directory → "nothing to do" output
// ---------------------------------------------------------------------------

#[test]
fn test_sync_dry_run_empty_dir_prints_plan() {
    let dir = tempfile::tempdir().unwrap();

    // The command will attempt to list remote objects, which requires auth and
    // a real API.  However, we only care that:
    //   1. The dir-exists check passes (no "not found" error before the plan).
    //   2. The --dry-run flag is accepted without a clap error.
    //
    // If the API call fails (no credentials in CI), the error comes *after*
    // the local-dir validation, so we just verify there is no clap parse error.
    let out = raps()
        .args([
            "sync",
            dir.path().to_str().unwrap(),
            "my-bucket",
            "--dry-run",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "clap rejected --dry-run: {stderr}"
    );
    // The local-dir check must have passed (no "not found" error for the dir).
    assert!(
        !stderr.contains("Local directory not found"),
        "dry-run failed at dir check: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Argument count validation
// ---------------------------------------------------------------------------

#[test]
fn test_sync_requires_local_dir_and_bucket() {
    // Missing both positional args → clap error
    raps()
        .args(["sync"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("LOCAL_DIR")));
}

#[test]
fn test_sync_requires_bucket() {
    // Only one positional arg → clap error
    raps()
        .args(["sync", "/tmp"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("BUCKET")));
}

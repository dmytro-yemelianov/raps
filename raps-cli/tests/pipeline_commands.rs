//! Integration tests for pipeline commands
//!
//! Tests CLI argument parsing, help output, and error handling for pipeline commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_pipeline_help() {
    raps()
        .args(["pipeline", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("run"))
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("sample"))
        .stdout(predicate::str::contains("create"));
}

#[test]
fn test_pipeline_run_help() {
    raps()
        .args(["pipeline", "run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run a pipeline"))
        .stdout(predicate::str::contains("--ignore-failure"))
        .stdout(predicate::str::contains("--dry-run"));
}

#[test]
fn test_pipeline_validate_help() {
    raps()
        .args(["pipeline", "validate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Validate"));
}

#[test]
fn test_pipeline_sample_help() {
    raps()
        .args(["pipeline", "sample", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("sample"))
        .stdout(predicate::str::contains("--out-file"));
}

#[test]
fn test_pipeline_create_help() {
    raps()
        .args(["pipeline", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Create"))
        .stdout(predicate::str::contains("--source"))
        .stdout(predicate::str::contains("--cron"))
        .stdout(predicate::str::contains("--action"))
        .stdout(predicate::str::contains("--notify"))
        .stdout(predicate::str::contains("--serverless"));
}

// ==================== Error Handling Tests ====================

#[test]
fn test_pipeline_run_missing_file() {
    raps()
        .args(["pipeline", "run", "/nonexistent/pipeline.yaml"])
        .assert()
        .failure();
}

#[test]
fn test_pipeline_validate_missing_file() {
    raps()
        .args(["pipeline", "validate", "/nonexistent/pipeline.yaml"])
        .assert()
        .failure();
}

#[test]
fn test_pipeline_run_no_args() {
    raps()
        .args(["pipeline", "run"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("FILE"));
}

#[test]
fn test_pipeline_create_no_name() {
    raps()
        .args(["pipeline", "create"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("NAME"));
}

#[test]
fn test_pipeline_run_dry_run_with_tempfile() {
    let dir = tempfile::tempdir().unwrap();
    let pipeline_file = dir.path().join("test.yaml");
    std::fs::write(
        &pipeline_file,
        "name: test\nsteps:\n  - name: echo\n    command: echo hello\n",
    )
    .unwrap();

    raps()
        .args([
            "pipeline",
            "run",
            pipeline_file.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success();
}

//! Integration tests for demo commands
//!
//! Tests CLI argument parsing, help output, and error handling for demo commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_demo_help() {
    raps()
        .args(["demo", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("bucket-lifecycle"))
        .stdout(predicate::str::contains("model-pipeline"))
        .stdout(predicate::str::contains("data-management"))
        .stdout(predicate::str::contains("batch-processing"));
}

#[test]
fn test_demo_bucket_lifecycle_help() {
    raps()
        .args(["demo", "bucket-lifecycle", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--prefix"))
        .stdout(predicate::str::contains("--skip-cleanup"));
}

#[test]
fn test_demo_model_pipeline_help() {
    raps()
        .args(["demo", "model-pipeline", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--file"))
        .stdout(predicate::str::contains("--format"));
}

#[test]
fn test_demo_batch_processing_help() {
    raps()
        .args(["demo", "batch-processing", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--input"))
        .stdout(predicate::str::contains("--skip-cleanup"));
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_demo_no_subcommand() {
    raps()
        .arg("demo")
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommand"));
}

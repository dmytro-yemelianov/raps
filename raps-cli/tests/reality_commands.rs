//! Integration tests for reality capture commands
//!
//! Tests CLI argument parsing, help output, and error handling for reality commands.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_reality_help() {
    raps()
        .args(["reality", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("upload"))
        .stdout(predicate::str::contains("process"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("result"));
}

#[test]
fn test_reality_create_help() {
    raps()
        .args(["reality", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--name"))
        .stdout(predicate::str::contains("--scene-type"))
        .stdout(predicate::str::contains("--format"));
}

#[test]
fn test_reality_upload_help() {
    raps()
        .args(["reality", "upload", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PHOTOSCENE_ID"))
        .stdout(predicate::str::contains("PHOTOS"));
}

#[test]
fn test_reality_process_help() {
    raps()
        .args(["reality", "process", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PHOTOSCENE_ID"));
}

#[test]
fn test_reality_status_help() {
    raps()
        .args(["reality", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PHOTOSCENE_ID"))
        .stdout(predicate::str::contains("--wait"));
}

#[test]
fn test_reality_result_help() {
    raps()
        .args(["reality", "result", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PHOTOSCENE_ID"))
        .stdout(predicate::str::contains("--format"));
}

// ==================== Missing Args Tests ====================

#[test]
fn test_reality_upload_missing_photoscene_id() {
    raps()
        .args(["reality", "upload"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("PHOTOSCENE_ID"));
}

#[test]
fn test_reality_process_missing_photoscene_id() {
    raps()
        .args(["reality", "process"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("PHOTOSCENE_ID"));
}

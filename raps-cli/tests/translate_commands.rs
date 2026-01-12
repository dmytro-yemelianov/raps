//! Integration tests for translate commands
//!
//! Tests CLI argument parsing, help output, and error handling for translate commands.

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Help Output Tests ====================

#[test]
fn test_translate_help() {
    raps()
        .args(["translate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Translate"))
        .stdout(predicate::str::contains("start"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("manifest"))
        .stdout(predicate::str::contains("derivatives"))
        .stdout(predicate::str::contains("download"));
}

#[test]
fn test_translate_start_help() {
    raps()
        .args(["translate", "start", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Start a translation"))
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("--root-filename"));
}

#[test]
fn test_translate_status_help() {
    raps()
        .args(["translate", "status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("URN"))
        .stdout(predicate::str::contains("--wait"));
}

#[test]
fn test_translate_manifest_help() {
    raps()
        .args(["translate", "manifest", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("manifest"))
        .stdout(predicate::str::contains("URN"));
}

#[test]
fn test_translate_derivatives_help() {
    raps()
        .args(["translate", "derivatives", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("derivatives"))
        .stdout(predicate::str::contains("--format"));
}

#[test]
fn test_translate_download_help() {
    raps()
        .args(["translate", "download", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Download"))
        .stdout(predicate::str::contains("--format"))
        .stdout(predicate::str::contains("--guid"))
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--all"));
}

#[test]
fn test_translate_preset_help() {
    raps()
        .args(["translate", "preset", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("preset"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("show"));
}

// ==================== Argument Validation Tests ====================

#[test]
fn test_translate_status_requires_urn() {
    raps()
        .args(["translate", "status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("URN").or(predicate::str::contains("required")));
}

#[test]
fn test_translate_manifest_requires_urn() {
    raps()
        .args(["translate", "manifest"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("URN").or(predicate::str::contains("required")));
}

#[test]
fn test_translate_download_requires_urn() {
    raps()
        .args(["translate", "download"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("URN").or(predicate::str::contains("required")));
}

#[test]
fn test_translate_derivatives_requires_urn() {
    raps()
        .args(["translate", "derivatives"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("URN").or(predicate::str::contains("required")));
}

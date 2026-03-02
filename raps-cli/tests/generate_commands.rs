//! Integration tests for generate command
//!
//! Tests CLI argument parsing, help output, and file generation.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

#[test]
fn test_generate_help() {
    raps()
        .args(["generate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("generate"))
        .stdout(predicate::str::contains("files"));
}

#[test]
fn test_generate_files_help() {
    raps()
        .args(["generate", "files", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--count"))
        .stdout(predicate::str::contains("--out-dir"));
}

#[test]
fn test_generate_files_creates_output() {
    let tmp = tempfile::tempdir().unwrap();
    raps()
        .args([
            "generate",
            "files",
            "--count",
            "1",
            "--out-dir",
            tmp.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    // Should have created at least one file
    let entries: Vec<_> = std::fs::read_dir(tmp.path()).unwrap().collect();
    assert!(
        !entries.is_empty(),
        "Expected generated files in output dir"
    );
}

#[test]
fn test_generate_files_zero_count() {
    let tmp = tempfile::tempdir().unwrap();
    raps()
        .args([
            "generate",
            "files",
            "--count",
            "0",
            "--out-dir",
            tmp.path().to_str().unwrap(),
        ])
        .assert()
        .success();
}

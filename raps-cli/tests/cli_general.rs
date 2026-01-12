//! General CLI integration tests
//!
//! Tests for the main raps binary, version, help, and top-level commands.

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a command instance for the raps binary
fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ==================== Version and Help ====================

#[test]
fn test_version() {
    raps()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("raps"));
}

#[test]
fn test_help() {
    raps()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("RAPS"))
        .stdout(predicate::str::contains("bucket"))
        .stdout(predicate::str::contains("object"))
        .stdout(predicate::str::contains("auth"))
        .stdout(predicate::str::contains("translate"))
        .stdout(predicate::str::contains("config"));
}

#[test]
fn test_help_short() {
    raps()
        .arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains("RAPS"));
}

// ==================== Top Level Commands ====================

#[test]
fn test_unknown_command() {
    raps()
        .arg("unknown-command")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

// ==================== Global Options ====================

#[test]
fn test_output_format_flag() {
    raps()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--output"));
}

#[test]
fn test_verbose_flag() {
    raps()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--verbose").or(predicate::str::contains("-v")));
}

#[test]
fn test_quiet_flag() {
    raps()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--quiet").or(predicate::str::contains("-q")));
}

#[test]
fn test_debug_flag() {
    raps()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--debug"));
}

#[test]
fn test_timeout_flag() {
    raps()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--timeout"));
}

#[test]
fn test_concurrency_flag() {
    raps()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--concurrency"));
}

// ==================== Shell Completions ====================
// Note: completions command has a clap panic bug with positional args
// These tests are disabled until the bug is fixed

#[test]
fn test_completions_help() {
    raps()
        .args(["completions", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("shell completions"))
        .stdout(predicate::str::contains("bash"))
        .stdout(predicate::str::contains("zsh"))
        .stdout(predicate::str::contains("fish"))
        .stdout(predicate::str::contains("powershell"));
}

// ==================== Generate Commands ====================

#[test]
fn test_generate_help() {
    raps()
        .args(["generate", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generate"))
        .stdout(predicate::str::contains("files"));
}

#[test]
fn test_generate_files_help() {
    raps()
        .args(["generate", "files", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generate synthetic"));
}

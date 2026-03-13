// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Tests for `raps logs` sub-commands (path, show, clear, follow).
//!
//! These are pure CLI-invocation tests; they do not require network access.

use assert_cmd::Command;
use predicates::prelude::*;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ---------------------------------------------------------------------------
// raps logs --help
// ---------------------------------------------------------------------------

#[test]
fn test_logs_help_exits_zero() {
    raps().args(["logs", "--help"]).assert().success();
}

#[test]
fn test_logs_help_lists_subcommands() {
    raps().args(["logs", "--help"]).assert().success().stdout(
        predicate::str::contains("show")
            .and(predicate::str::contains("path"))
            .and(predicate::str::contains("clear")),
    );
}

// ---------------------------------------------------------------------------
// raps logs path — prints a non-empty path
// ---------------------------------------------------------------------------

#[test]
fn test_logs_path_exits_zero() {
    raps().args(["logs", "path"]).assert().success();
}

#[test]
fn test_logs_path_prints_nonempty_line() {
    let out = raps().args(["logs", "path"]).output().unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    // The path line must be non-empty (it's a filesystem path, not just a newline).
    assert!(
        stdout.trim().len() > 1,
        "logs path produced an empty or trivial path: {stdout:?}"
    );
}

#[test]
fn test_logs_path_json_output_contains_key() {
    let out = raps()
        .args(["logs", "path", "--output", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("log_directory"),
        "JSON output missing 'log_directory' key: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// raps logs show --lines 0 — prints nothing (no log lines)
// ---------------------------------------------------------------------------

#[test]
fn test_logs_show_help_accepted() {
    raps()
        .args(["logs", "show", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--lines").or(predicate::str::contains("-n")));
}

#[test]
fn test_logs_show_lines_zero_prints_nothing_or_no_log_found() {
    // With --lines 0 either:
    //   a) a log file exists → stdout is empty (0 tail lines), or
    //   b) no log file exists yet → command fails with "No log files found".
    // Either outcome is acceptable for this test.
    let out = raps()
        .args(["logs", "show", "--lines", "0"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    if out.status.success() {
        // If it succeeded, stdout must be empty (0 lines requested).
        assert!(
            stdout.trim().is_empty(),
            "Expected empty stdout for --lines 0, got: {stdout:?}"
        );
    } else {
        // Acceptable failure: no log file found.
        assert!(
            stderr.contains("No log files") || stderr.contains("log"),
            "Unexpected error for --lines 0: {stderr}"
        );
    }
}

// ---------------------------------------------------------------------------
// raps logs clear --yes — no log files → succeeds silently
// ---------------------------------------------------------------------------

#[test]
fn test_logs_clear_yes_flag_is_accepted() {
    // --yes is a valid flag; clap must not reject it.
    let out = raps().args(["logs", "clear", "--yes"]).output().unwrap();

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("unrecognized"),
        "clap rejected --yes: {stderr}"
    );
}

#[test]
fn test_logs_clear_yes_exits_zero() {
    // Whether or not log files exist, --yes must exit 0.
    raps().args(["logs", "clear", "--yes"]).assert().success();
}

#[test]
fn test_logs_clear_yes_does_not_prompt() {
    // With --yes the command must complete without reading stdin.
    // We pipe empty stdin and expect success (no "Aborted" on stdout).
    let out = raps()
        .args(["logs", "clear", "--yes"])
        .write_stdin("")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("Aborted"),
        "logs clear --yes printed 'Aborted': {stdout}"
    );
    assert!(out.status.success(), "logs clear --yes exited non-zero");
}

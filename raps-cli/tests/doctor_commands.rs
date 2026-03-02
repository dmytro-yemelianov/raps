//! Integration tests for doctor command
//!
//! Tests CLI argument parsing and help output for diagnostic checks.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

#[test]
fn test_doctor_help() {
    raps()
        .args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("diagnostic"));
}

#[test]
fn test_doctor_no_subcommand() {
    // doctor runs checks — may fail without credentials but should not panic
    raps().arg("doctor").assert().code(predicate::ne(101));
}

#[test]
fn test_doctor_output_json() {
    raps()
        .args(["doctor", "--output", "json"])
        .assert()
        .code(predicate::ne(101));
}

#[test]
fn test_doctor_output_yaml() {
    raps()
        .args(["doctor", "--output", "yaml"])
        .assert()
        .code(predicate::ne(101));
}

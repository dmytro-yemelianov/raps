//! Integration tests for schema command
//!
//! Tests CLI argument parsing, help output, and schema generation.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

#[test]
fn test_schema_help() {
    raps()
        .args(["schema", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("schema"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("generate"))
        .stdout(predicate::str::contains("all"));
}

#[test]
fn test_schema_list() {
    raps().args(["schema", "list"]).assert().success();
}

#[test]
fn test_schema_all() {
    raps()
        .args(["schema", "all"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{"));
}

#[test]
fn test_schema_generate_valid() {
    raps()
        .args(["schema", "generate", "bucket.list"])
        .assert()
        .success();
}

#[test]
fn test_schema_generate_unknown() {
    raps()
        .args(["schema", "generate", "nonexistent_type_xyz"])
        .assert()
        .failure();
}

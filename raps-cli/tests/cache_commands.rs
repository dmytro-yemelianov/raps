//! Integration tests for cache command
//!
//! Tests CLI argument parsing, help output, and cache operations.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

#[test]
fn test_cache_help() {
    raps()
        .args(["cache", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cache"))
        .stdout(predicate::str::contains("stats"))
        .stdout(predicate::str::contains("clear"))
        .stdout(predicate::str::contains("dir"))
        .stdout(predicate::str::contains("prune"));
}

#[test]
fn test_cache_stats() {
    raps().args(["cache", "stats"]).assert().success();
}

#[test]
fn test_cache_dir() {
    raps().args(["cache", "dir"]).assert().success();
}

#[test]
fn test_cache_clear() {
    raps().args(["cache", "clear", "--yes"]).assert().success();
}

#[test]
fn test_cache_prune_help() {
    raps()
        .args(["cache", "prune", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("prune"));
}

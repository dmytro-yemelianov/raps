//! Integration tests for plugin command
//!
//! Tests CLI argument parsing, help output, and plugin management.

#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

#[test]
fn test_plugin_help() {
    raps()
        .args(["plugin", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("plugin"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("enable"))
        .stdout(predicate::str::contains("disable"))
        .stdout(predicate::str::contains("info"));
}

#[test]
fn test_plugin_list() {
    raps().args(["plugin", "list"]).assert().success();
}

#[test]
fn test_plugin_enable_missing_name() {
    raps().args(["plugin", "enable"]).assert().failure();
}

#[test]
fn test_plugin_disable_missing_name() {
    raps().args(["plugin", "disable"]).assert().failure();
}

#[test]
fn test_plugin_info_missing_name() {
    raps().args(["plugin", "info"]).assert().failure();
}

#[test]
fn test_plugin_alias_list() {
    raps().args(["plugin", "alias", "list"]).assert().success();
}

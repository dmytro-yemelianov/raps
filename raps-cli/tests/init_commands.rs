#![allow(deprecated)]
//! CLI help and smoke tests for `raps init`.

use assert_cmd::Command;
use predicates::prelude::*;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

#[test]
fn test_init_help() {
    raps()
        .args(["init", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wizard"));
}

#[test]
fn test_init_non_interactive_exits_gracefully() {
    raps()
        .args(["init", "--non-interactive"])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .code(predicate::ne(101));
}

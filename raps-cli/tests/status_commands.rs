//! CLI help and smoke tests for `raps status`.

use assert_cmd::Command;
use predicates::prelude::*;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

#[test]
fn test_status_help() {
    raps()
        .args(["status", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("status").or(predicate::str::contains("context")));
}

#[test]
fn test_status_no_credentials_does_not_panic() {
    raps()
        .args(["status", "--non-interactive"])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .code(predicate::ne(101_i32));
}

#[test]
fn test_status_output_json_flag_accepted() {
    raps()
        .args(["status", "--output", "json", "--non-interactive"])
        .env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .assert()
        .code(predicate::ne(101_i32));
}

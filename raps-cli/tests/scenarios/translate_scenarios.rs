//! Scenario: translate commands via mock server.

use crate::test_utils::start_cli_test;
use predicates::prelude::*;

const MOCK_URN: &str = "dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6bW9jay1idWNrZXQvdGVzdC5ydnQ";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_translate_start_does_not_panic() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["translate", "start", MOCK_URN, "--output", "json"])
        .assert()
        .code(predicate::ne(101));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_translate_status_does_not_panic() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["translate", "status", MOCK_URN, "--output", "json"])
        .assert()
        .code(predicate::ne(101));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_translate_manifest_does_not_panic() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["translate", "manifest", MOCK_URN, "--output", "json"])
        .assert()
        .code(predicate::ne(101));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_translate_start_missing_urn_exits_with_usage_error() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["translate", "start", "--non-interactive"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("urn").or(predicate::str::contains("required")));
}

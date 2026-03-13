//! Scenario: webhook commands via mock server.

use crate::test_utils::start_cli_test;
use predicates::prelude::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_webhook_list_returns_json_from_mock() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["webhook", "list", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_webhook_create_missing_url_exits_with_usage_error() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args([
        "webhook",
        "create",
        "--event",
        "dm.version.added",
        "--non-interactive",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("url").or(predicate::str::contains("required")));
}

//! Scenario: `raps status` renders a JSON context dashboard via mock server.

use crate::test_utils::start_cli_test;
use predicates::prelude::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_status_json_shows_auth_and_hubs() {
    let (_server, mut cmd) = start_cli_test().await;

    cmd.args(["status", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("two_legged"))
        .stdout(predicate::str::contains("hubs"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_status_no_credentials_does_not_panic() {
    let (server, mut cmd) = start_cli_test().await;
    cmd.env_remove("APS_CLIENT_ID")
        .env_remove("APS_CLIENT_SECRET")
        .env("APS_BASE_URL", &server.url);

    cmd.args(["status", "--output", "json", "--non-interactive"])
        .assert()
        .code(predicate::ne(101)); // no panic
}

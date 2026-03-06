//! Scenario: `hub list` returns hub data from mock server.

use crate::test_utils::start_cli_test;
use predicates::prelude::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hub_list_json_output_contains_id_and_name() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token");

    cmd.args(["hub", "list", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\""))
        .stdout(predicate::str::contains("\"name\""));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_hub_list_table_output_is_nonempty() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token");

    cmd.args(["hub", "list"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

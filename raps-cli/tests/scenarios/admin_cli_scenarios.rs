//! Full CLI scenario tests for admin commands.

use crate::test_utils::start_cli_test;
use predicates::prelude::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_user_add_to_all_projects_cli() {
    let (_server, mut cmd) = start_cli_test().await;

    // We need a fake token file since 3-leg auth normally requires browser flow
    // In tests, we can use an environment variable to point to a mock token
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token");

    cmd.args([
        "--verbose",
        "admin",
        "user",
        "add-to-all-projects",
        "test@example.com",
        "--account",
        "mock-account-001",
        "--role",
        "Project Admin",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("\"total\": 2"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_admin_user_add_account_not_found_exit_code_4() {
    let (_server, mut cmd) = start_cli_test().await;
    cmd.env("RAPS_FORCE_TOKEN", "mock-3leg-token");

    // "non-existent" account triggers 404 in raps-mock
    cmd.args([
        "admin",
        "user",
        "add-to-all-projects",
        "test@example.com",
        "--account",
        "non-existent",
    ])
    .assert()
    .failure()
    .code(4)
    .stderr(predicate::str::contains("Resource not found"));
}

//! Scenario: --dry-run guarantees no write API calls are made for any operation.

use raps_admin::filter::ProjectFilter;
use raps_admin::{BulkConfig, bulk_add_user, bulk_remove_user};
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_dry_run_add_makes_zero_write_calls() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    bulk_add_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "dryrun@example.com",
        Some("role-admin"),
        vec![],
        None,
        &ProjectFilter::new(),
        BulkConfig {
            dry_run: true,
            ..Default::default()
        },
        |_| {},
    )
    .await
    .unwrap();

    server.trace.assert_call_count(0);
}

#[tokio::test]
async fn test_dry_run_remove_makes_zero_write_calls() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    bulk_remove_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "alice@example.com",
        &ProjectFilter::new(),
        BulkConfig {
            dry_run: true,
            ..Default::default()
        },
        |_| {},
    )
    .await
    .unwrap();

    // The user-search POST is a read-only lookup; no mutation calls (DELETE/PUT/PATCH) should occur.
    server.trace.assert_not_called_with("DELETE", "/users");
    server.trace.assert_not_called_with("PUT", "/users");
    server.trace.assert_not_called_with("PATCH", "/users");
}

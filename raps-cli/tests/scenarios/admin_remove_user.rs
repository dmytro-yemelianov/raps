//! Scenario: admin removes a user from all projects.
//!
//! Expected API call sequence (write calls only, recorded by TraceRecorder):
//!   1. DELETE /projects/proj-001/users/user-001
//!   (proj-002 skipped: alice is not a member there)

use raps_admin::filter::ProjectFilter;
use raps_admin::{BulkConfig, bulk_remove_user};
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_remove_user_trace_contains_delete_calls() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    // alice@example.com is seeded in proj-001 (user-001)
    bulk_remove_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "alice@example.com",
        &ProjectFilter::new(),
        BulkConfig::default(),
        |_| {},
    )
    .await
    .unwrap();

    let calls = server.trace.calls();
    let trace: Vec<serde_json::Value> = calls
        .iter()
        .map(|c| serde_json::json!({"method": c.method, "path": c.path}))
        .collect();

    insta::assert_json_snapshot!("admin_remove_user_trace", trace);
}

#[tokio::test]
async fn test_remove_user_dry_run_sends_no_delete() {
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

    server.trace.assert_not_called_with("DELETE", "/users");
}

//! Scenario: admin adds a user to all active projects as project administrator.
//!
//! Expected API call sequence (write calls only, recorded by TraceRecorder):
//!   1. POST /projects/proj-001/users
//!   2. POST /projects/proj-002/users

use raps_admin::filter::ProjectFilter;
use raps_admin::{BulkConfig, bulk_add_user};
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_add_user_to_all_projects_trace_matches_snapshot() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    bulk_add_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "scenario@example.com",
        Some("role-project-admin"),
        vec![],
        None,
        &ProjectFilter::new(),
        BulkConfig {
            concurrency: 1,
            ..Default::default()
        },
        |_| {},
    )
    .await
    .unwrap();

    let calls = server.trace.calls();
    let trace: Vec<serde_json::Value> = calls
        .iter()
        .map(|c| serde_json::json!({"method": c.method, "path": c.path}))
        .collect();

    insta::assert_json_snapshot!("admin_add_user_all_projects_trace", trace);
}

#[tokio::test]
async fn test_add_user_all_projects_result_counts() {
    let server = TestServer::start_default().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = bulk_add_user(
        &clients.admin,
        clients.users.clone(),
        "mock-account-001",
        "counts@example.com",
        None,
        vec![],
        None,
        &ProjectFilter::new(),
        BulkConfig::default(),
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(result.total, 2, "2 active projects in mock");
    assert!(result.failed == 0, "no failures expected");
}

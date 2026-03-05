//! Scenario: admin archives a project.

use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_archive_project_trace_snapshot() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    clients
        .admin
        .archive_project("mock-account-001", "proj-002")
        .await
        .unwrap();

    let calls = server.trace.calls();
    let trace: Vec<serde_json::Value> = calls
        .iter()
        .map(|c| serde_json::json!({"method": c.method, "path": c.path}))
        .collect();

    insta::assert_json_snapshot!("admin_archive_project_trace", trace);
}

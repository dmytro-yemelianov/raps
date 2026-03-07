//! Operation tests: archiving a project via AccountAdminClient.

use raps_acc::admin::UpdateProjectRequest;
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_archive_project_sends_patch_with_status_archived() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    clients
        .admin
        .archive_project("mock-account-001", "proj-001")
        .await
        .unwrap();

    server
        .trace
        .assert_called_with("PATCH", "/projects/proj-001");
}

#[tokio::test]
async fn test_archive_project_via_update_returns_archived_status() {
    let server = TestServer::start_default().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = clients
        .admin
        .update_project(
            "mock-account-001",
            "proj-001",
            UpdateProjectRequest {
                status: Some("archived".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(result.status.as_deref(), Some("archived"));
}

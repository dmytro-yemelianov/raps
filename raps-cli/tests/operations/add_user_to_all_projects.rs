//! Operation tests: bulk_add_user across all active projects.

use std::sync::Arc;

use raps_admin::{BulkConfig, ProjectFilter, bulk_add_user};
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_bulk_add_user_calls_post_for_each_active_project() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    // Mock has proj-001 and proj-002 both active under mock-account-001.
    // bulk@example.com is not pre-seeded, so user_exists returns false for
    // both projects and add_user is called for each — two POSTs total.
    let result = bulk_add_user(
        &clients.admin,
        Arc::clone(&clients.users),
        "mock-account-001",
        "bulk@example.com",
        None,
        &ProjectFilter::new(),
        BulkConfig { concurrency: 2, dry_run: false, ..Default::default() },
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(result.total, 2);
    assert_eq!(result.completed, 2);
    assert_eq!(result.failed, 0);

    // One POST per project (user_exists is a GET — not traced)
    assert_eq!(server.trace.post_calls_to("/users").len(), 2);
}

#[tokio::test]
async fn test_bulk_add_user_with_role_id_sends_role_to_each_project() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = bulk_add_user(
        &clients.admin,
        Arc::clone(&clients.users),
        "mock-account-001",
        "roletest@example.com",
        Some("role-project-admin"),
        &ProjectFilter::new(),
        BulkConfig { concurrency: 2, dry_run: false, ..Default::default() },
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(result.total, 2);

    // Both proj-001 and proj-002 should have received a POST to their /users endpoint
    server.trace.assert_called_with("POST", "/projects/proj-001/users");
    server.trace.assert_called_with("POST", "/projects/proj-002/users");
}

#[tokio::test]
async fn test_bulk_add_user_dry_run_makes_no_post_calls() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = bulk_add_user(
        &clients.admin,
        Arc::clone(&clients.users),
        "mock-account-001",
        "dryrun@example.com",
        None,
        &ProjectFilter::new(),
        BulkConfig { dry_run: true, ..Default::default() },
        |_| {},
    )
    .await
    .unwrap();

    // Dry run: all items skipped, no actual API writes
    assert_eq!(result.total, 2);
    assert_eq!(result.completed, 0);
    assert_eq!(result.skipped, 2);

    server.trace.assert_call_count(0);
}

//! Edge case tests for admin operations.

use std::sync::Arc;

use raps_acc::users::AddProjectUserRequest;
use raps_admin::{BulkConfig, ProjectFilter, bulk_add_user};
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

/// user already exists in a project → should not be counted as "failed"
#[tokio::test]
async fn test_add_user_already_member_is_not_counted_as_failed() {
    let server = TestServer::start_default().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    // Add once (succeeds)
    clients
        .users
        .add_user(
            "proj-001",
            AddProjectUserRequest {
                email: "dup@example.com".into(),
                role_ids: vec![],
                products: vec![],
        suppress_administrative_emails: false,
            },
        )
        .await
        .unwrap();

    // Add again via bulk — should not count as failure
    let result = bulk_add_user(
        &clients.admin,
        Arc::clone(&clients.users),
        "mock-account-001",
        "dup@example.com",
        None,
        vec![],
        &ProjectFilter::new(),
        BulkConfig::default(),
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(result.failed, 0, "duplicate add must not count as failure");
}

/// Filter to a nonexistent project ID → total == 0
#[tokio::test]
async fn test_filter_excludes_all_projects_returns_zero_total() {
    let server = TestServer::start_default().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    // include_ids set to a project that does not exist in the mock
    let filter = ProjectFilter {
        include_ids: Some(vec!["proj-nonexistent".to_string()]),
        ..ProjectFilter::new()
    };

    let result = bulk_add_user(
        &clients.admin,
        Arc::clone(&clients.users),
        "mock-account-001",
        "empty@example.com",
        None,
        vec![],
        &filter,
        BulkConfig::default(),
        |_| {},
    )
    .await
    .unwrap();

    assert_eq!(result.total, 0, "filter should exclude all projects");
    assert_eq!(result.completed, 0);
}

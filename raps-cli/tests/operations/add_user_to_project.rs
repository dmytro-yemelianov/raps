//! Operation tests: adding a single user to a single project.

use raps_acc::users::AddProjectUserRequest;
use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_add_user_sends_post_to_correct_endpoint() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    clients
        .users
        .add_user(
            "proj-001",
            AddProjectUserRequest {
                email: "new@example.com".into(),
                role_ids: vec![],
                products: vec![],
        suppress_administrative_emails: false,
            },
        )
        .await
        .unwrap();

    server
        .trace
        .assert_called_with("POST", "/projects/proj-001/users");
    server.trace.assert_call_count(1);
}

#[tokio::test]
async fn test_add_user_with_role_id_sends_role_in_body() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = clients
        .users
        .add_user(
            "proj-001",
            AddProjectUserRequest {
                email: "roletest@example.com".into(),
                role_ids: vec!["role-project-admin".into()],
                products: vec![],
        suppress_administrative_emails: false,
            },
        )
        .await
        .unwrap();

    assert_eq!(result.role_ids.first().map(String::as_str), Some("role-project-admin"));
}

#[tokio::test]
async fn test_add_user_without_role_omits_role_id_key() {
    let server = TestServer::start_default().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = clients
        .users
        .add_user(
            "proj-001",
            AddProjectUserRequest {
                email: "norole@example.com".into(),
                role_ids: vec![],
                products: vec![],
        suppress_administrative_emails: false,
            },
        )
        .await
        .unwrap();

    // Mock assigns "role-default" when no roleIds in body
    assert_eq!(result.role_ids.first().map(String::as_str), Some("role-default"));
}

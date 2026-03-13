//! Operation tests: removing a user from a single project.

use raps_mock::TestServer;

use crate::test_utils::{inject_token, make_clients};

#[tokio::test]
async fn test_remove_existing_user_sends_delete() {
    let server = TestServer::start_with_trace().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    // user-001 / alice@example.com is seeded in proj-001 by the mock
    clients
        .users
        .remove_user("proj-001", "user-001")
        .await
        .unwrap();

    server
        .trace
        .assert_called_with("DELETE", "/projects/proj-001/users/user-001");
    server.trace.assert_call_count(1);
}

#[tokio::test]
async fn test_remove_nonexistent_user_succeeds_with_no_content() {
    // The mock returns 204 No Content unconditionally for DELETE, even when the
    // user does not exist. remove_user() therefore succeeds (returns Ok(())).
    let server = TestServer::start_default().await.unwrap();
    let clients = make_clients(&server.url);
    inject_token(&clients.auth, &server.url).await;

    let result = clients
        .users
        .remove_user("proj-001", "user-does-not-exist")
        .await;
    assert!(result.is_ok());
}

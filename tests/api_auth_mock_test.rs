use std::net::TcpListener;
use std::thread;

use raps::{api::auth::AuthClient, config::Config, http::HttpClientConfig};
use tiny_http::{Response, Server, StatusCode};

fn start_mock_server(status: u16, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind localhost");
    let addr = listener.local_addr().expect("read addr");
    let server = Server::from_tcp(listener).expect("start server");

    thread::spawn(move || {
        if let Some(request) = server.incoming_requests().next() {
            let response = Response::from_string(body).with_status_code(StatusCode(status));
            let _ = request.respond(response);
        }
    });

    format!("http://{}", addr)
}

fn test_config(base_url: &str) -> Config {
    Config {
        client_id: "test-client-id".into(),
        client_secret: "test-client-secret".into(),
        base_url: base_url.to_string(),
        callback_url: format!("{}/callback", base_url),
        da_nickname: None,
    }
}

#[tokio::test]
async fn get_token_returns_access_token_when_backend_succeeds() {
    let server_url = start_mock_server(
        200,
        r#"{
            "access_token": "mock-access-token",
            "token_type": "Bearer",
            "expires_in": 3600
        }"#,
    );

    let client =
        AuthClient::new_with_http_config(test_config(&server_url), HttpClientConfig::default());

    let token = client.get_token().await.expect("token should be returned");

    assert_eq!(token, "mock-access-token");
}

#[tokio::test]
async fn get_token_propagates_failure_status_and_message() {
    let server_url = start_mock_server(401, "invalid client credentials");

    let client =
        AuthClient::new_with_http_config(test_config(&server_url), HttpClientConfig::default());

    let err = client
        .get_token()
        .await
        .expect_err("token request should fail");

    let message = format!("{}", err);
    assert!(message.contains("401"));
    assert!(message.contains("invalid client credentials"));
}

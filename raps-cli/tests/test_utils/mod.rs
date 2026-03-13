//! Shared test utilities for operation and scenario tests.

use raps_acc::admin::AccountAdminClient;
use raps_acc::users::ProjectUsersClient;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;
use raps_kernel::types::StoredToken;

use assert_cmd::Command;
use raps_mock::TestServer;

pub struct TestClients {
    pub admin: AccountAdminClient,
    pub users: std::sync::Arc<ProjectUsersClient>,
    pub auth: AuthClient,
}

/// Start a mock server and return a Command configured to use it.
pub async fn start_cli_test() -> (TestServer, Command) {
    let _ = tracing_subscriber::fmt::try_init();
    let server = TestServer::start_default().await.unwrap();
    let mut cmd = Command::cargo_bin("raps").unwrap();

    // Configure CLI to use mock server
    cmd.env("APS_BASE_URL", &server.url);
    cmd.env("APS_CLIENT_ID", "test-client");
    cmd.env("APS_CLIENT_SECRET", "test-secret");

    (server, cmd)
}

pub fn make_clients(base_url: &str) -> TestClients {
    let config = Config {
        client_id: "test-client".into(),
        client_secret: "test-secret".into(),
        base_url: base_url.to_string(),
        callback_url: "http://localhost:8080/callback".into(),
        da_nickname: None,
        http_config: HttpClientConfig::default(),
    };
    let auth = AuthClient::new(config.clone());
    TestClients {
        admin: AccountAdminClient::new(config.clone(), auth.clone()),
        users: std::sync::Arc::new(ProjectUsersClient::new(config, auth.clone())),
        auth,
    }
}

/// Get a valid token from the mock server and inject it as the 3-leg token.
pub async fn inject_token(auth: &AuthClient, base_url: &str) {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/authentication/v2/token", base_url))
        .json(&serde_json::json!({
            "client_id": "test-client",
            "client_secret": "test-secret",
            "grant_type": "client_credentials"
        }))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let access_token = body["access_token"].as_str().unwrap().to_string();
    auth.set_3leg_token_for_testing(StoredToken {
        access_token,
        refresh_token: None,
        expires_at: chrono::Utc::now().timestamp() + 3600,
        scopes: vec![],
    })
    .await;
}

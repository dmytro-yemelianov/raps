// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! Object Storage Service (OSS) API module
//!
//! Handles bucket and object operations for storing files in APS.
//! Supports multipart chunked uploads for large files with resume capability.

mod batch;
mod buckets;
mod multipart;
mod objects;
mod range;
pub mod types;

pub use types::*;

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

/// OSS API client
#[derive(Clone)]
pub struct OssClient {
    pub(crate) config: Config,
    pub(crate) auth: AuthClient,
    pub(crate) http_client: reqwest::Client,
}

impl OssClient {
    /// Create a new OSS client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new OSS client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
    ) -> Self {
        // Create HTTP client with configured timeouts
        let http_client = http_config.create_client().unwrap_or_else(|e| {
            tracing::warn!("HTTP client configuration failed, using defaults: {e}");
            reqwest::Client::new()
        });

        Self {
            config,
            auth,
            http_client,
        }
    }

    /// Generate a base64-encoded URN for an object
    pub fn get_urn(&self, bucket_key: &str, object_key: &str) -> String {
        use base64::Engine;
        let object_id = format!("urn:adsk.objects:os.object:{}/{}", bucket_key, object_key);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(object_id)
    }
}

/// Integration tests using raps-mock
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;

    fn create_mock_client(mock_url: &str) -> OssClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        OssClient::new(config, auth)
    }

    #[tokio::test]
    async fn test_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }

    #[test]
    fn test_get_urn_encoding() {
        let server_url = "http://localhost:3000";
        let client = create_mock_client(server_url);
        let urn = client.get_urn("my-bucket", "model.rvt");
        // URN should be base64url-encoded
        assert!(!urn.contains('/'));
        assert!(!urn.is_empty());
        // Decode and verify the raw URN
        use base64::Engine;
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&urn)
            .unwrap();
        let raw = String::from_utf8(decoded).unwrap();
        assert_eq!(raw, "urn:adsk.objects:os.object:my-bucket/model.rvt");
    }

    #[test]
    fn test_get_urn_special_characters() {
        let client = create_mock_client("http://localhost:3000");
        let urn = client.get_urn("test-bucket", "path/to/model (v2).rvt");
        assert!(!urn.is_empty());
        use base64::Engine;
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&urn)
            .unwrap();
        let raw = String::from_utf8(decoded).unwrap();
        assert!(raw.contains("path/to/model (v2).rvt"));
    }

    /// Helper: get a valid mock token by calling the mock server's token endpoint
    /// (with JSON body that the mock accepts), then cache it in the auth client.
    async fn acquire_mock_token(client: &OssClient, mock_url: &str) {
        let http = reqwest::Client::new();
        let resp = http
            .post(format!("{}/authentication/v2/token", mock_url))
            .json(&serde_json::json!({
                "grant_type": "client_credentials",
                "client_id": "test-client-id",
                "client_secret": "test-client-secret"
            }))
            .send()
            .await
            .expect("token request failed");
        assert!(
            resp.status().is_success(),
            "token endpoint returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.expect("invalid token JSON");
        let token = body["access_token"]
            .as_str()
            .expect("no access_token")
            .to_string();
        let expires_in = body["expires_in"].as_u64().unwrap_or(3600);
        client
            .auth
            .set_2leg_token_for_testing(token, expires_in)
            .await;
    }

    #[tokio::test]
    async fn test_bucket_list_with_mock() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_client(&server.url);
        acquire_mock_token(&client, &server.url).await;

        let result = client.list_buckets().await;
        assert!(result.is_ok(), "list_buckets failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_object_list_with_mock() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_client(&server.url);
        acquire_mock_token(&client, &server.url).await;

        // List objects in a pre-seeded bucket (mock always returns empty list for unknown buckets)
        let result = client.list_objects("test-bucket").await;
        assert!(result.is_ok(), "list_objects failed: {:?}", result.err());
    }
}

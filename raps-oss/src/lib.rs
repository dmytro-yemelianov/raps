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
///
/// `list_buckets`, `list_objects`, and `create_bucket` work through the client API.
/// Some other mutation operations still use raw HTTP to verify mock endpoints.
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

    async fn acquire_mock_token(mock_url: &str) -> String {
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
        body["access_token"]
            .as_str()
            .expect("no access_token")
            .to_string()
    }

    async fn setup_client_with_token(mock_url: &str) -> (OssClient, String) {
        let client = create_mock_client(mock_url);
        let token = acquire_mock_token(mock_url).await;
        client
            .auth
            .set_2leg_token_for_testing(token.clone(), 3600)
            .await;
        (client, token)
    }

    /// Create a bucket via raw HTTP (bypasses client type parsing)
    async fn create_bucket_raw(mock_url: &str, token: &str, bucket_key: &str) {
        let http = reqwest::Client::new();
        let resp = http
            .post(format!("{}/oss/v2/buckets", mock_url))
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "bucketKey": bucket_key,
                "policyKey": "transient"
            }))
            .send()
            .await
            .expect("create bucket request failed");
        assert!(
            resp.status().is_success(),
            "create bucket returned {}",
            resp.status()
        );
    }

    // --- Client-based tests that work with mock ---

    #[tokio::test]
    async fn test_bucket_list_with_mock() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (client, _token) = setup_client_with_token(&server.url).await;

        let result = client.list_buckets().await;
        assert!(result.is_ok(), "list_buckets failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_object_list_with_mock() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (client, _token) = setup_client_with_token(&server.url).await;

        let result = client.list_objects("test-bucket").await;
        assert!(result.is_ok(), "list_objects failed: {:?}", result.err());
    }

    // --- Client method tests for bucket creation (mock now returns full Bucket response) ---

    #[tokio::test]
    async fn test_create_bucket_client_method() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (client, _token) = setup_client_with_token(&server.url).await;

        let result = client
            .create_bucket("test-new-bucket", RetentionPolicy::Transient, Region::US)
            .await;
        assert!(result.is_ok(), "create_bucket failed: {:?}", result.err());
        let bucket = result.unwrap();
        assert!(!bucket.bucket_key.is_empty());
        assert!(!bucket.bucket_owner.is_empty());
        assert!(!bucket.policy_key.is_empty());
    }

    #[tokio::test]
    async fn test_create_bucket_persistent_policy_client_method() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (client, _token) = setup_client_with_token(&server.url).await;

        let result = client
            .create_bucket("persistent-bucket", RetentionPolicy::Persistent, Region::US)
            .await;
        assert!(
            result.is_ok(),
            "create persistent bucket failed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_get_bucket_details_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;

        // Create bucket first
        create_bucket_raw(&server.url, &token, "details-bucket").await;

        let http = reqwest::Client::new();
        let resp = http
            .get(format!("{}/oss/v2/buckets/details-bucket/details", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "get bucket details returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["bucketKey"].is_string(), "should contain bucketKey");
    }

    #[tokio::test]
    async fn test_delete_bucket_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;

        create_bucket_raw(&server.url, &token, "delete-bucket").await;

        let http = reqwest::Client::new();
        let resp = http
            .delete(format!("{}/oss/v2/buckets/delete-bucket", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "delete bucket returned {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_upload_object_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        create_bucket_raw(&server.url, &token, "upload-bucket").await;

        // Get signed upload URL
        let sign_resp = http
            .get(format!(
                "{}/oss/v2/buckets/upload-bucket/objects/test-upload.txt/signeds3upload",
                server.url
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            sign_resp.status().is_success(),
            "get signed upload URL returned {}",
            sign_resp.status()
        );
        let sign_body: serde_json::Value = sign_resp.json().await.unwrap();
        assert!(sign_body["urls"].is_array(), "should contain upload URLs");
        assert!(sign_body["uploadKey"].is_string(), "should contain uploadKey");
    }

    #[tokio::test]
    async fn test_delete_object_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        create_bucket_raw(&server.url, &token, "del-obj-bucket").await;

        let resp = http
            .delete(format!(
                "{}/oss/v2/buckets/del-obj-bucket/objects/some-object.txt",
                server.url
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 204 || resp.status().as_u16() == 404,
            "delete object returned {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_list_objects_in_created_bucket_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        create_bucket_raw(&server.url, &token, "list-objects-bucket").await;

        let resp = http
            .get(format!(
                "{}/oss/v2/buckets/list-objects-bucket/objects",
                server.url
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "list objects returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["items"].is_array(), "should contain items array");
    }

    #[tokio::test]
    async fn test_copy_object_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        create_bucket_raw(&server.url, &token, "copy-bucket").await;

        // Copy uses PUT with x-ads-copy-from header in format "bucket/objectKey"
        let resp = http
            .put(format!(
                "{}/oss/v2/buckets/copy-bucket/objects/copied.txt",
                server.url
            ))
            .bearer_auth(&token)
            .header("x-ads-copy-from", "copy-bucket/original.txt")
            .send()
            .await
            .unwrap();
        // Mock returns 400 if source doesn't exist or copy-from format is wrong;
        // we verify the endpoint is reachable and responds
        assert!(
            resp.status().as_u16() == 200 || resp.status().as_u16() == 400,
            "copy object returned unexpected status {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_bucket_create_list_delete_lifecycle_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        // Create bucket
        create_bucket_raw(&server.url, &token, "lifecycle-bucket").await;

        // List buckets - should contain the created bucket
        let list_resp = http
            .get(format!("{}/oss/v2/buckets", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(list_resp.status().is_success());
        let list_body: serde_json::Value = list_resp.json().await.unwrap();
        let items = list_body["items"].as_array().unwrap();
        let has_lifecycle = items
            .iter()
            .any(|b| b["bucketKey"].as_str() == Some("lifecycle-bucket"));
        assert!(has_lifecycle, "created bucket should appear in list");

        // Delete bucket
        let del_resp = http
            .delete(format!("{}/oss/v2/buckets/lifecycle-bucket", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(del_resp.status().is_success(), "delete bucket should succeed");
    }

    #[tokio::test]
    async fn test_get_signed_download_url_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        create_bucket_raw(&server.url, &token, "download-bucket").await;

        // Upload an object first so the download URL can be generated
        // Get signed upload URL
        let sign_resp = http
            .get(format!(
                "{}/oss/v2/buckets/download-bucket/objects/download-test.txt/signeds3upload",
                server.url
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(sign_resp.status().is_success());
        let sign_body: serde_json::Value = sign_resp.json().await.unwrap();
        let upload_url = sign_body["urls"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .expect("need upload URL");
        let upload_key = sign_body["uploadKey"].as_str().expect("need uploadKey");

        // Upload content
        let _ = http
            .put(upload_url)
            .body("test download content")
            .send()
            .await
            .unwrap();

        // Complete upload
        http.post(format!(
            "{}/oss/v2/buckets/download-bucket/objects/download-test.txt/signeds3upload",
            server.url
        ))
        .bearer_auth(&token)
        .json(&serde_json::json!({"uploadKey": upload_key}))
        .send()
        .await
        .unwrap();

        // Now get signed download URL
        let resp = http
            .get(format!(
                "{}/oss/v2/buckets/download-bucket/objects/download-test.txt/signeds3download",
                server.url
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "get signed download URL returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["url"].is_string(), "should contain download URL");
    }
}

// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! Model Derivative API module
//!
//! Handles translation of CAD files and retrieval of derivative manifests.
//! Supports downloading translated derivatives directly from manifest.

mod download;
mod metadata;
pub mod translation_cache;
mod translations;
pub mod types;

pub use translation_cache::TranslationCache;
pub use types::*;

use anyhow::Context;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

/// Model Derivative API client
#[derive(Clone)]
pub struct DerivativeClient {
    pub(crate) config: Config,
    pub(crate) auth: AuthClient,
    pub(crate) http_client: reqwest::Client,
}

impl DerivativeClient {
    /// Create a new Model Derivative client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
            .expect("default HTTP client configuration must always succeed")
    }

    /// Create a new Model Derivative client with custom HTTP config.
    ///
    /// Returns an error if the HTTP client cannot be built (e.g. invalid proxy URL).
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
    ) -> anyhow::Result<Self> {
        let http_client = http_config
            .create_client()
            .context("Failed to initialise HTTP client for Model Derivative")?;

        Ok(Self {
            config,
            auth,
            http_client,
        })
    }
}

/// Integration tests using raps-mock
#[cfg(test)]
mod integration_tests {
    use super::*;
    use base64::Engine;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;

    fn create_mock_client(mock_url: &str) -> DerivativeClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        DerivativeClient::new(config, auth)
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

    async fn setup_client_with_token(mock_url: &str) -> (DerivativeClient, String) {
        let client = create_mock_client(mock_url);
        let token = acquire_mock_token(mock_url).await;
        let expires_in = 3600u64;
        client
            .auth
            .set_2leg_token_for_testing(token.clone(), expires_in)
            .await;
        (client, token)
    }

    /// Start a translation job via raw HTTP (bypassing client type parsing)
    /// to prime the mock server's state for subsequent tests.
    async fn start_mock_translation(mock_url: &str, token: &str, urn: &str) {
        let http = reqwest::Client::new();
        let resp = http
            .post(format!("{}/modelderivative/v2/designdata/job", mock_url))
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "input": {"urn": urn},
                "output": {
                    "destination": {"region": "us"},
                    "formats": [{"type": "svf2", "views": ["2d", "3d"]}]
                }
            }))
            .send()
            .await
            .expect("translation POST failed");
        assert!(
            resp.status().is_success(),
            "translation POST returned {}",
            resp.status()
        );
    }

    /// Poll manifest via raw HTTP to advance the mock's translation state.
    async fn advance_to_success_raw(mock_url: &str, token: &str, urn: &str) {
        let http = reqwest::Client::new();
        for _ in 0..6 {
            let _ = http
                .get(format!(
                    "{}/modelderivative/v2/designdata/{}/manifest",
                    mock_url, urn
                ))
                .bearer_auth(token)
                .send()
                .await;
        }
    }

    #[tokio::test]
    async fn test_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }

    #[tokio::test]
    async fn test_translate_sends_request_successfully() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let test_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:test-bucket/model.rvt");
        let http = reqwest::Client::new();
        let resp = http
            .post(format!("{}/modelderivative/v2/designdata/job", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "input": {"urn": &test_urn},
                "output": {
                    "destination": {"region": "us"},
                    "formats": [{"type": "svf2", "views": ["2d", "3d"]}]
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success(), "translation request should succeed");
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["result"], "success");
    }

    #[tokio::test]
    async fn test_translate_response_contains_urn() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let test_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:test-bucket/urn-check.rvt");
        let http = reqwest::Client::new();
        let resp = http
            .post(format!("{}/modelderivative/v2/designdata/job", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "input": {"urn": &test_urn},
                "output": {
                    "destination": {"region": "us"},
                    "formats": [{"type": "svf2", "views": ["2d", "3d"]}]
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["urn"].is_string(), "response should contain urn");
        assert!(body["acceptedJobs"].is_object(), "response should contain acceptedJobs");
    }

    #[tokio::test]
    async fn test_get_manifest_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let test_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:test-bucket/manifest-raw.rvt");
        start_mock_translation(&server.url, &token, &test_urn).await;

        let http = reqwest::Client::new();
        let resp = http
            .get(format!(
                "{}/modelderivative/v2/designdata/{}/manifest",
                server.url, test_urn
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success(), "get manifest should succeed");
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["type"], "manifest");
        assert!(body["status"].is_string(), "status should be present");
        assert!(body["progress"].is_string(), "progress should be present");
        assert!(body["urn"].is_string(), "urn should be present");
    }

    #[tokio::test]
    async fn test_manifest_pending_status_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let test_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:test-bucket/pending-raw.rvt");
        start_mock_translation(&server.url, &token, &test_urn).await;

        let http = reqwest::Client::new();
        let resp = http
            .get(format!(
                "{}/modelderivative/v2/designdata/{}/manifest",
                server.url, test_urn
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "get manifest returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        let status = body["status"].as_str().unwrap();
        assert!(
            status == "pending" || status == "inprogress",
            "expected pending or inprogress, got: {}",
            status
        );
    }

    #[tokio::test]
    async fn test_manifest_progress_advances_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let test_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:test-bucket/progress-raw.rvt");
        start_mock_translation(&server.url, &token, &test_urn).await;

        let http = reqwest::Client::new();
        let manifest_url = format!(
            "{}/modelderivative/v2/designdata/{}/manifest",
            server.url, test_urn
        );

        // First poll
        let resp1 = http.get(&manifest_url).bearer_auth(&token).send().await.unwrap();
        let body1: serde_json::Value = resp1.json().await.unwrap();
        let progress1 = body1["progress"].as_str().unwrap().to_string();

        // Poll multiple times to advance
        for _ in 0..5 {
            let _ = http.get(&manifest_url).bearer_auth(&token).send().await;
        }

        let resp2 = http.get(&manifest_url).bearer_auth(&token).send().await.unwrap();
        let body2: serde_json::Value = resp2.json().await.unwrap();
        let progress2 = body2["progress"].as_str().unwrap().to_string();
        let status2 = body2["status"].as_str().unwrap();

        assert!(
            progress1 != progress2 || status2 == "success",
            "manifest should progress: start={}, end={}/{}",
            progress1, status2, progress2
        );
    }

    #[tokio::test]
    async fn test_get_metadata_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let test_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:test-bucket/metadata-raw.rvt");
        start_mock_translation(&server.url, &token, &test_urn).await;
        advance_to_success_raw(&server.url, &token, &test_urn).await;

        let http = reqwest::Client::new();
        let resp = http
            .get(format!(
                "{}/modelderivative/v2/designdata/{}/metadata",
                server.url, test_urn
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success(), "get metadata should succeed");
        let body: serde_json::Value = resp.json().await.unwrap();
        let metadata = &body["data"]["metadata"];
        assert!(metadata.is_array(), "metadata should be an array");
        let views = metadata.as_array().unwrap();
        assert!(!views.is_empty(), "metadata views should not be empty");
        assert!(views[0]["guid"].is_string(), "view should have a guid");
        assert!(views[0]["name"].is_string(), "view should have a name");
    }

    #[tokio::test]
    async fn test_get_object_tree_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let test_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:test-bucket/tree-raw.rvt");
        start_mock_translation(&server.url, &token, &test_urn).await;
        advance_to_success_raw(&server.url, &token, &test_urn).await;

        // Get metadata to find a model guid
        let http = reqwest::Client::new();
        let meta_resp = http
            .get(format!(
                "{}/modelderivative/v2/designdata/{}/metadata",
                server.url, test_urn
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let meta_body: serde_json::Value = meta_resp.json().await.unwrap();
        let model_guid = meta_body["data"]["metadata"][0]["guid"]
            .as_str()
            .expect("need a model guid");

        let resp = http
            .get(format!(
                "{}/modelderivative/v2/designdata/{}/metadata/{}",
                server.url, test_urn, model_guid
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "get object tree should succeed, got {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["data"]["objects"].is_array(), "should contain objects array");
    }

    #[tokio::test]
    async fn test_get_properties_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let test_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:test-bucket/props-raw.rvt");
        start_mock_translation(&server.url, &token, &test_urn).await;
        advance_to_success_raw(&server.url, &token, &test_urn).await;

        // Get metadata to find a model guid
        let http = reqwest::Client::new();
        let meta_resp = http
            .get(format!(
                "{}/modelderivative/v2/designdata/{}/metadata",
                server.url, test_urn
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let meta_body: serde_json::Value = meta_resp.json().await.unwrap();
        let model_guid = meta_body["data"]["metadata"][0]["guid"]
            .as_str()
            .expect("need a model guid");

        let resp = http
            .get(format!(
                "{}/modelderivative/v2/designdata/{}/metadata/{}/properties",
                server.url, test_urn, model_guid
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "get properties should succeed, got {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(
            body["data"]["collection"].is_array(),
            "should contain collection array"
        );
    }

    #[tokio::test]
    async fn test_query_properties_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let test_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:test-bucket/query-props-raw.rvt");
        start_mock_translation(&server.url, &token, &test_urn).await;
        advance_to_success_raw(&server.url, &token, &test_urn).await;

        // Get metadata to find a model guid
        let http = reqwest::Client::new();
        let meta_resp = http
            .get(format!(
                "{}/modelderivative/v2/designdata/{}/metadata",
                server.url, test_urn
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let meta_body: serde_json::Value = meta_resp.json().await.unwrap();
        let model_guid = meta_body["data"]["metadata"][0]["guid"]
            .as_str()
            .expect("need a model guid");

        let resp = http
            .post(format!(
                "{}/modelderivative/v2/designdata/{}/metadata/{}/properties:query",
                server.url, test_urn, model_guid
            ))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "query": {"$in": ["objectid", 1, 2]}
            }))
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "query properties should succeed, got {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_translate_request_with_obj_format() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let test_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:test-bucket/obj-test.rvt");
        let http = reqwest::Client::new();
        let resp = http
            .post(format!("{}/modelderivative/v2/designdata/job", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "input": {"urn": &test_urn},
                "output": {
                    "destination": {"region": "us"},
                    "formats": [{"type": "obj"}]
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success(), "OBJ translate request should succeed");
    }

    #[tokio::test]
    async fn test_translate_request_with_stl_format() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let test_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:test-bucket/stl-test.rvt");
        let http = reqwest::Client::new();
        let resp = http
            .post(format!("{}/modelderivative/v2/designdata/job", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "input": {"urn": &test_urn},
                "output": {
                    "destination": {"region": "us"},
                    "formats": [{"type": "stl"}]
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success(), "STL translate request should succeed");
    }

    #[tokio::test]
    async fn test_manifest_not_found_for_unknown_urn() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let unknown_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:nonexistent/unknown.rvt");
        let http = reqwest::Client::new();
        let resp = http
            .get(format!(
                "{}/modelderivative/v2/designdata/{}/manifest",
                server.url, unknown_urn
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 404, "unknown URN should return 404");
    }

    #[tokio::test]
    async fn test_translate_then_get_manifest_lifecycle() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (_client, token) = setup_client_with_token(&server.url).await;

        let test_urn = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode("urn:adsk.objects:os.object:test-bucket/lifecycle.rvt");
        let http = reqwest::Client::new();

        // 1. Start translation
        let resp = http
            .post(format!("{}/modelderivative/v2/designdata/job", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "input": {"urn": &test_urn},
                "output": {
                    "destination": {"region": "us"},
                    "formats": [{"type": "svf2", "views": ["2d", "3d"]}]
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());

        // 2. Poll manifest until success
        let manifest_url = format!(
            "{}/modelderivative/v2/designdata/{}/manifest",
            server.url, test_urn
        );
        let mut final_status = String::new();
        for _ in 0..10 {
            let resp = http.get(&manifest_url).bearer_auth(&token).send().await.unwrap();
            let body: serde_json::Value = resp.json().await.unwrap();
            final_status = body["status"].as_str().unwrap_or("").to_string();
            if final_status == "success" {
                break;
            }
        }
        assert_eq!(final_status, "success", "translation should reach success");

        // 3. Fetch metadata
        let meta_resp = http
            .get(format!(
                "{}/modelderivative/v2/designdata/{}/metadata",
                server.url, test_urn
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(meta_resp.status().is_success(), "metadata should be available after success");
    }
}

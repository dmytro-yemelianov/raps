// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! Design Automation API module
//!
//! Handles automation of CAD processing with engines like AutoCAD, Revit, Inventor, 3ds Max.

mod activities;
mod appbundles;
mod engines;
pub mod types;
mod workitems;

pub use types::*;

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

/// Design Automation API client
#[derive(Clone)]
pub struct DesignAutomationClient {
    pub(crate) config: Config,
    pub(crate) auth: AuthClient,
    pub(crate) http_client: reqwest::Client,
}

impl DesignAutomationClient {
    /// Create a new Design Automation client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new Design Automation client with custom HTTP config
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
}

/// Integration tests using raps-mock
///
/// All DA endpoints are tested via raw HTTP because the mock server's DA routes
/// are at `/da/us-east/v3/*` and the client's type parsing may not match the
/// simplified mock responses.
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;

    fn create_mock_da_client(mock_url: &str) -> DesignAutomationClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: Some("test-nickname".to_string()),
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        DesignAutomationClient::new(config, auth)
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

    #[tokio::test]
    async fn test_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_da_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }

    #[tokio::test]
    async fn test_nickname_returns_configured() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_da_client(&server.url);
        assert_eq!(client.nickname(), "test-nickname");
    }

    #[tokio::test]
    async fn test_list_engines_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;

        let http = reqwest::Client::new();
        let resp = http
            .get(format!("{}/da/us-east/v3/engines", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "list engines returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        let data = body["data"].as_array().expect("data should be an array");
        assert!(!data.is_empty(), "engines list should not be empty");

        let engines: Vec<&str> = data.iter().filter_map(|v| v.as_str()).collect();
        let has_revit = engines.iter().any(|e| e.contains("Revit"));
        let has_autocad = engines.iter().any(|e| e.contains("AutoCAD"));
        assert!(has_revit, "engines should contain Revit, got: {:?}", engines);
        assert!(has_autocad, "engines should contain AutoCAD, got: {:?}", engines);
    }

    #[tokio::test]
    async fn test_create_appbundle_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;

        let http = reqwest::Client::new();
        let resp = http
            .post(format!("{}/da/us-east/v3/appbundles", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "id": "TestBundle",
                "engine": "Autodesk.Revit+2025",
                "description": "A test bundle"
            }))
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "create appbundle returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["id"].is_string(), "response should contain id");
        assert!(body["engine"].is_string(), "response should contain engine");
        assert!(
            body["uploadParameters"].is_object(),
            "response should contain uploadParameters"
        );
    }

    #[tokio::test]
    async fn test_list_appbundles_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        // Create one first
        http.post(format!("{}/da/us-east/v3/appbundles", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "id": "ListBundle",
                "engine": "Autodesk.Revit+2025",
                "description": ""
            }))
            .send()
            .await
            .unwrap();

        let resp = http
            .get(format!("{}/da/us-east/v3/appbundles", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "list appbundles returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        let data = body["data"].as_array().expect("data should be an array");
        assert!(!data.is_empty(), "appbundle list should not be empty after create");
    }

    #[tokio::test]
    async fn test_delete_appbundle_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        // Create first
        http.post(format!("{}/da/us-east/v3/appbundles", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "id": "DeleteBundle",
                "engine": "Autodesk.Revit+2025",
                "description": ""
            }))
            .send()
            .await
            .unwrap();

        let resp = http
            .delete(format!("{}/da/us-east/v3/appbundles/DeleteBundle", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 204,
            "delete appbundle returned {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_create_activity_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;

        let http = reqwest::Client::new();
        let resp = http
            .post(format!("{}/da/us-east/v3/activities", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "id": "TestActivity",
                "engine": "Autodesk.Revit+2025",
                "commandLine": ["$(engine.path)\\\\revitcoreconsole.exe /i $(args[inputFile].path)"],
                "appBundles": ["test-nickname.TestBundle+prod"],
                "parameters": {},
                "description": "Test activity"
            }))
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "create activity returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["id"].is_string(), "response should contain id");
        assert!(body["engine"].is_string(), "response should contain engine");
    }

    #[tokio::test]
    async fn test_list_activities_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        // Create one first
        http.post(format!("{}/da/us-east/v3/activities", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "id": "ListActivity",
                "engine": "Autodesk.Revit+2025",
                "commandLine": ["test"],
                "appBundles": [],
                "parameters": {}
            }))
            .send()
            .await
            .unwrap();

        let resp = http
            .get(format!("{}/da/us-east/v3/activities", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "list activities returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        let data = body["data"].as_array().expect("data should be an array");
        assert!(!data.is_empty(), "activities list should not be empty after create");
    }

    #[tokio::test]
    async fn test_delete_activity_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        // Create first
        http.post(format!("{}/da/us-east/v3/activities", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "id": "DeleteActivity",
                "engine": "Autodesk.Revit+2025",
                "commandLine": ["test"],
                "appBundles": [],
                "parameters": {}
            }))
            .send()
            .await
            .unwrap();

        let resp = http
            .delete(format!("{}/da/us-east/v3/activities/DeleteActivity", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 204,
            "delete activity returned {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_create_workitem_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;

        let http = reqwest::Client::new();
        let resp = http
            .post(format!("{}/da/us-east/v3/workitems", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "activityId": "test-nickname.TestActivity+prod",
                "arguments": {
                    "inputFile": {
                        "url": "https://example.com/input.rvt",
                        "verb": "get"
                    }
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "create workitem returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["id"].is_string(), "response should contain id");
        assert!(body["status"].is_string(), "response should contain status");
        assert_eq!(body["status"], "pending", "new workitem should be pending");
    }

    #[tokio::test]
    async fn test_get_workitem_status_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        // Create a workitem first
        let create_resp = http
            .post(format!("{}/da/us-east/v3/workitems", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "activityId": "test-nickname.TestActivity+prod",
                "arguments": {
                    "inputFile": {
                        "url": "https://example.com/input.rvt",
                        "verb": "get"
                    }
                }
            }))
            .send()
            .await
            .unwrap();
        let created: serde_json::Value = create_resp.json().await.unwrap();
        let workitem_id = created["id"].as_str().expect("need workitem id");

        let resp = http
            .get(format!(
                "{}/da/us-east/v3/workitems/{}",
                server.url, workitem_id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "get workitem status returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["id"], workitem_id);
        assert!(body["status"].is_string(), "workitem status should be present");
    }

    #[tokio::test]
    async fn test_list_workitems_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;

        let http = reqwest::Client::new();
        let resp = http
            .get(format!("{}/da/us-east/v3/workitems", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "list workitems returned {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_appbundle_create_list_delete_lifecycle() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        // Create
        let resp = http
            .post(format!("{}/da/us-east/v3/appbundles", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "id": "LifecycleBundle",
                "engine": "Autodesk.Revit+2025",
                "description": "lifecycle test"
            }))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success(), "create failed");
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["id"].is_string());

        // List - should contain it
        let list_resp = http
            .get(format!("{}/da/us-east/v3/appbundles", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let list_body: serde_json::Value = list_resp.json().await.unwrap();
        let bundles = list_body["data"].as_array().unwrap();
        let has_lifecycle = bundles
            .iter()
            .any(|b| b.as_str().map_or(false, |s| s.contains("LifecycleBundle")));
        assert!(has_lifecycle, "created bundle should appear in list");

        // Delete
        let del_resp = http
            .delete(format!(
                "{}/da/us-east/v3/appbundles/LifecycleBundle",
                server.url
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            del_resp.status().is_success() || del_resp.status().as_u16() == 204,
            "delete failed"
        );
    }

    #[tokio::test]
    async fn test_activity_create_list_delete_lifecycle() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        // Create
        let resp = http
            .post(format!("{}/da/us-east/v3/activities", server.url))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "id": "LifecycleActivity",
                "engine": "Autodesk.Revit+2025",
                "commandLine": ["test"],
                "appBundles": [],
                "parameters": {},
                "description": "lifecycle test"
            }))
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success(), "create failed");
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["id"].is_string());

        // List - should contain it
        let list_resp = http
            .get(format!("{}/da/us-east/v3/activities", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let list_body: serde_json::Value = list_resp.json().await.unwrap();
        let activities = list_body["data"].as_array().unwrap();
        let has_lifecycle = activities
            .iter()
            .any(|a| a.as_str().map_or(false, |s| s.contains("LifecycleActivity")));
        assert!(has_lifecycle, "created activity should appear in list");

        // Delete
        let del_resp = http
            .delete(format!(
                "{}/da/us-east/v3/activities/LifecycleActivity",
                server.url
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            del_resp.status().is_success() || del_resp.status().as_u16() == 204,
            "delete failed"
        );
    }
}

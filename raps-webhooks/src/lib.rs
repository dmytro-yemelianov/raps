// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! Webhooks API module
//!
//! Handles webhook subscriptions for automated event notifications.

use anyhow::{Context, Result};
use raps_kernel::error::RapsError;
use serde::{Deserialize, Serialize};

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::{self, HttpClientConfig};

/// Available webhook events
pub const WEBHOOK_EVENTS: &[(&str, &str)] = &[
    ("dm.version.added", "New file version added"),
    ("dm.version.modified", "File version modified"),
    ("dm.version.deleted", "File version deleted"),
    ("dm.version.moved", "File version moved"),
    ("dm.version.copied", "File version copied"),
    ("dm.folder.added", "Folder created"),
    ("dm.folder.modified", "Folder modified"),
    ("dm.folder.deleted", "Folder deleted"),
    ("dm.folder.moved", "Folder moved"),
    ("dm.folder.copied", "Folder copied"),
    (
        "extraction.finished",
        "Model derivative extraction finished",
    ),
    ("extraction.updated", "Model derivative extraction updated"),
];

/// Webhook subscription
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Webhook {
    pub hook_id: String,
    pub tenant: Option<String>,
    pub callback_url: String,
    pub created_by: Option<String>,
    pub event: String,
    pub created_date: Option<String>,
    pub last_updated_date: Option<String>,
    pub system: String,
    pub creator_type: Option<String>,
    pub status: String,
    pub scope: Option<WebhookScope>,
    pub hook_attribute: Option<serde_json::Value>,
    pub urn: Option<String>,
    pub auto_reactivate_hook: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookScope {
    pub folder: Option<String>,
    pub workflow: Option<String>,
}

/// Request to create a webhook
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebhookRequest {
    pub callback_url: String,
    pub scope: CreateWebhookScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_attribute: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hub_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_reactivate_hook: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebhookScope {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<String>,
}

/// Request to update a webhook
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWebhookRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

/// Webhooks response
#[derive(Debug, Deserialize)]
pub struct WebhooksResponse {
    pub data: Vec<Webhook>,
    pub links: Option<WebhooksLinks>,
}

#[derive(Debug, Deserialize)]
pub struct WebhooksLinks {
    pub next: Option<String>,
}

/// Webhooks API client
///
/// Uses 2-legged OAuth (client credentials) via [`AuthClient::get_token()`] for all
/// operations, as required by the APS Webhooks API. This is distinct from 3-legged
/// OAuth used by Data Management endpoints that operate in a user context.
#[derive(Clone)]
pub struct WebhooksClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl WebhooksClient {
    /// Create a new Webhooks client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new Webhooks client with custom HTTP config
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

    /// List all webhooks for a system and event
    pub async fn list_webhooks(&self, system: &str, event: &str) -> Result<Vec<Webhook>> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/systems/{}/events/{}/hooks",
            self.config.webhooks_url(),
            system,
            event
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let webhooks_response: WebhooksResponse = response
            .json()
            .await
            .context("Failed to parse webhooks response")?;

        Ok(webhooks_response.data)
    }

    /// List all webhooks across all events
    pub async fn list_all_webhooks(&self) -> Result<Vec<Webhook>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/hooks", self.config.webhooks_url());

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let webhooks_response: WebhooksResponse = response
            .json()
            .await
            .context("Failed to parse webhooks response")?;

        Ok(webhooks_response.data)
    }

    /// Create a new webhook subscription
    pub async fn create_webhook(
        &self,
        system: &str,
        event: &str,
        callback_url: &str,
        folder_urn: Option<&str>,
    ) -> Result<Webhook> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/systems/{}/events/{}/hooks",
            self.config.webhooks_url(),
            system,
            event
        );

        let request = CreateWebhookRequest {
            callback_url: callback_url.to_string(),
            scope: CreateWebhookScope {
                folder: folder_urn.map(|s| s.to_string()),
                workflow: None,
            },
            hook_attribute: None,
            filter: None,
            hub_id: None,
            project_id: None,
            auto_reactivate_hook: Some(true),
        };

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&request)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let webhook: Webhook = response
            .json()
            .await
            .context("Failed to parse webhook response")?;

        Ok(webhook)
    }

    /// Get a specific webhook
    pub async fn get_webhook(&self, system: &str, event: &str, hook_id: &str) -> Result<Webhook> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/systems/{}/events/{}/hooks/{}",
            self.config.webhooks_url(),
            system,
            event,
            hook_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let webhook: Webhook = response
            .json()
            .await
            .context("Failed to parse webhook response")?;

        Ok(webhook)
    }

    /// Update a webhook
    pub async fn update_webhook(
        &self,
        system: &str,
        event: &str,
        hook_id: &str,
        request: UpdateWebhookRequest,
    ) -> Result<Webhook> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/systems/{}/events/{}/hooks/{}",
            self.config.webhooks_url(),
            system,
            event,
            hook_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .patch(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&request)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let webhook: Webhook = response
            .json()
            .await
            .context("Failed to parse webhook response")?;

        Ok(webhook)
    }

    /// Delete a webhook
    pub async fn delete_webhook(&self, system: &str, event: &str, hook_id: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/systems/{}/events/{}/hooks/{}",
            self.config.webhooks_url(),
            system,
            event,
            hook_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        Ok(())
    }

    /// Get available webhook events
    pub fn available_events(&self) -> &[(&str, &str)] {
        WEBHOOK_EVENTS
    }

    /// Check if an event type is a known webhook event
    pub fn is_valid_event(event: &str) -> bool {
        WEBHOOK_EVENTS.iter().any(|(e, _)| *e == event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_events_constant() {
        assert!(!WEBHOOK_EVENTS.is_empty());
        assert!(WEBHOOK_EVENTS.len() >= 10);

        // Check some expected events exist
        let events: Vec<&str> = WEBHOOK_EVENTS.iter().map(|(e, _)| *e).collect();
        assert!(events.contains(&"dm.version.added"));
        assert!(events.contains(&"dm.folder.added"));
        assert!(events.contains(&"extraction.finished"));
    }

    #[test]
    fn test_is_valid_event() {
        assert!(WebhooksClient::is_valid_event("dm.version.added"));
        assert!(WebhooksClient::is_valid_event("extraction.finished"));
        assert!(!WebhooksClient::is_valid_event("nonexistent.event"));
        assert!(!WebhooksClient::is_valid_event(""));
    }

    #[test]
    fn test_webhook_deserialization() {
        let json = r#"{
            "hookId": "hook-123",
            "callbackUrl": "https://example.com/webhook",
            "event": "dm.version.added",
            "system": "data",
            "status": "active"
        }"#;

        let webhook: Webhook = serde_json::from_str(json).unwrap();
        assert_eq!(webhook.hook_id, "hook-123");
        assert_eq!(webhook.callback_url, "https://example.com/webhook");
        assert_eq!(webhook.event, "dm.version.added");
        assert_eq!(webhook.status, "active");
    }

    #[test]
    fn test_webhook_with_scope_deserialization() {
        let json = r#"{
            "hookId": "hook-456",
            "callbackUrl": "https://example.com/webhook",
            "event": "dm.version.added",
            "system": "data",
            "status": "active",
            "scope": {
                "folder": "urn:adsk.wipprod:fs.folder:folder-id"
            }
        }"#;

        let webhook: Webhook = serde_json::from_str(json).unwrap();
        assert!(webhook.scope.is_some());
        let scope = webhook.scope.unwrap();
        assert!(scope.folder.is_some());
    }

    #[test]
    fn test_create_webhook_request_serialization() {
        let request = CreateWebhookRequest {
            callback_url: "https://example.com/callback".to_string(),
            scope: CreateWebhookScope {
                folder: Some("folder-urn".to_string()),
                workflow: None,
            },
            hook_attribute: None,
            filter: None,
            hub_id: None,
            project_id: None,
            auto_reactivate_hook: Some(true),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["callbackUrl"], "https://example.com/callback");
        assert_eq!(json["scope"]["folder"], "folder-urn");
        assert_eq!(json["autoReactivateHook"], true);
    }

    #[test]
    fn test_create_webhook_request_skips_none_fields() {
        let request = CreateWebhookRequest {
            callback_url: "https://example.com/callback".to_string(),
            scope: CreateWebhookScope {
                folder: None,
                workflow: None,
            },
            hook_attribute: None,
            filter: None,
            hub_id: None,
            project_id: None,
            auto_reactivate_hook: None,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("hookAttribute").is_none());
        assert!(json.get("filter").is_none());
        assert!(json.get("hubId").is_none());
    }

    #[test]
    fn test_webhooks_response_deserialization() {
        let json = r#"{
            "data": [
                {
                    "hookId": "hook-1",
                    "callbackUrl": "https://example.com/1",
                    "event": "dm.version.added",
                    "system": "data",
                    "status": "active"
                },
                {
                    "hookId": "hook-2",
                    "callbackUrl": "https://example.com/2",
                    "event": "dm.folder.added",
                    "system": "data",
                    "status": "inactive"
                }
            ],
            "links": {
                "next": "https://api.example.com/webhooks?page=2"
            }
        }"#;

        let response: WebhooksResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 2);
        assert!(response.links.is_some());
        assert!(response.links.unwrap().next.is_some());
    }

    #[test]
    fn test_update_webhook_request_serialization() {
        let request = UpdateWebhookRequest {
            callback_url: Some("https://new-url.com/hook".to_string()),
            status: Some("inactive".to_string()),
            filter: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("https://new-url.com/hook"));
        assert!(json.contains("inactive"));
        assert!(!json.contains("filter"));
    }

    #[test]
    fn test_update_webhook_request_status_only() {
        let request = UpdateWebhookRequest {
            callback_url: None,
            status: Some("active".to_string()),
            filter: None,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("active"));
        assert!(!json.contains("callbackUrl"));
    }

    // ==================== Contract Tests ====================

    #[test]
    fn test_contract_webhook() {
        let json = include_str!("../../tests/fixtures/webhook.json");
        let response: Webhook = serde_json::from_str(json).unwrap();
        insta::assert_debug_snapshot!(response);
    }

    #[test]
    fn test_contract_webhooks_response() {
        let json = include_str!("../../tests/fixtures/webhooks_response.json");
        let response: WebhooksResponse = serde_json::from_str(json).unwrap();
        insta::assert_debug_snapshot!(response);
    }
}

/// Integration tests using raps-mock
///
/// Create, list, and delete operations use the client API.
/// `get_webhook` and `update_webhook` routes are not registered in the mock,
/// so those operations still use raw HTTP to verify the request/response cycle.
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;

    fn create_mock_webhooks_client(mock_url: &str) -> WebhooksClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        WebhooksClient::new(config, auth)
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

    async fn setup_client_with_token(mock_url: &str) -> (WebhooksClient, String) {
        let client = create_mock_webhooks_client(mock_url);
        let token = acquire_mock_token(mock_url).await;
        client
            .auth
            .set_2leg_token_for_testing(token.clone(), 3600)
            .await;
        (client, token)
    }

    /// Create a webhook via raw HTTP and return the hook_id
    async fn create_webhook_raw(
        mock_url: &str,
        token: &str,
        event: &str,
        callback_url: &str,
    ) -> String {
        let http = reqwest::Client::new();
        let resp = http
            .post(format!(
                "{}/webhooks/v1/systems/data/events/{}/hooks",
                mock_url, event
            ))
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "callbackUrl": callback_url,
                "scope": { "folder": null },
                "autoReactivateHook": true
            }))
            .send()
            .await
            .expect("create webhook request failed");
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 201,
            "create webhook returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        body["hookId"]
            .as_str()
            .expect("response should contain hookId")
            .to_string()
    }

    #[tokio::test]
    async fn test_webhooks_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_webhooks_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }

    #[tokio::test]
    async fn test_create_webhook_client_method() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (client, _token) = setup_client_with_token(&server.url).await;

        let result = client
            .create_webhook(
                "data",
                "dm.version.added",
                "https://example.com/webhook",
                Some("urn:adsk.wipprod:fs.folder:test-folder"),
            )
            .await;
        assert!(
            result.is_ok(),
            "create_webhook failed: {:?}",
            result.err()
        );
        let webhook = result.unwrap();
        assert!(!webhook.hook_id.is_empty());
        assert_eq!(webhook.status, "active");
        assert_eq!(webhook.callback_url, "https://example.com/webhook");
    }

    #[tokio::test]
    async fn test_create_webhook_without_folder_client_method() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (client, _token) = setup_client_with_token(&server.url).await;

        let result = client
            .create_webhook(
                "data",
                "dm.folder.added",
                "https://example.com/folder-hook",
                None,
            )
            .await;
        assert!(
            result.is_ok(),
            "create_webhook without folder failed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_list_webhooks_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;

        // Create a webhook first
        create_webhook_raw(
            &server.url,
            &token,
            "dm.version.added",
            "https://example.com/list-test",
        )
        .await;

        let http = reqwest::Client::new();
        let resp = http
            .get(format!(
                "{}/webhooks/v1/systems/data/events/dm.version.added/hooks",
                server.url
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "list webhooks returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        let data = body["data"].as_array().expect("data should be an array");
        assert!(!data.is_empty(), "webhook list should not be empty after create");
        assert!(data[0]["hookId"].is_string(), "each webhook should have hookId");
    }

    #[tokio::test]
    async fn test_list_all_webhooks_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;

        create_webhook_raw(
            &server.url,
            &token,
            "dm.version.added",
            "https://example.com/all-test",
        )
        .await;

        let http = reqwest::Client::new();
        let resp = http
            .get(format!("{}/webhooks/v1/hooks", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "list all webhooks returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        let data = body["data"].as_array().expect("data should be an array");
        assert!(!data.is_empty(), "all-webhooks list should not be empty");
    }

    #[tokio::test]
    async fn test_delete_webhook_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;

        let hook_id = create_webhook_raw(
            &server.url,
            &token,
            "dm.version.added",
            "https://example.com/delete-test",
        )
        .await;

        let http = reqwest::Client::new();
        let resp = http
            .delete(format!(
                "{}/webhooks/v1/systems/data/events/dm.version.added/hooks/{}",
                server.url, hook_id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 204,
            "delete webhook returned {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_webhook_create_list_delete_lifecycle() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let token = acquire_mock_token(&server.url).await;
        let http = reqwest::Client::new();

        // Create
        let hook_id = create_webhook_raw(
            &server.url,
            &token,
            "dm.version.added",
            "https://example.com/lifecycle",
        )
        .await;
        assert!(!hook_id.is_empty());

        // List - should contain the created hook
        let list_resp = http
            .get(format!(
                "{}/webhooks/v1/systems/data/events/dm.version.added/hooks",
                server.url
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let list_body: serde_json::Value = list_resp.json().await.unwrap();
        let hooks = list_body["data"].as_array().unwrap();
        let has_hook = hooks
            .iter()
            .any(|h| h["hookId"].as_str() == Some(&hook_id));
        assert!(has_hook, "created hook should appear in list");

        // Delete
        let del_resp = http
            .delete(format!(
                "{}/webhooks/v1/systems/data/events/dm.version.added/hooks/{}",
                server.url, hook_id
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
    async fn test_list_webhooks_client_method_after_raw_create() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (client, token) = setup_client_with_token(&server.url).await;

        // Create via raw HTTP (since client create fails to parse response)
        create_webhook_raw(
            &server.url,
            &token,
            "dm.version.added",
            "https://example.com/client-list-test",
        )
        .await;

        // List via client method (mock list response has all required Webhook fields)
        let result = client.list_webhooks("data", "dm.version.added").await;
        assert!(result.is_ok(), "list_webhooks failed: {:?}", result.err());
        let webhooks = result.unwrap();
        assert!(!webhooks.is_empty(), "webhook list should not be empty");
        assert_eq!(webhooks[0].system, "data");
    }

    #[tokio::test]
    async fn test_list_all_webhooks_client_method_after_raw_create() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let (client, token) = setup_client_with_token(&server.url).await;

        create_webhook_raw(
            &server.url,
            &token,
            "dm.version.added",
            "https://example.com/client-all-test",
        )
        .await;

        let result = client.list_all_webhooks().await;
        assert!(result.is_ok(), "list_all_webhooks failed: {:?}", result.err());
        let webhooks = result.unwrap();
        assert!(!webhooks.is_empty(), "all-webhooks list should not be empty");
    }
}

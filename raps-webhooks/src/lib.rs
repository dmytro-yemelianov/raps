// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! Webhooks API module
//!
//! Handles webhook subscriptions for automated event notifications.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

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
        let http_client = http_config
            .create_client()
            .unwrap_or_else(|_| reqwest::Client::new()); // Fallback to default if config fails

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

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list webhooks")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list webhooks ({status}): {error_text}");
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

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list all webhooks")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list webhooks ({status}): {error_text}");
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

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to create webhook")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create webhook ({status}): {error_text}");
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

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to delete webhook")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete webhook ({status}): {error_text}");
        }

        Ok(())
    }

    /// Get available webhook events
    pub fn available_events(&self) -> &[(&str, &str)] {
        WEBHOOK_EVENTS
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
}

/// Integration tests using wiremock
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;
    use wiremock::matchers::{header, method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Create a Webhooks client configured to use the mock server
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

    /// Setup mock for 2-legged auth token
    async fn setup_auth_mock(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/authentication/v2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test-token-12345",
                "token_type": "Bearer",
                "expires_in": 3600
            })))
            .mount(server)
            .await;
    }

    // ==================== List Webhooks ====================

    #[tokio::test]
    async fn test_list_webhooks_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path("/webhooks/v1/systems/data/events/dm.version.added/hooks"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [
                    {
                        "hookId": "hook-123",
                        "callbackUrl": "https://example.com/webhook",
                        "event": "dm.version.added",
                        "system": "data",
                        "status": "active"
                    },
                    {
                        "hookId": "hook-456",
                        "callbackUrl": "https://example.com/webhook2",
                        "event": "dm.version.added",
                        "system": "data",
                        "status": "active"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client.list_webhooks("data", "dm.version.added").await;

        assert!(result.is_ok());
        let webhooks = result.unwrap();
        assert_eq!(webhooks.len(), 2);
        assert_eq!(webhooks[0].hook_id, "hook-123");
    }

    #[tokio::test]
    async fn test_list_webhooks_empty() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/webhooks/v1/systems/.+/events/.+/hooks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": []
            })))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client.list_webhooks("data", "dm.folder.added").await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_list_webhooks_unauthorized() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/webhooks/v1/systems/.+/events/.+/hooks"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "code": "Unauthorized",
                "message": "Invalid token"
            })))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client.list_webhooks("data", "dm.version.added").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("401"));
    }

    // ==================== List All Webhooks ====================

    #[tokio::test]
    async fn test_list_all_webhooks_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path("/webhooks/v1/hooks"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
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
                        "status": "active"
                    },
                    {
                        "hookId": "hook-3",
                        "callbackUrl": "https://example.com/3",
                        "event": "extraction.finished",
                        "system": "derivative",
                        "status": "inactive"
                    }
                ],
                "links": {
                    "next": null
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client.list_all_webhooks().await;

        assert!(result.is_ok());
        let webhooks = result.unwrap();
        assert_eq!(webhooks.len(), 3);
    }

    #[tokio::test]
    async fn test_list_all_webhooks_empty() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path("/webhooks/v1/hooks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": []
            })))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client.list_all_webhooks().await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ==================== Create Webhook ====================

    #[tokio::test]
    async fn test_create_webhook_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path("/webhooks/v1/systems/data/events/dm.version.added/hooks"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "hookId": "new-hook-789",
                "callbackUrl": "https://example.com/my-webhook",
                "event": "dm.version.added",
                "system": "data",
                "status": "active",
                "createdDate": "2024-01-15T10:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client
            .create_webhook(
                "data",
                "dm.version.added",
                "https://example.com/my-webhook",
                None,
            )
            .await;

        assert!(result.is_ok());
        let webhook = result.unwrap();
        assert_eq!(webhook.hook_id, "new-hook-789");
        assert_eq!(webhook.status, "active");
    }

    #[tokio::test]
    async fn test_create_webhook_with_folder_scope() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path_regex(r"/webhooks/v1/systems/.+/events/.+/hooks"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "hookId": "scoped-hook-123",
                "callbackUrl": "https://example.com/webhook",
                "event": "dm.version.added",
                "system": "data",
                "status": "active",
                "scope": {
                    "folder": "urn:adsk.wipprod:fs.folder:co.12345"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client
            .create_webhook(
                "data",
                "dm.version.added",
                "https://example.com/webhook",
                Some("urn:adsk.wipprod:fs.folder:co.12345"),
            )
            .await;

        assert!(result.is_ok());
        let webhook = result.unwrap();
        assert!(webhook.scope.is_some());
    }

    #[tokio::test]
    async fn test_create_webhook_invalid_callback() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path_regex(r"/webhooks/v1/systems/.+/events/.+/hooks"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "code": "BadRequest",
                "message": "Invalid callback URL"
            })))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client
            .create_webhook("data", "dm.version.added", "invalid-url", None)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("400"));
    }

    #[tokio::test]
    async fn test_create_webhook_invalid_event() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path_regex(r"/webhooks/v1/systems/.+/events/.+/hooks"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "code": "BadRequest",
                "message": "Invalid event type"
            })))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client
            .create_webhook(
                "data",
                "invalid.event",
                "https://example.com/webhook",
                None,
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("400"));
    }

    // ==================== Delete Webhook ====================

    #[tokio::test]
    async fn test_delete_webhook_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("DELETE"))
            .and(path_regex(
                r"/webhooks/v1/systems/data/events/dm.version.added/hooks/.+",
            ))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client
            .delete_webhook("data", "dm.version.added", "hook-123")
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_webhook_not_found() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("DELETE"))
            .and(path_regex(r"/webhooks/v1/systems/.+/events/.+/hooks/.+"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "code": "NotFound",
                "message": "Webhook not found"
            })))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client
            .delete_webhook("data", "dm.version.added", "nonexistent")
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("404"));
    }

    // ==================== Error Handling ====================

    #[tokio::test]
    async fn test_server_error() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path("/webhooks/v1/hooks"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "code": "InternalError",
                "message": "Internal server error"
            })))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client.list_all_webhooks().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("500"));
    }

    #[tokio::test]
    async fn test_rate_limit() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path_regex(r"/webhooks/v1/systems/.+/events/.+/hooks"))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
                "code": "RateLimited",
                "message": "Too many requests"
            })))
            .mount(&server)
            .await;

        let client = create_mock_webhooks_client(&server.uri());
        let result = client
            .create_webhook(
                "data",
                "dm.version.added",
                "https://example.com/webhook",
                None,
            )
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("429"));
    }
}

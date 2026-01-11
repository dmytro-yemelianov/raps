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

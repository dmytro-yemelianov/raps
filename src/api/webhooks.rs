//! Webhooks API module
//!
//! Handles webhook subscriptions for automated event notifications.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::AuthClient;
use crate::config::Config;

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
        Self::new_with_http_config(config, auth, crate::http::HttpClientConfig::default())
    }

    /// Create a new Webhooks client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: crate::http::HttpClientConfig,
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
            anyhow::bail!("Failed to list webhooks ({}): {}", status, error_text);
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
            anyhow::bail!("Failed to list webhooks ({}): {}", status, error_text);
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
            anyhow::bail!("Failed to create webhook ({}): {}", status, error_text);
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
            anyhow::bail!("Failed to delete webhook ({}): {}", status, error_text);
        }

        Ok(())
    }

    /// Get available webhook events
    pub fn available_events(&self) -> &[(&str, &str)] {
        WEBHOOK_EVENTS
    }
}

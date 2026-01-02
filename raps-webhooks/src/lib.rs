// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Webhooks API
//!
//! Manage webhook subscriptions for APS events.

use raps_kernel::{AuthClient, Config, HttpClient, RapsError, Result};
use serde::{Deserialize, Serialize};

/// Webhook subscription
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Webhook {
    /// Webhook ID (hook ID)
    pub hook_id: String,
    /// Callback URL
    pub callback_url: String,
    /// Scope (e.g., "folder")
    pub scope: String,
    /// Event type
    pub event: String,
    /// Active status
    pub status: String,
}

/// Webhooks client
pub struct WebhooksClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
}

impl WebhooksClient {
    /// Create a new webhooks client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config) -> Self {
        Self { http, auth, config }
    }

    /// List all webhook subscriptions
    pub async fn list(&self, system: &str, event: &str) -> Result<Vec<Webhook>> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/webhooks/v1/systems/{}/events/{}/hooks",
            system, event
        );

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to list webhooks".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to list webhooks ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        #[derive(Deserialize)]
        struct WebhooksResponse {
            data: Vec<Webhook>,
        }

        let resp: WebhooksResponse = response.json().await.map_err(|e| RapsError::Internal {
            message: format!("Failed to parse webhooks response: {}", e),
        })?;

        Ok(resp.data)
    }
}

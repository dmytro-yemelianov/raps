// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Hub operations

use raps_kernel::{AuthClient, Config, HttpClient, Result, RapsError};
use crate::types::*;

/// Hub client for Data Management operations
pub struct HubClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
    project_url: String,
}

impl HubClient {
    /// Create new hub client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config, project_url: String) -> Self {
        Self {
            http,
            auth,
            config,
            project_url,
        }
    }

    /// List all accessible hubs (requires 3-legged auth)
    pub async fn list_hubs(&self) -> Result<Vec<Hub>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/hubs", self.project_url);

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to list hubs".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to list hubs ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let api_response: JsonApiResponse<Vec<Hub>> = response
            .json()
            .await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse hubs response: {}", e),
            })?;

        Ok(api_response.data)
    }

    /// Get hub details
    pub async fn get_hub(&self, hub_id: &str) -> Result<Hub> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/hubs/{}", self.project_url, hub_id);

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to get hub".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to get hub ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let api_response: JsonApiResponse<Hub> = response
            .json()
            .await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse hub response: {}", e),
            })?;

        Ok(api_response.data)
    }
}

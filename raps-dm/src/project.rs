// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Project operations

use crate::types::*;
use raps_kernel::{AuthClient, Config, HttpClient, RapsError, Result};

/// Project client for Data Management operations
pub struct ProjectClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
    project_url: String,
}

impl ProjectClient {
    /// Create new project client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config, project_url: String) -> Self {
        Self {
            http,
            auth,
            config,
            project_url,
        }
    }

    /// List projects in a hub
    pub async fn list_projects(&self, hub_id: &str) -> Result<Vec<Project>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/hubs/{}/projects", self.project_url, hub_id);

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to list projects".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to list projects ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let api_response: JsonApiResponse<Vec<Project>> =
            response.json().await.map_err(|e| RapsError::Internal {
                message: format!("Failed to parse projects response: {}", e),
            })?;

        Ok(api_response.data)
    }

    /// Get project details
    pub async fn get_project(&self, hub_id: &str, project_id: &str) -> Result<Project> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/hubs/{}/projects/{}",
            self.project_url, hub_id, project_id
        );

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to get project".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to get project ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let api_response: JsonApiResponse<Project> =
            response.json().await.map_err(|e| RapsError::Internal {
                message: format!("Failed to parse project response: {}", e),
            })?;

        Ok(api_response.data)
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC RFI (Request for Information) API

use raps_kernel::{AuthClient, Config, HttpClient, RapsError, Result};
use serde::{Deserialize, Serialize};

/// RFI information
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rfi {
    /// Unique RFI ID
    pub id: String,
    /// RFI number
    pub number: Option<String>,
    /// RFI title/subject
    pub title: String,
    /// Question/description
    pub question: Option<String>,
    /// Current status
    pub status: String,
    /// Due date
    pub due_date: Option<String>,
    /// Created timestamp
    pub created_at: Option<String>,
    /// Updated timestamp
    pub updated_at: Option<String>,
}

/// RFI client for ACC RFI API
pub struct RfiClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
}

impl RfiClient {
    /// Create a new RFI client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config) -> Self {
        Self { http, auth, config }
    }

    /// List RFIs in a project
    pub async fn list(&self, project_id: &str) -> Result<Vec<Rfi>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/rfis/v1/projects/{}/rfis",
            project_id
        );

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to list RFIs".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to list RFIs ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        #[derive(Deserialize)]
        struct RfiResponse {
            results: Vec<Rfi>,
        }

        let resp: RfiResponse = response.json().await.map_err(|e| RapsError::Internal {
            message: format!("Failed to parse RFI response: {}", e),
        })?;

        Ok(resp.results)
    }

    /// Get a specific RFI
    pub async fn get(&self, project_id: &str, rfi_id: &str) -> Result<Rfi> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/rfis/v1/projects/{}/rfis/{}",
            project_id, rfi_id
        );

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to get RFI".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to get RFI ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let rfi: Rfi = response.json().await.map_err(|e| RapsError::Internal {
            message: format!("Failed to parse RFI response: {}", e),
        })?;

        Ok(rfi)
    }
}

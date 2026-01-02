// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC Submittals API

use raps_kernel::{AuthClient, Config, HttpClient, Result, RapsError};
use serde::{Deserialize, Serialize};

/// Submittal information
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Submittal {
    /// Unique submittal ID
    pub id: String,
    /// Submittal title
    pub title: String,
    /// Submittal number
    pub number: Option<String>,
    /// Current status
    pub status: String,
    /// Spec section
    pub spec_section: Option<String>,
    /// Due date
    pub due_date: Option<String>,
    /// Created timestamp
    pub created_at: Option<String>,
    /// Updated timestamp
    pub updated_at: Option<String>,
}

/// Submittals client for ACC Submittals API
pub struct SubmittalsClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
}

impl SubmittalsClient {
    /// Create a new submittals client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config) -> Self {
        Self { http, auth, config }
    }

    /// List submittals in a project
    pub async fn list(&self, project_id: &str) -> Result<Vec<Submittal>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/submittals/v1/projects/{}/submittals",
            project_id
        );

        let response = self.http.inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to list submittals".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to list submittals ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        #[derive(Deserialize)]
        struct SubmittalsResponse {
            results: Vec<Submittal>,
        }

        let resp: SubmittalsResponse = response.json().await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse submittals response: {}", e),
            })?;

        Ok(resp.results)
    }
}

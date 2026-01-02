// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC Checklists API

use raps_kernel::{AuthClient, Config, HttpClient, Result, RapsError};
use serde::{Deserialize, Serialize};

/// Checklist information
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checklist {
    /// Unique checklist ID
    pub id: String,
    /// Template ID
    pub template_id: Option<String>,
    /// Checklist title
    pub title: String,
    /// Current status
    pub status: String,
    /// Assignee ID
    pub assignee_id: Option<String>,
    /// Location
    pub location: Option<String>,
    /// Due date
    pub due_date: Option<String>,
    /// Created timestamp
    pub created_at: Option<String>,
    /// Updated timestamp
    pub updated_at: Option<String>,
}

/// Checklists client for ACC Checklists API
pub struct ChecklistsClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
}

impl ChecklistsClient {
    /// Create a new checklists client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config) -> Self {
        Self { http, auth, config }
    }

    /// List checklists in a project
    pub async fn list(&self, project_id: &str) -> Result<Vec<Checklist>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/checklists/v1/projects/{}/checklists",
            project_id
        );

        let response = self.http.inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to list checklists".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to list checklists ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        #[derive(Deserialize)]
        struct ChecklistsResponse {
            results: Vec<Checklist>,
        }

        let resp: ChecklistsResponse = response.json().await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse checklists response: {}", e),
            })?;

        Ok(resp.results)
    }
}

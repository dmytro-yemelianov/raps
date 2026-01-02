// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC Issues API

use raps_kernel::{AuthClient, Config, HttpClient, Result, RapsError};
use serde::{Deserialize, Serialize};

/// Issue information
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// Unique issue ID
    pub id: String,
    /// Display ID (human-readable number)
    pub display_id: Option<i32>,
    /// Issue title
    pub title: String,
    /// Issue description
    pub description: Option<String>,
    /// Current status
    pub status: String,
    /// Issue type ID
    pub issue_type_id: Option<String>,
    /// Assigned user ID
    pub assigned_to: Option<String>,
    /// Due date
    pub due_date: Option<String>,
    /// Created timestamp
    pub created_at: Option<String>,
    /// Updated timestamp
    pub updated_at: Option<String>,
}

/// Issues client for ACC Issues API
pub struct IssuesClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
}

impl IssuesClient {
    /// Create a new issues client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config) -> Self {
        Self { http, auth, config }
    }

    /// List issues in a project
    pub async fn list(&self, project_id: &str) -> Result<Vec<Issue>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/issues/v1/projects/{}/issues",
            project_id
        );

        let response = self.http.inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to list issues".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to list issues ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        #[derive(Deserialize)]
        struct IssuesResponse {
            results: Vec<Issue>,
        }

        let resp: IssuesResponse = response.json().await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse issues response: {}", e),
            })?;

        Ok(resp.results)
    }

    /// Get a specific issue
    pub async fn get(&self, project_id: &str, issue_id: &str) -> Result<Issue> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/issues/v1/projects/{}/issues/{}",
            project_id, issue_id
        );

        let response = self.http.inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to get issue".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to get issue ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let issue: Issue = response.json().await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse issue response: {}", e),
            })?;

        Ok(issue)
    }
}

//! ACC RFI (Request for Information) API module
//!
//! Handles RFIs in ACC (Autodesk Construction Cloud) projects.
//! Uses the Construction RFIs API.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::AuthClient;
use crate::config::Config;

/// RFI information
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rfi {
    pub id: String,
    pub title: String,
    pub number: Option<String>,
    pub status: String,
    pub priority: Option<String>,
    pub question: Option<String>,
    pub answer: Option<String>,
    pub due_date: Option<String>,
    pub assigned_to: Option<String>,
    pub assigned_to_name: Option<String>,
    pub created_by: Option<String>,
    pub created_by_name: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub closed_at: Option<String>,
    pub location: Option<String>,
    pub discipline: Option<String>,
}

/// RFIs response with pagination
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RfisResponse {
    pub results: Vec<Rfi>,
    #[allow(dead_code)]
    pub pagination: Option<Pagination>,
}

/// Pagination information
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Pagination {
    pub limit: i32,
    pub offset: i32,
    pub total_results: i32,
}

/// RFI status values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RfiStatus {
    Draft,
    Open,
    Answered,
    Closed,
    Void,
}

impl RfiStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RfiStatus::Draft => "draft",
            RfiStatus::Open => "open",
            RfiStatus::Answered => "answered",
            RfiStatus::Closed => "closed",
            RfiStatus::Void => "void",
        }
    }
}

impl std::fmt::Display for RfiStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// RFI priority values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RfiPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl RfiPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            RfiPriority::Low => "low",
            RfiPriority::Normal => "normal",
            RfiPriority::High => "high",
            RfiPriority::Critical => "critical",
        }
    }
}

impl std::fmt::Display for RfiPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Request body for creating an RFI
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRfiRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discipline: Option<String>,
}

/// Request body for updating an RFI
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRfiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// RFI API client
pub struct RfiClient {
    #[allow(dead_code)]
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl RfiClient {
    /// Create a new RFI client
    #[allow(dead_code)]
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, crate::http::HttpClientConfig::default())
    }

    /// Create a new RFI client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: crate::http::HttpClientConfig,
    ) -> Self {
        let http_client = http_config
            .create_client()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            auth,
            http_client,
        }
    }

    /// List RFIs in a project
    pub async fn list_rfis(&self, project_id: &str) -> Result<Vec<Rfi>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/rfis/v2/projects/{}/rfis",
            project_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list RFIs")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list RFIs ({}): {}", status, error_text);
        }

        let rfis_response: RfisResponse = response
            .json()
            .await
            .context("Failed to parse RFIs response")?;

        Ok(rfis_response.results)
    }

    /// Get a specific RFI by ID
    pub async fn get_rfi(&self, project_id: &str, rfi_id: &str) -> Result<Rfi> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/rfis/v2/projects/{}/rfis/{}",
            project_id, rfi_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get RFI")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get RFI ({}): {}", status, error_text);
        }

        let rfi: Rfi = response.json().await.context("Failed to parse RFI response")?;
        Ok(rfi)
    }

    /// Create a new RFI
    pub async fn create_rfi(&self, project_id: &str, request: CreateRfiRequest) -> Result<Rfi> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/rfis/v2/projects/{}/rfis",
            project_id
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await
            .context("Failed to create RFI")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create RFI ({}): {}", status, error_text);
        }

        let rfi: Rfi = response.json().await.context("Failed to parse RFI response")?;
        Ok(rfi)
    }

    /// Update an existing RFI
    pub async fn update_rfi(
        &self,
        project_id: &str,
        rfi_id: &str,
        request: UpdateRfiRequest,
    ) -> Result<Rfi> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/rfis/v2/projects/{}/rfis/{}",
            project_id, rfi_id
        );

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await
            .context("Failed to update RFI")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update RFI ({}): {}", status, error_text);
        }

        let rfi: Rfi = response.json().await.context("Failed to parse RFI response")?;
        Ok(rfi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfi_status_display() {
        assert_eq!(RfiStatus::Open.as_str(), "open");
        assert_eq!(RfiStatus::Closed.as_str(), "closed");
        assert_eq!(RfiStatus::Answered.as_str(), "answered");
    }

    #[test]
    fn test_rfi_priority_display() {
        assert_eq!(RfiPriority::High.as_str(), "high");
        assert_eq!(RfiPriority::Critical.as_str(), "critical");
    }

    #[test]
    fn test_create_rfi_request_serialization() {
        let request = CreateRfiRequest {
            title: "Test RFI".to_string(),
            question: Some("What is the answer?".to_string()),
            priority: Some("high".to_string()),
            due_date: None,
            assigned_to: None,
            location: None,
            discipline: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Test RFI"));
        assert!(json.contains("high"));
        // None fields should be skipped
        assert!(!json.contains("dueDate"));
    }

    #[test]
    fn test_update_rfi_request_serialization() {
        let request = UpdateRfiRequest {
            status: Some("closed".to_string()),
            answer: Some("The answer is 42".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("closed"));
        assert!(json.contains("42"));
        // None fields should be skipped
        assert!(!json.contains("title"));
    }
}


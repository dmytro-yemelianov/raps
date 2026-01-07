// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC Issues API module
//!
//! Handles issues and RFIs in ACC (Autodesk Construction Cloud) projects.
//! Uses the Construction Issues API v1: /construction/issues/v1

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::AuthClient;
use crate::config::Config;

/// Issue information
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub id: String,
    #[serde(default)]
    pub container_id: Option<String>,
    pub display_id: Option<i32>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub issue_type_id: Option<String>,
    pub issue_subtype_id: Option<String>,
    pub assigned_to: Option<String>,
    pub assigned_to_type: Option<String>,
    pub due_date: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub created_by: Option<String>,
    pub closed_at: Option<String>,
    pub closed_by: Option<String>,
}

/// Issue type (category)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueType {
    pub id: String,
    pub title: String,
    pub is_active: Option<bool>,
    pub subtypes: Option<Vec<IssueSubType>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueSubType {
    pub id: String,
    pub title: String,
    pub is_active: Option<bool>,
}

/// Issues response
#[derive(Debug, Deserialize)]
pub struct IssuesResponse {
    pub results: Vec<Issue>,
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub limit: i32,
    pub offset: i32,
    pub total_results: i32,
}

/// Issue types response
#[derive(Debug, Deserialize)]
pub struct IssueTypesResponse {
    pub results: Vec<IssueType>,
}

/// Request to create an issue
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIssueRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_subtype_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
}

/// Request to update an issue
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateIssueRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
}

/// Issue comment
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueComment {
    pub id: String,
    pub body: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub created_by: Option<String>,
}

/// Comments response
#[derive(Debug, Deserialize)]
pub struct CommentsResponse {
    pub results: Vec<IssueComment>,
    pub pagination: Option<Pagination>,
}

/// Request to create a comment
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCommentRequest {
    pub body: String,
}

/// Issue attachment
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueAttachment {
    pub id: String,
    pub name: String,
    pub urn: Option<String>,
    pub url: Option<String>,
    pub created_at: Option<String>,
    pub created_by: Option<String>,
}

/// Attachments response
#[derive(Debug, Deserialize)]
pub struct AttachmentsResponse {
    pub results: Vec<IssueAttachment>,
    pub pagination: Option<Pagination>,
}

/// Issues API client
pub struct IssuesClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl IssuesClient {
    /// Create a new Issues client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, crate::http::HttpClientConfig::default())
    }

    /// Create a new Issues client with custom HTTP config
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

    /// List issues in a project
    ///
    /// Note: project_id should NOT include the "b." prefix used by Data Management API
    pub async fn list_issues(&self, project_id: &str, filter: Option<&str>) -> Result<Vec<Issue>> {
        let token = self.auth.get_3leg_token().await?;
        let mut url = format!(
            "{}/projects/{}/issues",
            self.config.issues_url(),
            project_id
        );

        if let Some(f) = filter {
            url = format!("{}?filter[{}]", url, f);
        }

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list issues")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list issues ({}): {}", status, error_text);
        }

        let issues_response: IssuesResponse = response
            .json()
            .await
            .context("Failed to parse issues response")?;

        Ok(issues_response.results)
    }

    /// Get issue details
    pub async fn get_issue(&self, project_id: &str, issue_id: &str) -> Result<Issue> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/issues/{}",
            self.config.issues_url(),
            project_id,
            issue_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get issue")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get issue ({}): {}", status, error_text);
        }

        let issue: Issue = response
            .json()
            .await
            .context("Failed to parse issue response")?;

        Ok(issue)
    }

    /// Create a new issue
    pub async fn create_issue(
        &self,
        project_id: &str,
        request: CreateIssueRequest,
    ) -> Result<Issue> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/issues",
            self.config.issues_url(),
            project_id
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to create issue")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create issue ({}): {}", status, error_text);
        }

        let issue: Issue = response
            .json()
            .await
            .context("Failed to parse issue response")?;

        Ok(issue)
    }

    /// Update an issue
    pub async fn update_issue(
        &self,
        project_id: &str,
        issue_id: &str,
        request: UpdateIssueRequest,
    ) -> Result<Issue> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/issues/{}",
            self.config.issues_url(),
            project_id,
            issue_id
        );

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to update issue")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update issue ({}): {}", status, error_text);
        }

        let issue: Issue = response
            .json()
            .await
            .context("Failed to parse issue response")?;

        Ok(issue)
    }

    /// List issue types (categories) for a project
    pub async fn list_issue_types(&self, project_id: &str) -> Result<Vec<IssueType>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/issue-types?include=subtypes",
            self.config.issues_url(),
            project_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list issue types")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list issue types ({}): {}", status, error_text);
        }

        let types_response: IssueTypesResponse = response
            .json()
            .await
            .context("Failed to parse issue types response")?;

        Ok(types_response.results)
    }

    // ============== COMMENTS ==============

    /// List comments for an issue
    pub async fn list_comments(
        &self,
        project_id: &str,
        issue_id: &str,
    ) -> Result<Vec<IssueComment>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/issues/{}/comments",
            self.config.issues_url(),
            project_id,
            issue_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list comments")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list comments ({}): {}", status, error_text);
        }

        let comments_response: CommentsResponse = response
            .json()
            .await
            .context("Failed to parse comments response")?;

        Ok(comments_response.results)
    }

    /// Add a comment to an issue
    pub async fn add_comment(
        &self,
        project_id: &str,
        issue_id: &str,
        body: &str,
    ) -> Result<IssueComment> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/issues/{}/comments",
            self.config.issues_url(),
            project_id,
            issue_id
        );

        let request = CreateCommentRequest {
            body: body.to_string(),
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to add comment")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to add comment ({}): {}", status, error_text);
        }

        let comment: IssueComment = response
            .json()
            .await
            .context("Failed to parse comment response")?;

        Ok(comment)
    }

    /// Delete a comment from an issue
    pub async fn delete_comment(
        &self,
        project_id: &str,
        issue_id: &str,
        comment_id: &str,
    ) -> Result<()> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/issues/{}/comments/{}",
            self.config.issues_url(),
            project_id,
            issue_id,
            comment_id
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to delete comment")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete comment ({}): {}", status, error_text);
        }

        Ok(())
    }

    // ============== ATTACHMENTS ==============

    /// List attachments for an issue
    pub async fn list_attachments(
        &self,
        project_id: &str,
        issue_id: &str,
    ) -> Result<Vec<IssueAttachment>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/issues/{}/attachments",
            self.config.issues_url(),
            project_id,
            issue_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list attachments")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list attachments ({}): {}", status, error_text);
        }

        let attachments_response: AttachmentsResponse = response
            .json()
            .await
            .context("Failed to parse attachments response")?;

        Ok(attachments_response.results)
    }
}

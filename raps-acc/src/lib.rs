// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC/BIM 360 API module
//!
//! This crate provides clients for ACC (Autodesk Construction Cloud) APIs:
//! - Issues - Construction Issues management
//! - RFI - Request for Information management
//! - Extended APIs - Assets, Submittals, Checklists

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

// ============================================================================
// ISSUES API
// ============================================================================

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
    pub pagination: Option<IssuesPagination>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuesPagination {
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
    pub pagination: Option<IssuesPagination>,
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
    pub pagination: Option<IssuesPagination>,
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
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new Issues client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
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
            anyhow::bail!("Failed to list issues ({status}): {error_text}");
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
            anyhow::bail!("Failed to get issue ({status}): {error_text}");
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
            anyhow::bail!("Failed to create issue ({status}): {error_text}");
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
            anyhow::bail!("Failed to update issue ({status}): {error_text}");
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
            anyhow::bail!("Failed to list issue types ({status}): {error_text}");
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
            anyhow::bail!("Failed to list comments ({status}): {error_text}");
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
            anyhow::bail!("Failed to add comment ({status}): {error_text}");
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
            anyhow::bail!("Failed to delete comment ({status}): {error_text}");
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
            anyhow::bail!("Failed to list attachments ({status}): {error_text}");
        }

        let attachments_response: AttachmentsResponse = response
            .json()
            .await
            .context("Failed to parse attachments response")?;

        Ok(attachments_response.results)
    }
}

// ============================================================================
// RFI API
// ============================================================================

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
    pub pagination: Option<RfiPagination>,
}

/// Pagination information
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct RfiPagination {
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
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new RFI client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
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
            "{}/projects/{}/rfis",
            self.config.rfi_url(),
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
            anyhow::bail!("Failed to list RFIs ({status}): {error_text}");
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
            "{}/projects/{}/rfis/{}",
            self.config.rfi_url(),
            project_id,
            rfi_id
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
            anyhow::bail!("Failed to get RFI ({status}): {error_text}");
        }

        let rfi: Rfi = response
            .json()
            .await
            .context("Failed to parse RFI response")?;
        Ok(rfi)
    }

    /// Create a new RFI
    pub async fn create_rfi(&self, project_id: &str, request: CreateRfiRequest) -> Result<Rfi> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/rfis",
            self.config.rfi_url(),
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
            anyhow::bail!("Failed to create RFI ({status}): {error_text}");
        }

        let rfi: Rfi = response
            .json()
            .await
            .context("Failed to parse RFI response")?;
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
            "{}/projects/{}/rfis/{}",
            self.config.rfi_url(),
            project_id,
            rfi_id
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
            anyhow::bail!("Failed to update RFI ({status}): {error_text}");
        }

        let rfi: Rfi = response
            .json()
            .await
            .context("Failed to parse RFI response")?;
        Ok(rfi)
    }
}

// ============================================================================
// ACC EXTENDED API (Assets, Submittals, Checklists)
// ============================================================================

/// ACC Asset information
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub category_id: Option<String>,
    pub status_id: Option<String>,
    pub client_asset_id: Option<String>,
    pub description: Option<String>,
    pub barcode: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Assets response
#[derive(Debug, Deserialize)]
pub struct AssetsResponse {
    pub results: Vec<Asset>,
    pub pagination: Option<Pagination>,
}

/// ACC Submittal information
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Submittal {
    pub id: String,
    pub title: String,
    pub number: Option<String>,
    pub status: String,
    pub spec_section: Option<String>,
    pub due_date: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Submittals response
#[derive(Debug, Deserialize)]
pub struct SubmittalsResponse {
    pub results: Vec<Submittal>,
    pub pagination: Option<Pagination>,
}

/// ACC Checklist template
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistTemplate {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: Option<String>,
}

/// ACC Checklist instance
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checklist {
    pub id: String,
    pub template_id: Option<String>,
    pub title: String,
    pub status: String,
    pub assignee_id: Option<String>,
    pub location: Option<String>,
    pub due_date: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Checklists response
#[derive(Debug, Deserialize)]
pub struct ChecklistsResponse {
    pub results: Vec<Checklist>,
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub limit: i32,
    pub offset: i32,
    pub total_results: i32,
}

/// ACC Extended API client
pub struct AccClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl AccClient {
    /// Create a new ACC client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new ACC client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
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

    // ============== ASSETS ==============

    /// List assets in a project
    pub async fn list_assets(&self, project_id: &str) -> Result<Vec<Asset>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/assets",
            self.config.assets_url(),
            project_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list assets")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list assets ({status}): {error_text}");
        }

        let assets_response: AssetsResponse = response
            .json()
            .await
            .context("Failed to parse assets response")?;

        Ok(assets_response.results)
    }

    /// Get a specific asset by ID
    pub async fn get_asset(&self, project_id: &str, asset_id: &str) -> Result<Asset> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/assets/{}",
            self.config.assets_url(),
            project_id,
            asset_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get asset")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get asset ({status}): {error_text}");
        }

        let asset: Asset = response
            .json()
            .await
            .context("Failed to parse asset response")?;
        Ok(asset)
    }

    /// Create a new asset
    pub async fn create_asset(
        &self,
        project_id: &str,
        request: CreateAssetRequest,
    ) -> Result<Asset> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/assets",
            self.config.assets_url(),
            project_id
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await
            .context("Failed to create asset")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create asset ({status}): {error_text}");
        }

        let asset: Asset = response
            .json()
            .await
            .context("Failed to parse asset response")?;
        Ok(asset)
    }

    /// Update an existing asset
    pub async fn update_asset(
        &self,
        project_id: &str,
        asset_id: &str,
        request: UpdateAssetRequest,
    ) -> Result<Asset> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/assets/{}",
            self.config.assets_url(),
            project_id,
            asset_id
        );

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await
            .context("Failed to update asset")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update asset ({status}): {error_text}");
        }

        let asset: Asset = response
            .json()
            .await
            .context("Failed to parse asset response")?;
        Ok(asset)
    }

    // ============== SUBMITTALS ==============

    /// List submittals in a project
    pub async fn list_submittals(&self, project_id: &str) -> Result<Vec<Submittal>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items",
            self.config.submittals_url(),
            project_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list submittals")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list submittals ({status}): {error_text}");
        }

        let submittals_response: SubmittalsResponse = response
            .json()
            .await
            .context("Failed to parse submittals response")?;

        Ok(submittals_response.results)
    }

    /// Get a specific submittal by ID
    pub async fn get_submittal(&self, project_id: &str, submittal_id: &str) -> Result<Submittal> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}",
            self.config.submittals_url(),
            project_id,
            submittal_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get submittal")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get submittal ({status}): {error_text}");
        }

        let submittal: Submittal = response
            .json()
            .await
            .context("Failed to parse submittal response")?;
        Ok(submittal)
    }

    /// Create a new submittal
    pub async fn create_submittal(
        &self,
        project_id: &str,
        request: CreateSubmittalRequest,
    ) -> Result<Submittal> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items",
            self.config.submittals_url(),
            project_id
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await
            .context("Failed to create submittal")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create submittal ({status}): {error_text}");
        }

        let submittal: Submittal = response
            .json()
            .await
            .context("Failed to parse submittal response")?;
        Ok(submittal)
    }

    /// Update an existing submittal
    pub async fn update_submittal(
        &self,
        project_id: &str,
        submittal_id: &str,
        request: UpdateSubmittalRequest,
    ) -> Result<Submittal> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}",
            self.config.submittals_url(),
            project_id,
            submittal_id
        );

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await
            .context("Failed to update submittal")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update submittal ({status}): {error_text}");
        }

        let submittal: Submittal = response
            .json()
            .await
            .context("Failed to parse submittal response")?;
        Ok(submittal)
    }

    // ============== CHECKLISTS ==============

    /// List checklists in a project
    pub async fn list_checklists(&self, project_id: &str) -> Result<Vec<Checklist>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/checklists",
            self.config.checklists_url(),
            project_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list checklists")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list checklists ({status}): {error_text}");
        }

        let checklists_response: ChecklistsResponse = response
            .json()
            .await
            .context("Failed to parse checklists response")?;

        Ok(checklists_response.results)
    }

    /// List checklist templates in a project
    pub async fn list_checklist_templates(
        &self,
        project_id: &str,
    ) -> Result<Vec<ChecklistTemplate>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/templates",
            self.config.checklists_url(),
            project_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list checklist templates")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to list checklist templates ({}): {}",
                status,
                error_text
            );
        }

        #[derive(Deserialize)]
        struct TemplatesResponse {
            results: Vec<ChecklistTemplate>,
        }

        let templates_response: TemplatesResponse = response
            .json()
            .await
            .context("Failed to parse checklist templates response")?;

        Ok(templates_response.results)
    }

    /// Get a specific checklist by ID
    pub async fn get_checklist(&self, project_id: &str, checklist_id: &str) -> Result<Checklist> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/checklists/{}",
            self.config.checklists_url(),
            project_id,
            checklist_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get checklist")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get checklist ({status}): {error_text}");
        }

        let checklist: Checklist = response
            .json()
            .await
            .context("Failed to parse checklist response")?;
        Ok(checklist)
    }

    /// Create a new checklist
    pub async fn create_checklist(
        &self,
        project_id: &str,
        request: CreateChecklistRequest,
    ) -> Result<Checklist> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/checklists",
            self.config.checklists_url(),
            project_id
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await
            .context("Failed to create checklist")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create checklist ({status}): {error_text}");
        }

        let checklist: Checklist = response
            .json()
            .await
            .context("Failed to parse checklist response")?;
        Ok(checklist)
    }

    /// Update an existing checklist
    pub async fn update_checklist(
        &self,
        project_id: &str,
        checklist_id: &str,
        request: UpdateChecklistRequest,
    ) -> Result<Checklist> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/checklists/{}",
            self.config.checklists_url(),
            project_id,
            checklist_id
        );

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&token)
            .json(&request)
            .send()
            .await
            .context("Failed to update checklist")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update checklist ({status}): {error_text}");
        }

        let checklist: Checklist = response
            .json()
            .await
            .context("Failed to parse checklist response")?;
        Ok(checklist)
    }
}

// ============== REQUEST TYPES ==============

/// Request body for creating an asset
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssetRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barcode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_asset_id: Option<String>,
}

/// Request body for updating an asset
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssetRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barcode: Option<String>,
}

/// Request body for creating a submittal
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubmittalRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
}

/// Request body for updating a submittal
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubmittalRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
}

/// Request body for creating a checklist
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChecklistRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_id: Option<String>,
}

/// Request body for updating a checklist
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateChecklistRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_id: Option<String>,
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

/// Integration tests using wiremock
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;
    use raps_kernel::types::StoredToken;
    use wiremock::matchers::{header, method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Create a valid test token that won't expire during tests
    fn create_test_token() -> StoredToken {
        StoredToken {
            access_token: "test-3leg-token-12345".to_string(),
            refresh_token: Some("test-refresh-token".to_string()),
            expires_at: chrono::Utc::now().timestamp() + 3600, // 1 hour from now
            scopes: vec!["data:read".to_string(), "data:write".to_string()],
        }
    }

    /// Create an Issues client configured to use the mock server with pre-set token
    async fn create_mock_issues_client(mock_url: &str) -> IssuesClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        auth.set_3leg_token_for_testing(create_test_token()).await;
        IssuesClient::new(config, auth)
    }

    /// Create an RFI client configured to use the mock server with pre-set token
    async fn create_mock_rfi_client(mock_url: &str) -> RfiClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        auth.set_3leg_token_for_testing(create_test_token()).await;
        RfiClient::new(config, auth)
    }

    /// Create an ACC client configured to use the mock server with pre-set token
    async fn create_mock_acc_client(mock_url: &str) -> AccClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        auth.set_3leg_token_for_testing(create_test_token()).await;
        AccClient::new(config, auth)
    }

    // ==================== Issues Client Tests ====================

    #[tokio::test]
    async fn test_list_issues_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/construction/issues/v1/projects/.+/issues"))
            .and(header("Authorization", "Bearer test-3leg-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [
                    {
                        "id": "issue-123",
                        "title": "Foundation crack",
                        "status": "open",
                        "description": "Crack found in basement foundation"
                    },
                    {
                        "id": "issue-456",
                        "title": "Missing insulation",
                        "status": "closed"
                    }
                ],
                "pagination": {
                    "limit": 50,
                    "offset": 0,
                    "totalResults": 2
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_issues_client(&server.uri()).await;
        let result = client.list_issues("project-123", None).await;

        assert!(result.is_ok());
        let issues = result.unwrap();
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].title, "Foundation crack");
    }

    #[tokio::test]
    async fn test_list_issues_empty() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/construction/issues/v1/projects/.+/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [],
                "pagination": {
                    "limit": 50,
                    "offset": 0,
                    "totalResults": 0
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_issues_client(&server.uri()).await;
        let result = client.list_issues("project-123", None).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_get_issue_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/construction/issues/v1/projects/.+/issues/.+"))
            .and(header("Authorization", "Bearer test-3leg-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "issue-123",
                "title": "Foundation crack",
                "description": "Crack found in basement foundation",
                "status": "open",
                "issueTypeId": "type-1",
                "assignedTo": "user-456",
                "dueDate": "2024-01-31",
                "createdAt": "2024-01-15T10:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_issues_client(&server.uri()).await;
        let result = client.get_issue("project-123", "issue-123").await;

        assert!(result.is_ok());
        let issue = result.unwrap();
        assert_eq!(issue.id, "issue-123");
        assert_eq!(issue.title, "Foundation crack");
    }

    #[tokio::test]
    async fn test_get_issue_not_found() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/construction/issues/v1/projects/.+/issues/.+"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "message": "Issue not found"
            })))
            .mount(&server)
            .await;

        let client = create_mock_issues_client(&server.uri()).await;
        let result = client.get_issue("project-123", "nonexistent").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("404"));
    }

    #[tokio::test]
    async fn test_create_issue_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path_regex(r"/construction/issues/v1/projects/.+/issues"))
            .and(header("Authorization", "Bearer test-3leg-token-12345"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "new-issue-789",
                "title": "New plumbing issue",
                "status": "open",
                "createdAt": "2024-01-15T12:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_issues_client(&server.uri()).await;
        let request = CreateIssueRequest {
            title: "New plumbing issue".to_string(),
            description: Some("Water leak detected".to_string()),
            status: "open".to_string(),
            issue_type_id: None,
            issue_subtype_id: None,
            assigned_to: None,
            assigned_to_type: None,
            due_date: None,
        };
        let result = client.create_issue("project-123", request).await;

        assert!(result.is_ok());
        let issue = result.unwrap();
        assert_eq!(issue.id, "new-issue-789");
    }

    #[tokio::test]
    async fn test_update_issue_success() {
        let server = MockServer::start().await;

        Mock::given(method("PATCH"))
            .and(path_regex(r"/construction/issues/v1/projects/.+/issues/.+"))
            .and(header("Authorization", "Bearer test-3leg-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "issue-123",
                "title": "Foundation crack",
                "status": "closed",
                "updatedAt": "2024-01-15T14:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_issues_client(&server.uri()).await;
        let request = UpdateIssueRequest {
            title: None,
            description: None,
            status: Some("closed".to_string()),
            assigned_to: None,
            due_date: None,
        };
        let result = client.update_issue("project-123", "issue-123", request).await;

        assert!(result.is_ok());
        let issue = result.unwrap();
        assert_eq!(issue.status, "closed");
    }

    #[tokio::test]
    async fn test_list_issue_types_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/construction/issues/v1/projects/.+/issue-types"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [
                    {
                        "id": "type-1",
                        "title": "Safety",
                        "isActive": true,
                        "subtypes": [
                            {"id": "subtype-1a", "title": "Fall hazard"},
                            {"id": "subtype-1b", "title": "Electrical"}
                        ]
                    },
                    {
                        "id": "type-2",
                        "title": "Quality",
                        "isActive": true
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = create_mock_issues_client(&server.uri()).await;
        let result = client.list_issue_types("project-123").await;

        assert!(result.is_ok());
        let types = result.unwrap();
        assert_eq!(types.len(), 2);
        assert!(types[0].subtypes.is_some());
    }

    #[tokio::test]
    async fn test_list_comments_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/construction/issues/v1/projects/.+/issues/.+/comments",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [
                    {
                        "id": "comment-1",
                        "body": "First comment",
                        "createdAt": "2024-01-15T10:00:00Z"
                    },
                    {
                        "id": "comment-2",
                        "body": "Second comment",
                        "createdAt": "2024-01-15T11:00:00Z"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = create_mock_issues_client(&server.uri()).await;
        let result = client.list_comments("project-123", "issue-123").await;

        assert!(result.is_ok());
        let comments = result.unwrap();
        assert_eq!(comments.len(), 2);
    }

    #[tokio::test]
    async fn test_add_comment_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path_regex(
                r"/construction/issues/v1/projects/.+/issues/.+/comments",
            ))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "comment-new",
                "body": "This is a new comment",
                "createdAt": "2024-01-15T15:00:00Z"
            })))
            .mount(&server)
            .await;

        let client = create_mock_issues_client(&server.uri()).await;
        let result = client
            .add_comment("project-123", "issue-123", "This is a new comment")
            .await;

        assert!(result.is_ok());
        let comment = result.unwrap();
        assert_eq!(comment.body, "This is a new comment");
    }

    // ==================== RFI Client Tests ====================

    #[tokio::test]
    async fn test_list_rfis_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/construction/rfis/v2/projects/.+/rfis"))
            .and(header("Authorization", "Bearer test-3leg-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [
                    {
                        "id": "rfi-123",
                        "title": "Clarification on wall specs",
                        "number": "RFI-001",
                        "status": "open",
                        "priority": "high",
                        "question": "What material for the west wall?"
                    },
                    {
                        "id": "rfi-456",
                        "title": "Foundation depth",
                        "number": "RFI-002",
                        "status": "answered"
                    }
                ],
                "pagination": {
                    "limit": 50,
                    "offset": 0,
                    "totalResults": 2
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_rfi_client(&server.uri()).await;
        let result = client.list_rfis("project-123").await;

        assert!(result.is_ok());
        let rfis = result.unwrap();
        assert_eq!(rfis.len(), 2);
        assert_eq!(rfis[0].title, "Clarification on wall specs");
    }

    #[tokio::test]
    async fn test_get_rfi_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/construction/rfis/v2/projects/.+/rfis/.+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "rfi-123",
                "title": "Clarification on wall specs",
                "number": "RFI-001",
                "status": "open",
                "priority": "high",
                "question": "What material for the west wall?",
                "dueDate": "2024-02-01"
            })))
            .mount(&server)
            .await;

        let client = create_mock_rfi_client(&server.uri()).await;
        let result = client.get_rfi("project-123", "rfi-123").await;

        assert!(result.is_ok());
        let rfi = result.unwrap();
        assert_eq!(rfi.id, "rfi-123");
    }

    #[tokio::test]
    async fn test_create_rfi_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path_regex(r"/construction/rfis/v2/projects/.+/rfis"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "rfi-new",
                "title": "New RFI",
                "number": "RFI-003",
                "status": "draft",
                "priority": "normal"
            })))
            .mount(&server)
            .await;

        let client = create_mock_rfi_client(&server.uri()).await;
        let request = CreateRfiRequest {
            title: "New RFI".to_string(),
            question: Some("What is the question?".to_string()),
            priority: Some("normal".to_string()),
            due_date: None,
            assigned_to: None,
            location: None,
            discipline: None,
        };
        let result = client.create_rfi("project-123", request).await;

        assert!(result.is_ok());
        let rfi = result.unwrap();
        assert_eq!(rfi.title, "New RFI");
    }

    #[tokio::test]
    async fn test_update_rfi_success() {
        let server = MockServer::start().await;

        Mock::given(method("PATCH"))
            .and(path_regex(r"/construction/rfis/v2/projects/.+/rfis/.+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "rfi-123",
                "title": "Clarification on wall specs",
                "status": "answered",
                "answer": "Use concrete block"
            })))
            .mount(&server)
            .await;

        let client = create_mock_rfi_client(&server.uri()).await;
        let request = UpdateRfiRequest {
            status: Some("answered".to_string()),
            answer: Some("Use concrete block".to_string()),
            ..Default::default()
        };
        let result = client.update_rfi("project-123", "rfi-123", request).await;

        assert!(result.is_ok());
        let rfi = result.unwrap();
        assert_eq!(rfi.status, "answered");
    }

    // ==================== ACC Extended Client Tests ====================

    #[tokio::test]
    async fn test_list_assets_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/construction/assets/v1/projects/.+/assets"))
            .and(header("Authorization", "Bearer test-3leg-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [
                    {
                        "id": "asset-1",
                        "categoryId": "cat-1",
                        "description": "HVAC Unit 1",
                        "barcode": "BC-001"
                    },
                    {
                        "id": "asset-2",
                        "categoryId": "cat-2",
                        "description": "Fire extinguisher"
                    }
                ],
                "pagination": {
                    "limit": 50,
                    "offset": 0,
                    "totalResults": 2
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_acc_client(&server.uri()).await;
        let result = client.list_assets("project-123").await;

        assert!(result.is_ok());
        let assets = result.unwrap();
        assert_eq!(assets.len(), 2);
    }

    #[tokio::test]
    async fn test_list_submittals_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/construction/submittals/v1/projects/.+/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [
                    {
                        "id": "submittal-1",
                        "title": "HVAC Specifications",
                        "number": "SUB-001",
                        "status": "pending",
                        "specSection": "23 00 00"
                    },
                    {
                        "id": "submittal-2",
                        "title": "Electrical Plans",
                        "number": "SUB-002",
                        "status": "approved"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = create_mock_acc_client(&server.uri()).await;
        let result = client.list_submittals("project-123").await;

        assert!(result.is_ok());
        let submittals = result.unwrap();
        assert_eq!(submittals.len(), 2);
    }

    #[tokio::test]
    async fn test_list_checklists_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/construction/checklists/v1/projects/.+/checklists",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [
                    {
                        "id": "checklist-1",
                        "title": "Safety Inspection",
                        "status": "in_progress",
                        "location": "Building A"
                    },
                    {
                        "id": "checklist-2",
                        "title": "Quality Check",
                        "status": "completed"
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = create_mock_acc_client(&server.uri()).await;
        let result = client.list_checklists("project-123").await;

        assert!(result.is_ok());
        let checklists = result.unwrap();
        assert_eq!(checklists.len(), 2);
    }

    // ==================== Error Handling ====================

    #[tokio::test]
    async fn test_unauthorized_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/construction/issues/v1/projects/.+/issues"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "message": "Unauthorized"
            })))
            .mount(&server)
            .await;

        let client = create_mock_issues_client(&server.uri()).await;
        let result = client.list_issues("project-123", None).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("401"));
    }

    #[tokio::test]
    async fn test_forbidden_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/construction/rfis/v2/projects/.+/rfis"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "message": "Access denied"
            })))
            .mount(&server)
            .await;

        let client = create_mock_rfi_client(&server.uri()).await;
        let result = client.list_rfis("project-123").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("403"));
    }

    #[tokio::test]
    async fn test_server_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/construction/assets/v1/projects/.+/assets"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "message": "Internal server error"
            })))
            .mount(&server)
            .await;

        let client = create_mock_acc_client(&server.uri()).await;
        let result = client.list_assets("project-123").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("500"));
    }
}

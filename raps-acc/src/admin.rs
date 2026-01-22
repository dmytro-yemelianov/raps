// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account Admin API client for ACC/BIM 360

use anyhow::{Context, Result};

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

use serde::Serialize;

use crate::types::{AccountProject, AccountUser, PaginatedResponse, ProjectClassification};

/// Client for ACC Account Admin API
///
/// Provides operations for managing users and projects at the account level.
/// Requires account admin privileges.
pub struct AccountAdminClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl AccountAdminClient {
    /// Create a new Account Admin client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create client with custom HTTP configuration
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

    /// Get the base URL for Account Admin API
    fn admin_url(&self, account_id: &str) -> String {
        format!(
            "{}/construction/admin/v1/accounts/{}",
            self.config.base_url, account_id
        )
    }

    /// List all users in an account (paginated)
    ///
    /// # Arguments
    /// * `account_id` - The account ID (without "b." prefix if present)
    /// * `limit` - Maximum number of results per page (max: 200)
    /// * `offset` - Starting index for pagination
    pub async fn list_users(
        &self,
        account_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<AccountUser>> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);

        let mut url = format!("{}/users", self.admin_url(&account_id));

        // Build query parameters
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l.min(200)));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list account users")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list users ({status}): {error_text}");
        }

        let users_response: PaginatedResponse<AccountUser> = response
            .json()
            .await
            .context("Failed to parse users response")?;

        Ok(users_response)
    }

    /// Search for a user by email address
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `email` - Email address to search for
    ///
    /// # Returns
    /// The user if found, None if not found
    pub async fn find_user_by_email(
        &self,
        account_id: &str,
        email: &str,
    ) -> Result<Option<AccountUser>> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);

        let url = format!("{}/users/search", self.admin_url(&account_id));

        let request_body = serde_json::json!({
            "email": email
        });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to search for user")?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to search for user ({status}): {error_text}");
        }

        // The search endpoint returns a single user or array
        let user: AccountUser = response
            .json()
            .await
            .context("Failed to parse user search response")?;

        Ok(Some(user))
    }

    /// List all projects in an account (paginated)
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `limit` - Maximum results per page (max: 200)
    /// * `offset` - Starting index
    pub async fn list_projects(
        &self,
        account_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<AccountProject>> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);

        let mut url = format!("{}/projects", self.admin_url(&account_id));

        // Build query parameters
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l.min(200)));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list projects")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list projects ({status}): {error_text}");
        }

        let projects_response: PaginatedResponse<AccountProject> = response
            .json()
            .await
            .context("Failed to parse projects response")?;

        Ok(projects_response)
    }

    /// Get details of a specific project
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `project_id` - The project ID
    pub async fn get_project(&self, account_id: &str, project_id: &str) -> Result<AccountProject> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);
        let project_id = normalize_project_id(project_id);

        let url = format!("{}/projects/{}", self.admin_url(&account_id), project_id);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get project")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get project ({status}): {error_text}");
        }

        let project: AccountProject = response
            .json()
            .await
            .context("Failed to parse project response")?;

        Ok(project)
    }

    /// Fetch all users in an account (handles pagination automatically)
    ///
    /// This is a convenience method that iterates through all pages.
    /// Use with caution for accounts with many users.
    pub async fn list_all_users(&self, account_id: &str) -> Result<Vec<AccountUser>> {
        let mut all_users = Vec::new();
        let mut offset = 0;
        let limit = 200; // Maximum allowed

        loop {
            let response = self
                .list_users(account_id, Some(limit), Some(offset))
                .await?;
            let has_more = response.has_more();
            let next_offset = response.next_offset();
            all_users.extend(response.results);

            if !has_more {
                break;
            }
            offset = next_offset;
        }

        Ok(all_users)
    }

    /// Fetch all projects in an account (handles pagination automatically)
    ///
    /// This is a convenience method that iterates through all pages.
    pub async fn list_all_projects(&self, account_id: &str) -> Result<Vec<AccountProject>> {
        let mut all_projects = Vec::new();
        let mut offset = 0;
        let limit = 200; // Maximum allowed

        loop {
            let response = self
                .list_projects(account_id, Some(limit), Some(offset))
                .await?;
            let has_more = response.has_more();
            let next_offset = response.next_offset();
            all_projects.extend(response.results);

            if !has_more {
                break;
            }
            offset = next_offset;
        }

        Ok(all_projects)
    }

    // ========================================================================
    // TEMPLATE OPERATIONS
    // ========================================================================

    /// List project templates in an account (paginated)
    ///
    /// Templates are projects with classification="template".
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `limit` - Maximum results per page (max: 200)
    /// * `offset` - Starting index
    pub async fn list_templates(
        &self,
        account_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<AccountProject>> {
        self.list_projects_filtered(
            account_id,
            Some(ProjectClassification::Template),
            None,
            limit,
            offset,
        )
        .await
    }

    /// List projects with optional classification and name filters
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `classification` - Filter by project classification (template, production, etc.)
    /// * `name_filter` - Filter by project name (partial match)
    /// * `limit` - Maximum results per page (max: 200)
    /// * `offset` - Starting index
    pub async fn list_projects_filtered(
        &self,
        account_id: &str,
        classification: Option<ProjectClassification>,
        name_filter: Option<&str>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<AccountProject>> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);

        let mut url = format!("{}/projects", self.admin_url(&account_id));

        // Build query parameters
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={}", l.min(200)));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        if let Some(c) = classification {
            params.push(format!("filter[classification]={}", c));
        }
        if let Some(name) = name_filter {
            params.push(format!("filter[name]={}", urlencoding::encode(name)));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list projects")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list projects ({status}): {error_text}");
        }

        let projects_response: PaginatedResponse<AccountProject> = response
            .json()
            .await
            .context("Failed to parse projects response")?;

        Ok(projects_response)
    }

    /// Fetch all templates in an account (handles pagination automatically)
    pub async fn list_all_templates(&self, account_id: &str) -> Result<Vec<AccountProject>> {
        let mut all_templates = Vec::new();
        let mut offset = 0;
        let limit = 200;

        loop {
            let response = self
                .list_templates(account_id, Some(limit), Some(offset))
                .await?;
            let has_more = response.has_more();
            let next_offset = response.next_offset();
            all_templates.extend(response.results);

            if !has_more {
                break;
            }
            offset = next_offset;
        }

        Ok(all_templates)
    }

    // ========================================================================
    // PROJECT CREATE/UPDATE/DELETE
    // ========================================================================

    /// Create a new project in an account
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `request` - Project creation parameters
    ///
    /// # Returns
    /// The created project (may be in pending status initially)
    pub async fn create_project(
        &self,
        account_id: &str,
        request: CreateProjectRequest,
    ) -> Result<AccountProject> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);

        let url = format!("{}/projects", self.admin_url(&account_id));

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to create project")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create project ({status}): {error_text}");
        }

        let project: AccountProject = response
            .json()
            .await
            .context("Failed to parse project creation response")?;

        Ok(project)
    }

    /// Update an existing project
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `project_id` - The project ID to update
    /// * `request` - Update parameters
    pub async fn update_project(
        &self,
        account_id: &str,
        project_id: &str,
        request: UpdateProjectRequest,
    ) -> Result<AccountProject> {
        let token = self.auth.get_3leg_token().await?;
        let account_id = normalize_account_id(account_id);
        let project_id = normalize_project_id(project_id);

        let url = format!("{}/projects/{}", self.admin_url(&account_id), project_id);

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to update project")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update project ({status}): {error_text}");
        }

        let project: AccountProject = response
            .json()
            .await
            .context("Failed to parse project update response")?;

        Ok(project)
    }

    /// Archive a project (soft delete)
    ///
    /// Projects cannot be permanently deleted via API. Archiving sets status to "archived".
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `project_id` - The project ID to archive
    pub async fn archive_project(&self, account_id: &str, project_id: &str) -> Result<()> {
        let request = UpdateProjectRequest {
            status: Some("archived".to_string()),
            ..Default::default()
        };
        self.update_project(account_id, project_id, request).await?;
        Ok(())
    }

    /// Wait for a project to become active
    ///
    /// Polls the project status until it becomes active or times out.
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `project_id` - The project ID to wait for
    /// * `timeout_secs` - Maximum time to wait (default: 120 seconds)
    /// * `poll_interval_ms` - Time between polls (default: 3000ms)
    pub async fn wait_for_project_active(
        &self,
        account_id: &str,
        project_id: &str,
        timeout_secs: Option<u64>,
        poll_interval_ms: Option<u64>,
    ) -> Result<AccountProject> {
        let timeout = std::time::Duration::from_secs(timeout_secs.unwrap_or(120));
        let poll_interval = std::time::Duration::from_millis(poll_interval_ms.unwrap_or(3000));
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                anyhow::bail!(
                    "Timeout waiting for project {} to become active (waited {}s)",
                    project_id,
                    timeout.as_secs()
                );
            }

            let project = self.get_project(account_id, project_id).await?;
            let status = project.status.as_deref().unwrap_or("unknown");

            match status.to_lowercase().as_str() {
                "active" => return Ok(project),
                "failed" | "error" => {
                    anyhow::bail!("Project creation failed for project {}", project_id);
                }
                _ => {
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    }
}

// ============================================================================
// REQUEST TYPES
// ============================================================================

/// Request to create a new project
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    /// Project name (required)
    pub name: String,
    /// Project type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Project classification (production, template, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<ProjectClassification>,
    /// Template configuration (for creating from template)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateConfig>,
    /// Products to enable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub products: Option<Vec<String>>,
    /// Project start date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    /// Project end date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    /// Project value/budget
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Currency code (e.g., "USD")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Project address line 1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_line1: Option<String>,
    /// Project address line 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_line2: Option<String>,
    /// City
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// State/Province
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_or_province: Option<String>,
    /// Postal code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    /// Country
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Time zone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// Construction type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub construction_type: Option<String>,
    /// Contract type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_type: Option<String>,
}

/// Template configuration for creating project from template
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateConfig {
    /// ID of the template project to clone from
    pub project_id: String,
    /// Options for what to include when cloning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<TemplateOptions>,
}

/// Options for template cloning
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TemplateOptions {
    /// Field-level options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<TemplateFieldOptions>,
}

/// Field-level options for template cloning
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TemplateFieldOptions {
    /// Whether to copy company data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_companies: Option<bool>,
    /// Whether to copy location data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_locations: Option<bool>,
}

/// Request to update an existing project
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    /// Project name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Project status (active, archived, suspended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Project start date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    /// Project end date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    /// Project type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Project value/budget
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Currency code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Address line 1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_line1: Option<String>,
    /// Address line 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_line2: Option<String>,
    /// City
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// State/Province
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_or_province: Option<String>,
    /// Postal code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    /// Country
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Time zone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

/// Normalize account ID to the format expected by ACC Admin API
///
/// Handles various input formats:
/// - `b.{uuid}` (BIM 360 hub format) -> extracts uuid
/// - `a.{base64}` (ACC hub format) -> decodes and extracts account ID
/// - Raw UUID -> returns as-is
fn normalize_account_id(account_id: &str) -> String {
    // Handle BIM 360 format: b.{uuid}
    if let Some(id) = account_id.strip_prefix("b.") {
        return id.to_string();
    }

    // Handle ACC format: a.{base64}
    if let Some(encoded) = account_id.strip_prefix("a.")
        && let Ok(decoded_bytes) = base64_decode(encoded)
        && let Ok(decoded) = String::from_utf8(decoded_bytes)
    {
        // Format is typically "business:{account_id}" or just the account_id
        if let Some(id) = decoded.strip_prefix("business:") {
            return id.to_string();
        }
        // Try splitting on colon for other formats
        if let Some((_, id)) = decoded.split_once(':') {
            return id.to_string();
        }
        return decoded;
    }

    // Already a raw account ID
    account_id.to_string()
}

/// Simple base64 decoder (URL-safe variant)
fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    STANDARD.decode(input).map_err(|_| ())
}

/// Remove "b." prefix from project ID if present
fn normalize_project_id(project_id: &str) -> String {
    project_id
        .strip_prefix("b.")
        .unwrap_or(project_id)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_account_id() {
        // BIM 360 format: b.{uuid}
        assert_eq!(normalize_account_id("b.123-456"), "123-456");
        // Raw UUID
        assert_eq!(normalize_account_id("123-456"), "123-456");
        // ACC format: a.{base64} where base64 decodes to "business:{account_id}"
        // "YnVzaW5lc3M6Z21haWw2MDUzMTAz" decodes to "business:gmail6053103"
        assert_eq!(
            normalize_account_id("a.YnVzaW5lc3M6Z21haWw2MDUzMTAz"),
            "gmail6053103"
        );
    }

    #[test]
    fn test_normalize_project_id() {
        assert_eq!(normalize_project_id("b.proj-123"), "proj-123");
        assert_eq!(normalize_project_id("proj-123"), "proj-123");
    }
}

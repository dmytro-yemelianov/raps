// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account Admin project operations

use anyhow::{Context, Result};

use raps_kernel::http;

use crate::types::{AccountProject, PaginatedResponse, ProjectClassification};

use super::types::{CreateProjectRequest, UpdateProjectRequest};
use super::{AccountAdminClient, normalize_account_id};

impl AccountAdminClient {
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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

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
        let project_id = crate::strip_project_prefix(project_id);

        let url = format!("{}/projects/{}", self.admin_url(&account_id), project_id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&request)
        })
        .await?;

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
        let project_id = crate::strip_project_prefix(project_id);

        let url = format!("{}/projects/{}", self.admin_url(&account_id), project_id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .patch(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&request)
        })
        .await?;

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

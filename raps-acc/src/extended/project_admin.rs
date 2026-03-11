// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC/BIM 360 Project Admin operations (project creation and management).

use anyhow::{Context, Result};

use raps_kernel::http;

use super::AccClient;
use super::types::*;

/// Map ACC product names to BIM 360 service_types.
///
/// ACC uses product names like "docs", "build", "model".
/// BIM 360 HQ v1 uses service_types like "doc_manager", "pm", "field".
fn products_to_service_types(products: &Option<Vec<String>>) -> String {
    let Some(products) = products else {
        return "doc_manager".to_string();
    };
    if products.is_empty() {
        return "doc_manager".to_string();
    }

    let mapped: Vec<&str> = products
        .iter()
        .map(|p| match p.as_str() {
            "docs" | "document_management" | "doc_manager" => "doc_manager",
            "build" | "field" | "construction" => "field",
            "model" | "model_coordination" | "glue" => "glue",
            "design" | "plan" => "plan",
            "insight" => "field", // closest BIM 360 equivalent
            "cost" | "quantify" => "field",
            "pm" | "project_management" => "pm",
            other => other,
        })
        .collect();

    // Deduplicate
    let mut unique: Vec<&str> = Vec::new();
    for s in &mapped {
        if !unique.contains(s) {
            unique.push(s);
        }
    }

    unique.join(",")
}

impl AccClient {
    /// Get the base URL for ACC Construction Admin v1 API
    fn admin_url(&self, account_id: &str) -> String {
        format!(
            "{}/construction/admin/v1/accounts/{}",
            self.config.base_url, account_id
        )
    }

    /// Get the base URL for BIM 360 HQ v1 API
    fn hq_url(&self, account_id: &str) -> String {
        format!("{}/hq/v1/accounts/{}", self.config.base_url, account_id)
    }

    /// Create a new ACC/BIM 360 project
    ///
    /// Tries ACC Construction Admin v1 first (3-legged auth). Falls back to
    /// BIM 360 HQ v1 (2-legged auth) if the account is a BIM 360 Business hub.
    /// The project is created asynchronously. Use `wait_for_project_activation`
    /// to poll until the project is active.
    ///
    /// # Arguments
    /// * `account_id` - The account ID (ACC or BIM 360)
    /// * `request` - Project creation parameters
    ///
    /// # Returns
    /// A `ProjectCreationJob` containing the project ID and initial status.
    pub async fn create_project(
        &self,
        account_id: &str,
        request: CreateProjectRequest,
    ) -> Result<ProjectCreationJob> {
        let token = self.auth.get_3leg_token().await?;

        // Try ACC Construction Admin v1 first (3-legged)
        let url = format!("{}/projects", self.admin_url(account_id));

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&request)
        })
        .await?;

        if response.status().is_success() {
            return self.parse_acc_creation_response(response).await;
        }

        let resp_status = response.status().as_u16();
        if resp_status != 400 && resp_status != 404 {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create project ({resp_status}): {error_text}");
        }

        // Fall back to BIM 360 HQ v1 (2-legged auth + x-user-id, different request format)
        let token_2leg = self.auth.get_token().await?;
        let user_info = self.auth.get_user_info().await?;
        let url = format!("{}/projects", self.hq_url(account_id));

        let bim360_request = Bim360CreateProjectRequest {
            name: request.name,
            service_types: products_to_service_types(&request.products),
            r#type: Some("project".to_string()),
        };

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token_2leg)
                .header("Content-Type", "application/json")
                .header("x-user-id", &user_info.sub)
                .json(&bim360_request)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create BIM 360 project ({status}): {error_text}");
        }

        self.parse_bim360_creation_response(response).await
    }

    /// Create a project directly via BIM 360 HQ v1 endpoint (2-legged auth).
    ///
    /// Use this for accounts whose hub extension_type is `bim360`.
    /// The ACC v1 endpoint accepts creates on BIM 360 hubs but registers
    /// them as ACC projects, which is incorrect.
    pub async fn create_project_bim360(
        &self,
        account_id: &str,
        request: CreateProjectRequest,
    ) -> Result<ProjectCreationJob> {
        let token_2leg = self.auth.get_token().await?;
        let user_info = self.auth.get_user_info().await?;
        let url = format!("{}/projects", self.hq_url(account_id));

        let bim360_request = Bim360CreateProjectRequest {
            name: request.name,
            service_types: products_to_service_types(&request.products),
            r#type: Some("project".to_string()),
        };

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token_2leg)
                .header("Content-Type", "application/json")
                .header("x-user-id", &user_info.sub)
                .json(&bim360_request)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create BIM 360 project ({status}): {error_text}");
        }

        self.parse_bim360_creation_response(response).await
    }

    /// Parse an ACC project creation response (camelCase) into a ProjectCreationJob
    async fn parse_acc_creation_response(
        &self,
        response: reqwest::Response,
    ) -> Result<ProjectCreationJob> {
        let project: AccProject = response
            .json()
            .await
            .context("Failed to parse ACC project creation response")?;

        let status = project
            .status
            .as_deref()
            .map(ProjectCreationStatus::parse)
            .unwrap_or(ProjectCreationStatus::Pending);

        Ok(ProjectCreationJob {
            job_id: project.job_id,
            project_id: Some(project.id),
            status,
            name: Some(project.name),
        })
    }

    /// Parse a BIM 360 project creation response (snake_case) into a ProjectCreationJob
    async fn parse_bim360_creation_response(
        &self,
        response: reqwest::Response,
    ) -> Result<ProjectCreationJob> {
        let project: Bim360Project = response
            .json()
            .await
            .context("Failed to parse BIM 360 project creation response")?;

        let status = project
            .status
            .as_deref()
            .map(ProjectCreationStatus::parse)
            .unwrap_or(ProjectCreationStatus::Pending);

        Ok(ProjectCreationJob {
            job_id: project.job_id,
            project_id: Some(project.id),
            status,
            name: Some(project.name),
        })
    }

    /// Wait for a project to become active
    ///
    /// Polls the project status until it becomes active or fails.
    /// Times out after the specified duration.
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `project_id` - The project ID to check
    /// * `timeout_secs` - Maximum time to wait (default: 60 seconds)
    /// * `poll_interval_ms` - Time between polls (default: 2000ms)
    ///
    /// # Returns
    /// The final `ProjectCreationJob` with updated status.
    pub async fn wait_for_project_activation(
        &self,
        account_id: &str,
        project_id: &str,
        timeout_secs: Option<u64>,
        poll_interval_ms: Option<u64>,
    ) -> Result<ProjectCreationJob> {
        let timeout = std::time::Duration::from_secs(timeout_secs.unwrap_or(60));
        let poll_interval = std::time::Duration::from_millis(poll_interval_ms.unwrap_or(2000));
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
            let status = project
                .status
                .as_deref()
                .map(ProjectCreationStatus::parse)
                .unwrap_or(ProjectCreationStatus::Processing);

            match status {
                ProjectCreationStatus::Active => {
                    return Ok(ProjectCreationJob {
                        job_id: project.job_id,
                        project_id: Some(project.id),
                        status: ProjectCreationStatus::Active,
                        name: Some(project.name),
                    });
                }
                ProjectCreationStatus::Failed => {
                    anyhow::bail!("Project creation failed for project {}", project_id);
                }
                _ => {
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    }

    /// Get a project by ID (ACC or BIM 360)
    ///
    /// Tries ACC Construction Admin v1 first (3-legged), falls back to
    /// BIM 360 HQ v1 (2-legged).
    ///
    /// # Arguments
    /// * `account_id` - The account ID
    /// * `project_id` - The project ID
    pub async fn get_project(&self, account_id: &str, project_id: &str) -> Result<AccProject> {
        let token = self.auth.get_3leg_token().await?;

        // Try ACC Construction Admin v1 first
        let url = format!("{}/projects/{}", self.admin_url(account_id), project_id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if response.status().is_success() {
            let project: AccProject = response
                .json()
                .await
                .context("Failed to parse project response")?;
            return Ok(project);
        }

        let resp_status = response.status().as_u16();
        if resp_status != 400 && resp_status != 404 {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get project ({resp_status}): {error_text}");
        }

        // Fall back to BIM 360 HQ v1 (2-legged auth)
        let token_2leg = self.auth.get_token().await?;
        let url = format!("{}/projects/{}", self.hq_url(account_id), project_id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token_2leg)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get BIM 360 project ({status}): {error_text}");
        }

        // BIM 360 returns snake_case — parse and convert
        let bim_project: Bim360Project = response
            .json()
            .await
            .context("Failed to parse BIM 360 project response")?;

        Ok(AccProject {
            id: bim_project.id,
            name: bim_project.name,
            status: bim_project.status,
            account_id: bim_project.account_id,
            job_id: bim_project.job_id,
        })
    }
}

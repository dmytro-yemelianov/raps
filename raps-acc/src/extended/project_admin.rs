// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC Project Admin operations (project creation and management).

use anyhow::{Context, Result};

use raps_kernel::http;

use super::types::*;
use super::AccClient;

impl AccClient {
    /// Get the base URL for ACC HQ Admin API
    fn hq_url(&self, account_id: &str) -> String {
        format!("{}/hq/v1/accounts/{}", self.config.base_url, account_id)
    }

    /// Create a new ACC project
    ///
    /// Creates a project in an ACC account. ACC only (not BIM 360).
    /// The project is created asynchronously. Use `wait_for_project_activation`
    /// to poll until the project is active.
    ///
    /// # Arguments
    /// * `account_id` - The ACC account ID
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
        let url = format!("{}/projects", self.hq_url(account_id));

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
            anyhow::bail!("Failed to create ACC project ({status}): {error_text}");
        }

        let project: AccProject = response
            .json()
            .await
            .context("Failed to parse project creation response")?;

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
    /// * `account_id` - The ACC account ID
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

    /// Get an ACC project by ID
    ///
    /// # Arguments
    /// * `account_id` - The ACC account ID
    /// * `project_id` - The project ID
    pub async fn get_project(&self, account_id: &str, project_id: &str) -> Result<AccProject> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/projects/{}", self.hq_url(account_id), project_id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get ACC project ({status}): {error_text}");
        }

        let project: AccProject = response
            .json()
            .await
            .context("Failed to parse project response")?;

        Ok(project)
    }
}

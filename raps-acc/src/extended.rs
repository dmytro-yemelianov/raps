// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC Extended API client (Assets, Submittals, Checklists) and Project Admin

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::{self, HttpClientConfig};

// ============================================================================
// ACC PROJECT ADMIN API (Project Creation)
// ============================================================================

/// Status of an ACC project creation job
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectCreationStatus {
    /// Job is pending
    Pending,
    /// Job is being processed
    Processing,
    /// Project created and active
    Active,
    /// Project creation failed
    Failed,
}

impl ProjectCreationStatus {
    /// Parse status from API response string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => ProjectCreationStatus::Pending,
            "processing" => ProjectCreationStatus::Processing,
            "active" => ProjectCreationStatus::Active,
            "failed" | "error" => ProjectCreationStatus::Failed,
            _ => ProjectCreationStatus::Processing, // Default to processing for unknown states
        }
    }
}

/// Result of an ACC project creation operation
#[derive(Debug, Clone)]
pub struct ProjectCreationJob {
    /// The job ID returned by the API
    pub job_id: Option<String>,
    /// The created project ID (available after activation)
    pub project_id: Option<String>,
    /// Current status of the project creation
    pub status: ProjectCreationStatus,
    /// Project name
    pub name: Option<String>,
}

/// Request to create an ACC project
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    /// Project name
    pub name: String,
    /// Optional template project ID to clone from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_project_id: Option<String>,
    /// Products to enable (e.g., ["build", "docs", "model"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub products: Option<Vec<String>>,
    /// Project type (default: "ACC")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_type: Option<String>,
}

/// ACC Project response from API
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccProject {
    /// Project ID
    pub id: String,
    /// Project name
    pub name: String,
    /// Project status (pending, active, etc.)
    pub status: Option<String>,
    /// Account ID
    pub account_id: Option<String>,
    /// Job ID (for tracking creation progress)
    pub job_id: Option<String>,
}

// ============================================================================
// ASSET TYPES
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

// ============================================================================
// SUBMITTAL TYPES
// ============================================================================

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

// ============================================================================
// CHECKLIST TYPES
// ============================================================================

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

// ============================================================================
// SHARED PAGINATION
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub limit: i32,
    pub offset: i32,
    pub total_results: i32,
}

// ============================================================================
// ACC CLIENT
// ============================================================================

/// ACC Extended API client
#[derive(Clone)]
pub struct AccClient {
    config: Config,
    pub(crate) auth: AuthClient,
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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .json(&request)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .patch(&url)
                .bearer_auth(&token)
                .json(&request)
        })
        .await?;

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

    /// Delete an asset
    pub async fn delete_asset(&self, project_id: &str, asset_id: &str) -> Result<()> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/assets/{}",
            self.config.assets_url(),
            project_id,
            asset_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete asset ({status}): {error_text}");
        }

        Ok(())
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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .json(&request)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .patch(&url)
                .bearer_auth(&token)
                .json(&request)
        })
        .await?;

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

    /// Delete a submittal
    pub async fn delete_submittal(&self, project_id: &str, submittal_id: &str) -> Result<()> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}",
            self.config.submittals_url(),
            project_id,
            submittal_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete submittal ({status}): {error_text}");
        }

        Ok(())
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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .json(&request)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .patch(&url)
                .bearer_auth(&token)
                .json(&request)
        })
        .await?;

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

    // ============== ACC PROJECT ADMIN ==============

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
            // Check if we've exceeded the timeout
            if start.elapsed() > timeout {
                anyhow::bail!(
                    "Timeout waiting for project {} to become active (waited {}s)",
                    project_id,
                    timeout.as_secs()
                );
            }

            // Get project status
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
                    // Still pending/processing, wait and retry
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

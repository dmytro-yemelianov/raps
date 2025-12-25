//! ACC (Autodesk Construction Cloud) Extended API module
//!
//! Provides support for additional ACC modules:
//! - Assets
//! - Submittals  
//! - Checklists
//!
//! Note: These APIs may require specific ACC entitlements.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::AuthClient;
use crate::config::Config;

// ============== ASSETS ==============

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

// ============== SUBMITTALS ==============

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

// ============== CHECKLISTS ==============

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

// ============== SHARED ==============

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
        Self::new_with_http_config(config, auth, crate::http::HttpClientConfig::default())
    }

    /// Create a new ACC client with custom HTTP config
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

    // ============== ASSETS ==============

    /// List assets in a project
    pub async fn list_assets(&self, project_id: &str) -> Result<Vec<Asset>> {
        let token = self.auth.get_3leg_token().await?;
        // Note: The actual API endpoint may vary based on ACC API version
        let url = format!(
            "https://developer.api.autodesk.com/construction/assets/v1/projects/{}/assets",
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
            anyhow::bail!("Failed to list assets ({}): {}", status, error_text);
        }

        let assets_response: AssetsResponse = response
            .json()
            .await
            .context("Failed to parse assets response")?;

        Ok(assets_response.results)
    }

    // ============== SUBMITTALS ==============

    /// List submittals in a project
    pub async fn list_submittals(&self, project_id: &str) -> Result<Vec<Submittal>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/submittals/v1/projects/{}/items",
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
            anyhow::bail!("Failed to list submittals ({}): {}", status, error_text);
        }

        let submittals_response: SubmittalsResponse = response
            .json()
            .await
            .context("Failed to parse submittals response")?;

        Ok(submittals_response.results)
    }

    // ============== CHECKLISTS ==============

    /// List checklists in a project
    pub async fn list_checklists(&self, project_id: &str) -> Result<Vec<Checklist>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/checklists/v1/projects/{}/checklists",
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
            anyhow::bail!("Failed to list checklists ({}): {}", status, error_text);
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
            "https://developer.api.autodesk.com/construction/checklists/v1/projects/{}/templates",
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

    // ============== ASSET CRUD ==============

    /// Get a specific asset by ID
    pub async fn get_asset(&self, project_id: &str, asset_id: &str) -> Result<Asset> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/assets/v1/projects/{}/assets/{}",
            project_id, asset_id
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
            anyhow::bail!("Failed to get asset ({}): {}", status, error_text);
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
            "https://developer.api.autodesk.com/construction/assets/v1/projects/{}/assets",
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
            anyhow::bail!("Failed to create asset ({}): {}", status, error_text);
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
            "https://developer.api.autodesk.com/construction/assets/v1/projects/{}/assets/{}",
            project_id, asset_id
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
            anyhow::bail!("Failed to update asset ({}): {}", status, error_text);
        }

        let asset: Asset = response
            .json()
            .await
            .context("Failed to parse asset response")?;
        Ok(asset)
    }

    // ============== SUBMITTAL CRUD ==============

    /// Get a specific submittal by ID
    pub async fn get_submittal(&self, project_id: &str, submittal_id: &str) -> Result<Submittal> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/submittals/v1/projects/{}/items/{}",
            project_id, submittal_id
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
            anyhow::bail!("Failed to get submittal ({}): {}", status, error_text);
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
            "https://developer.api.autodesk.com/construction/submittals/v1/projects/{}/items",
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
            anyhow::bail!("Failed to create submittal ({}): {}", status, error_text);
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
            "https://developer.api.autodesk.com/construction/submittals/v1/projects/{}/items/{}",
            project_id, submittal_id
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
            anyhow::bail!("Failed to update submittal ({}): {}", status, error_text);
        }

        let submittal: Submittal = response
            .json()
            .await
            .context("Failed to parse submittal response")?;
        Ok(submittal)
    }

    // ============== CHECKLIST CRUD ==============

    /// Get a specific checklist by ID
    pub async fn get_checklist(&self, project_id: &str, checklist_id: &str) -> Result<Checklist> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/checklists/v1/projects/{}/checklists/{}",
            project_id, checklist_id
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
            anyhow::bail!("Failed to get checklist ({}): {}", status, error_text);
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
            "https://developer.api.autodesk.com/construction/checklists/v1/projects/{}/checklists",
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
            anyhow::bail!("Failed to create checklist ({}): {}", status, error_text);
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
            "https://developer.api.autodesk.com/construction/checklists/v1/projects/{}/checklists/{}",
            project_id, checklist_id
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
            anyhow::bail!("Failed to update checklist ({}): {}", status, error_text);
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

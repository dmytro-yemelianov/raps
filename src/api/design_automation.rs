//! Design Automation API module
//!
//! Handles automation of CAD processing with engines like AutoCAD, Revit, Inventor, 3ds Max.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::AuthClient;
use crate::config::Config;

/// Engine information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Engine {
    pub id: String,
    pub description: Option<String>,
    pub product_version: Option<String>,
}

/// AppBundle information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBundle {
    pub id: String,
    pub engine: String,
    pub description: Option<String>,
    pub version: Option<i32>,
}

/// AppBundle details (full)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBundleDetails {
    pub id: String,
    pub engine: String,
    pub description: Option<String>,
    pub version: i32,
    pub package: Option<String>,
    pub upload_parameters: Option<UploadParameters>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadParameters {
    pub endpoint_url: String,
    pub form_data: std::collections::HashMap<String, String>,
}

/// Activity information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub engine: String,
    pub description: Option<String>,
    pub version: Option<i32>,
    pub command_line: Option<Vec<String>>,
    pub app_bundles: Option<Vec<String>>,
}

/// WorkItem information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItem {
    pub id: String,
    pub status: String,
    pub progress: Option<String>,
    pub report_url: Option<String>,
    pub stats: Option<WorkItemStats>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemStats {
    pub time_queued: Option<String>,
    pub time_download_started: Option<String>,
    pub time_instruction_started: Option<String>,
    pub time_instruction_ended: Option<String>,
    pub time_upload_ended: Option<String>,
    pub time_finished: Option<String>,
    pub bytes_downloaded: Option<i64>,
    pub bytes_uploaded: Option<i64>,
}

/// Request to create an AppBundle
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAppBundleRequest {
    pub id: String,
    pub engine: String,
    pub description: Option<String>,
}

/// Request to create an Activity
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateActivityRequest {
    pub id: String,
    pub engine: String,
    pub command_line: Vec<String>,
    pub app_bundles: Vec<String>,
    pub parameters: std::collections::HashMap<String, ActivityParameter>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityParameter {
    pub verb: String,
    pub local_name: Option<String>,
    pub description: Option<String>,
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip: Option<bool>,
}

/// Request to create a WorkItem
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkItemRequest {
    pub activity_id: String,
    pub arguments: std::collections::HashMap<String, WorkItemArgument>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemArgument {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Paginated response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination_token: Option<String>,
}

/// Design Automation API client
pub struct DesignAutomationClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl DesignAutomationClient {
    /// Create a new Design Automation client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, crate::http::HttpClientConfig::default())
    }

    /// Create a new Design Automation client with custom HTTP config
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

    /// Get the nickname for this client (or "default")
    fn nickname(&self) -> &str {
        self.config.da_nickname.as_deref().unwrap_or("default")
    }

    /// List available engines
    pub async fn list_engines(&self) -> Result<Vec<Engine>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/engines", self.config.da_url());

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list engines")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list engines ({}): {}", status, error_text);
        }

        let paginated: PaginatedResponse<Engine> = response
            .json()
            .await
            .context("Failed to parse engines response")?;

        Ok(paginated.data)
    }

    /// List all app bundles
    pub async fn list_appbundles(&self) -> Result<Vec<String>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles", self.config.da_url());

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list appbundles")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list appbundles ({}): {}", status, error_text);
        }

        let paginated: PaginatedResponse<String> = response
            .json()
            .await
            .context("Failed to parse appbundles response")?;

        Ok(paginated.data)
    }

    /// Create a new app bundle
    pub async fn create_appbundle(
        &self,
        id: &str,
        engine: &str,
        description: Option<&str>,
    ) -> Result<AppBundleDetails> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles", self.config.da_url());

        let request = CreateAppBundleRequest {
            id: id.to_string(),
            engine: engine.to_string(),
            description: description.map(|s| s.to_string()),
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to create appbundle")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create appbundle ({}): {}", status, error_text);
        }

        let appbundle: AppBundleDetails = response
            .json()
            .await
            .context("Failed to parse appbundle response")?;

        Ok(appbundle)
    }

    /// Delete an app bundle
    pub async fn delete_appbundle(&self, id: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles/{}", self.config.da_url(), id);

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to delete appbundle")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete appbundle ({}): {}", status, error_text);
        }

        Ok(())
    }

    /// List all activities
    pub async fn list_activities(&self) -> Result<Vec<String>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/activities", self.config.da_url());

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list activities")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list activities ({}): {}", status, error_text);
        }

        let paginated: PaginatedResponse<String> = response
            .json()
            .await
            .context("Failed to parse activities response")?;

        Ok(paginated.data)
    }

    /// Create a new activity
    pub async fn create_activity(&self, request: CreateActivityRequest) -> Result<Activity> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/activities", self.config.da_url());

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to create activity")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create activity ({}): {}", status, error_text);
        }

        let activity: Activity = response
            .json()
            .await
            .context("Failed to parse activity response")?;

        Ok(activity)
    }

    /// Delete an activity
    pub async fn delete_activity(&self, id: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/activities/{}", self.config.da_url(), id);

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to delete activity")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete activity ({}): {}", status, error_text);
        }

        Ok(())
    }

    /// Create a work item (run an activity)
    pub async fn create_workitem(
        &self,
        activity_id: &str,
        arguments: std::collections::HashMap<String, WorkItemArgument>,
    ) -> Result<WorkItem> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/workitems", self.config.da_url());

        let request = CreateWorkItemRequest {
            activity_id: activity_id.to_string(),
            arguments,
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to create workitem")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create workitem ({}): {}", status, error_text);
        }

        let workitem: WorkItem = response
            .json()
            .await
            .context("Failed to parse workitem response")?;

        Ok(workitem)
    }

    /// Get work item status
    pub async fn get_workitem_status(&self, id: &str) -> Result<WorkItem> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/workitems/{}", self.config.da_url(), id);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get workitem status")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get workitem status ({}): {}", status, error_text);
        }

        let workitem: WorkItem = response
            .json()
            .await
            .context("Failed to parse workitem response")?;

        Ok(workitem)
    }
}

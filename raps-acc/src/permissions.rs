// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Folder Permissions API client for ACC/BIM 360

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

/// Folder permission entry
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderPermission {
    /// Subject ID (user or company ID)
    pub subject_id: String,
    /// Subject type: "USER" or "COMPANY"
    pub subject_type: String,
    /// List of actions granted
    pub actions: Vec<String>,
    /// Inherited from parent folder
    #[serde(default)]
    pub inherited_from: Option<String>,
}

/// Request to update folder permissions
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePermissionRequest {
    /// Subject ID (user ID)
    pub subject_id: String,
    /// Subject type: "USER" or "COMPANY"
    pub subject_type: String,
    /// Actions to grant
    pub actions: Vec<String>,
}

/// Batch update permissions request
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdatePermissionsRequest {
    /// Array of permission updates
    pub permissions: Vec<UpdatePermissionRequest>,
}

/// Client for ACC Folder Permissions API
///
/// Provides operations for managing folder-level permissions within projects.
pub struct FolderPermissionsClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl FolderPermissionsClient {
    /// Create a new Folder Permissions client
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

    /// Get the base URL for Data Management API
    fn dm_url(&self) -> &str {
        &self.config.base_url
    }

    /// Get permissions for a folder
    ///
    /// # Arguments
    /// * `project_id` - The project ID (with or without "b." prefix)
    /// * `folder_id` - The folder ID (URN)
    pub async fn get_permissions(
        &self,
        project_id: &str,
        folder_id: &str,
    ) -> Result<Vec<FolderPermission>> {
        let token = self.auth.get_3leg_token().await?;
        let project_id = normalize_project_id(project_id);

        let url = format!(
            "{}/data/v1/projects/{}/folders/{}/permissions",
            self.dm_url(),
            project_id,
            folder_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get folder permissions")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get folder permissions ({status}): {error_text}");
        }

        #[derive(Deserialize)]
        struct PermissionsResponse {
            data: Vec<PermissionData>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct PermissionData {
            attributes: FolderPermission,
        }

        let perms_response: PermissionsResponse = response
            .json()
            .await
            .context("Failed to parse permissions response")?;

        Ok(perms_response
            .data
            .into_iter()
            .map(|d| d.attributes)
            .collect())
    }

    /// Update permissions for a folder
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    /// * `folder_id` - The folder ID (URN)
    /// * `request` - Permission update request
    pub async fn update_permissions(
        &self,
        project_id: &str,
        folder_id: &str,
        request: BatchUpdatePermissionsRequest,
    ) -> Result<()> {
        let token = self.auth.get_3leg_token().await?;
        let project_id = normalize_project_id(project_id);

        let url = format!(
            "{}/data/v1/projects/{}/folders/{}/permissions:batch-update",
            self.dm_url(),
            project_id,
            folder_id
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to update folder permissions")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update folder permissions ({status}): {error_text}");
        }

        Ok(())
    }

    /// Get the root folder ID for "Project Files" in a project
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    pub async fn get_project_files_folder_id(&self, project_id: &str) -> Result<String> {
        let token = self.auth.get_3leg_token().await?;
        let project_id = normalize_project_id(project_id);

        // Get top-level folders
        let url = format!(
            "{}/data/v1/projects/{}/topFolders",
            self.dm_url(),
            project_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get project folders")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get project folders ({status}): {error_text}");
        }

        #[derive(Deserialize)]
        struct TopFoldersResponse {
            data: Vec<FolderData>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FolderData {
            id: String,
            attributes: FolderAttributes,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FolderAttributes {
            display_name: Option<String>,
            name: String,
        }

        let folders_response: TopFoldersResponse = response
            .json()
            .await
            .context("Failed to parse folders response")?;

        // Look for "Project Files" folder
        for folder in folders_response.data {
            let name = folder
                .attributes
                .display_name
                .as_ref()
                .unwrap_or(&folder.attributes.name);
            if name.to_lowercase().contains("project files") {
                return Ok(folder.id);
            }
        }

        anyhow::bail!("Project Files folder not found in project {}", project_id)
    }

    /// Get the Plans folder ID in a project
    ///
    /// # Arguments
    /// * `project_id` - The project ID
    pub async fn get_plans_folder_id(&self, project_id: &str) -> Result<String> {
        let token = self.auth.get_3leg_token().await?;
        let project_id = normalize_project_id(project_id);

        // Get top-level folders
        let url = format!(
            "{}/data/v1/projects/{}/topFolders",
            self.dm_url(),
            project_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get project folders")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get project folders ({status}): {error_text}");
        }

        #[derive(Deserialize)]
        struct TopFoldersResponse {
            data: Vec<FolderData>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FolderData {
            id: String,
            attributes: FolderAttributes,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct FolderAttributes {
            display_name: Option<String>,
            name: String,
        }

        let folders_response: TopFoldersResponse = response
            .json()
            .await
            .context("Failed to parse folders response")?;

        // Look for "Plans" folder
        for folder in folders_response.data {
            let name = folder
                .attributes
                .display_name
                .as_ref()
                .unwrap_or(&folder.attributes.name);
            if name.to_lowercase().contains("plans") {
                return Ok(folder.id);
            }
        }

        anyhow::bail!("Plans folder not found in project {}", project_id)
    }

    /// Check if a user has permissions in a folder
    pub async fn user_has_permissions(
        &self,
        project_id: &str,
        folder_id: &str,
        user_id: &str,
    ) -> Result<bool> {
        let permissions = self.get_permissions(project_id, folder_id).await?;
        Ok(permissions
            .iter()
            .any(|p| p.subject_id == user_id && p.subject_type == "USER"))
    }
}

/// Normalize project ID (ensure "b." prefix)
fn normalize_project_id(project_id: &str) -> String {
    if project_id.starts_with("b.") {
        project_id.to_string()
    } else {
        format!("b.{}", project_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_project_id() {
        assert_eq!(normalize_project_id("b.123-456"), "b.123-456");
        assert_eq!(normalize_project_id("123-456"), "b.123-456");
    }

    #[test]
    fn test_update_permission_request_serialization() {
        let request = UpdatePermissionRequest {
            subject_id: "user-123".to_string(),
            subject_type: "USER".to_string(),
            actions: vec![
                "VIEW".to_string(),
                "DOWNLOAD".to_string(),
                "COLLABORATE".to_string(),
            ],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("user-123"));
        assert!(json.contains("USER"));
        assert!(json.contains("VIEW"));
    }
}

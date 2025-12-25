//! Data Management API module
//!
//! Handles access to Hubs, Projects, Folders, and Items in BIM 360/ACC.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::AuthClient;
use crate::config::Config;

/// Hub information
#[derive(Debug, Clone, Deserialize)]
pub struct Hub {
    #[serde(rename = "type")]
    pub hub_type: String,
    pub id: String,
    pub attributes: HubAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HubAttributes {
    pub name: String,
    pub region: Option<String>,
    #[serde(rename = "extension")]
    pub extension: Option<HubExtension>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HubExtension {
    #[serde(rename = "type")]
    pub extension_type: Option<String>,
}

/// Project information
#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    #[serde(rename = "type")]
    pub project_type: String,
    pub id: String,
    pub attributes: ProjectAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectAttributes {
    pub name: String,
    #[serde(rename = "scopes")]
    pub scopes: Option<Vec<String>>,
}

/// Folder information
#[derive(Debug, Clone, Deserialize)]
pub struct Folder {
    #[serde(rename = "type")]
    pub folder_type: String,
    pub id: String,
    pub attributes: FolderAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FolderAttributes {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "createTime")]
    pub create_time: Option<String>,
    #[serde(rename = "lastModifiedTime")]
    pub last_modified_time: Option<String>,
}

/// Item (file) information
#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    #[serde(rename = "type")]
    pub item_type: String,
    pub id: String,
    pub attributes: ItemAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemAttributes {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "createTime")]
    pub create_time: Option<String>,
    #[serde(rename = "lastModifiedTime")]
    pub last_modified_time: Option<String>,
    #[serde(rename = "extension")]
    pub extension: Option<ItemExtension>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemExtension {
    #[serde(rename = "type")]
    pub extension_type: Option<String>,
    pub version: Option<String>,
}

/// Version information for an item
#[derive(Debug, Clone, Deserialize)]
pub struct Version {
    #[serde(rename = "type")]
    pub version_type: String,
    pub id: String,
    pub attributes: VersionAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionAttributes {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "versionNumber")]
    pub version_number: Option<i32>,
    #[serde(rename = "createTime")]
    pub create_time: Option<String>,
    #[serde(rename = "storageSize")]
    pub storage_size: Option<i64>,
}

/// Folder contents (can be folders or items)
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FolderContent {
    Folder(Folder),
    Item(Item),
}

/// JSON:API response wrapper
#[derive(Debug, Deserialize)]
pub struct JsonApiResponse<T> {
    pub data: T,
    #[serde(default)]
    pub included: Vec<serde_json::Value>,
    pub links: Option<JsonApiLinks>,
}

#[derive(Debug, Deserialize)]
pub struct JsonApiLinks {
    #[serde(rename = "self")]
    pub self_link: Option<JsonApiLink>,
    pub next: Option<JsonApiLink>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum JsonApiLink {
    Simple(String),
    Complex { href: String },
}

/// Request to create a folder
#[derive(Debug, Serialize)]
pub struct CreateFolderRequest {
    pub jsonapi: JsonApiVersion,
    pub data: CreateFolderData,
}

#[derive(Debug, Serialize)]
pub struct JsonApiVersion {
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct CreateFolderData {
    #[serde(rename = "type")]
    pub data_type: String,
    pub attributes: CreateFolderAttributes,
    pub relationships: CreateFolderRelationships,
}

#[derive(Debug, Serialize)]
pub struct CreateFolderAttributes {
    pub name: String,
    pub extension: CreateFolderExtension,
}

#[derive(Debug, Serialize)]
pub struct CreateFolderExtension {
    #[serde(rename = "type")]
    pub ext_type: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct CreateFolderRelationships {
    pub parent: CreateFolderParent,
}

#[derive(Debug, Serialize)]
pub struct CreateFolderParent {
    pub data: CreateFolderParentData,
}

#[derive(Debug, Serialize)]
pub struct CreateFolderParentData {
    #[serde(rename = "type")]
    pub data_type: String,
    pub id: String,
}

/// Data Management API client
pub struct DataManagementClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl DataManagementClient {
    /// Create a new Data Management client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, crate::http::HttpClientConfig::default())
    }

    /// Create a new Data Management client with custom HTTP config
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

    /// List all accessible hubs (requires 3-legged auth)
    pub async fn list_hubs(&self) -> Result<Vec<Hub>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/hubs", self.config.project_url());

        // Log request in verbose/debug mode
        crate::logging::log_request("GET", &url);

        // Use retry logic for API requests
        let http_config = crate::http::HttpClientConfig::default();
        let response = crate::http::execute_with_retry(&http_config, || {
            let client = self.http_client.clone();
            let url = url.clone();
            let token = token.clone();
            Box::pin(async move {
                client
                    .get(&url)
                    .bearer_auth(&token)
                    .send()
                    .await
                    .context("Failed to list hubs")
            })
        })
        .await?;

        // Log response in verbose/debug mode
        crate::logging::log_response(response.status().as_u16(), &url);

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list hubs ({}): {}", status, error_text);
        }

        let api_response: JsonApiResponse<Vec<Hub>> = response
            .json()
            .await
            .context("Failed to parse hubs response")?;

        Ok(api_response.data)
    }

    /// Get hub details
    pub async fn get_hub(&self, hub_id: &str) -> Result<Hub> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/hubs/{}", self.config.project_url(), hub_id);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get hub")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get hub ({}): {}", status, error_text);
        }

        let api_response: JsonApiResponse<Hub> = response
            .json()
            .await
            .context("Failed to parse hub response")?;

        Ok(api_response.data)
    }

    /// List projects in a hub
    pub async fn list_projects(&self, hub_id: &str) -> Result<Vec<Project>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/hubs/{}/projects", self.config.project_url(), hub_id);

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
            anyhow::bail!("Failed to list projects ({}): {}", status, error_text);
        }

        let api_response: JsonApiResponse<Vec<Project>> = response
            .json()
            .await
            .context("Failed to parse projects response")?;

        Ok(api_response.data)
    }

    /// Get project details
    pub async fn get_project(&self, hub_id: &str, project_id: &str) -> Result<Project> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/hubs/{}/projects/{}",
            self.config.project_url(),
            hub_id,
            project_id
        );

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
            anyhow::bail!("Failed to get project ({}): {}", status, error_text);
        }

        let api_response: JsonApiResponse<Project> = response
            .json()
            .await
            .context("Failed to parse project response")?;

        Ok(api_response.data)
    }

    /// Get top folders for a project
    pub async fn get_top_folders(&self, hub_id: &str, project_id: &str) -> Result<Vec<Folder>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/hubs/{}/projects/{}/topFolders",
            self.config.project_url(),
            hub_id,
            project_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get top folders")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get top folders ({}): {}", status, error_text);
        }

        let api_response: JsonApiResponse<Vec<Folder>> = response
            .json()
            .await
            .context("Failed to parse folders response")?;

        Ok(api_response.data)
    }

    /// List folder contents
    pub async fn list_folder_contents(
        &self,
        project_id: &str,
        folder_id: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/folders/{}/contents",
            self.config.data_url(),
            project_id,
            folder_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to list folder contents")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to list folder contents ({}): {}",
                status,
                error_text
            );
        }

        let api_response: JsonApiResponse<Vec<serde_json::Value>> = response
            .json()
            .await
            .context("Failed to parse folder contents")?;

        Ok(api_response.data)
    }

    /// Create a new folder
    pub async fn create_folder(
        &self,
        project_id: &str,
        parent_folder_id: &str,
        name: &str,
    ) -> Result<Folder> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/projects/{}/folders", self.config.data_url(), project_id);

        let request = CreateFolderRequest {
            jsonapi: JsonApiVersion {
                version: "1.0".to_string(),
            },
            data: CreateFolderData {
                data_type: "folders".to_string(),
                attributes: CreateFolderAttributes {
                    name: name.to_string(),
                    extension: CreateFolderExtension {
                        ext_type: "folders:autodesk.core:Folder".to_string(),
                        version: "1.0".to_string(),
                    },
                },
                relationships: CreateFolderRelationships {
                    parent: CreateFolderParent {
                        data: CreateFolderParentData {
                            data_type: "folders".to_string(),
                            id: parent_folder_id.to_string(),
                        },
                    },
                },
            },
        };

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/vnd.api+json")
            .json(&request)
            .send()
            .await
            .context("Failed to create folder")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create folder ({}): {}", status, error_text);
        }

        let api_response: JsonApiResponse<Folder> = response
            .json()
            .await
            .context("Failed to parse folder response")?;

        Ok(api_response.data)
    }

    /// Get item details
    pub async fn get_item(&self, project_id: &str, item_id: &str) -> Result<Item> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}",
            self.config.data_url(),
            project_id,
            item_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get item")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get item ({}): {}", status, error_text);
        }

        let api_response: JsonApiResponse<Item> = response
            .json()
            .await
            .context("Failed to parse item response")?;

        Ok(api_response.data)
    }

    /// Get item versions
    pub async fn get_item_versions(&self, project_id: &str, item_id: &str) -> Result<Vec<Version>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}/versions",
            self.config.data_url(),
            project_id,
            item_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get item versions")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get item versions ({}): {}", status, error_text);
        }

        let api_response: JsonApiResponse<Vec<Version>> = response
            .json()
            .await
            .context("Failed to parse versions response")?;

        Ok(api_response.data)
    }
}

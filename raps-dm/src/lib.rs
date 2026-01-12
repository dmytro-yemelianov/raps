// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! Data Management API module
//!
//! Handles access to Hubs, Projects, Folders, and Items in BIM 360/ACC.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;
use raps_kernel::logging;

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
#[derive(Clone)]
pub struct DataManagementClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl DataManagementClient {
    /// Create a new Data Management client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new Data Management client with custom HTTP config
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

    /// List all accessible hubs (requires 3-legged auth)
    pub async fn list_hubs(&self) -> Result<Vec<Hub>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/hubs", self.config.project_url());

        // Log request in verbose/debug mode
        logging::log_request("GET", &url);

        // Use retry logic for API requests
        let http_config = HttpClientConfig::default();
        let response = raps_kernel::http::execute_with_retry(&http_config, || {
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
        logging::log_response(response.status().as_u16(), &url);

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list hubs ({status}): {error_text}");
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
            anyhow::bail!("Failed to get hub ({status}): {error_text}");
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
            anyhow::bail!("Failed to list projects ({status}): {error_text}");
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
            anyhow::bail!("Failed to get project ({status}): {error_text}");
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
            anyhow::bail!("Failed to get top folders ({status}): {error_text}");
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
            anyhow::bail!("Failed to create folder ({status}): {error_text}");
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
            anyhow::bail!("Failed to get item ({status}): {error_text}");
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
            anyhow::bail!("Failed to get item versions ({status}): {error_text}");
        }

        let api_response: JsonApiResponse<Vec<Version>> = response
            .json()
            .await
            .context("Failed to parse versions response")?;

        Ok(api_response.data)
    }

    /// Create an item from OSS storage object
    /// This binds an OSS object to a folder in ACC/BIM 360
    pub async fn create_item_from_storage(
        &self,
        project_id: &str,
        folder_id: &str,
        display_name: &str,
        storage_id: &str,
    ) -> Result<Item> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!("{}/projects/{}/items", self.config.data_url(), project_id);

        // Build JSON:API request for creating an item
        let request = serde_json::json!({
            "jsonapi": {
                "version": "1.0"
            },
            "data": {
                "type": "items",
                "attributes": {
                    "displayName": display_name,
                    "extension": {
                        "type": "items:autodesk.core:File",
                        "version": "1.0"
                    }
                },
                "relationships": {
                    "tip": {
                        "data": {
                            "type": "versions",
                            "id": "1"
                        }
                    },
                    "parent": {
                        "data": {
                            "type": "folders",
                            "id": folder_id
                        }
                    }
                }
            },
            "included": [
                {
                    "type": "versions",
                    "id": "1",
                    "attributes": {
                        "name": display_name,
                        "extension": {
                            "type": "versions:autodesk.core:File",
                            "version": "1.0"
                        }
                    },
                    "relationships": {
                        "storage": {
                            "data": {
                                "type": "objects",
                                "id": storage_id
                            }
                        }
                    }
                }
            ]
        });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/vnd.api+json")
            .json(&request)
            .send()
            .await
            .context("Failed to create item from storage")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to create item from storage ({}): {}",
                status,
                error_text
            );
        }

        let api_response: JsonApiResponse<Item> = response
            .json()
            .await
            .context("Failed to parse item response")?;

        Ok(api_response.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hub_deserialization() {
        let json = r#"{
            "type": "hubs",
            "id": "b.hub-id",
            "attributes": {
                "name": "Test Hub",
                "region": "US"
            }
        }"#;

        let hub: Hub = serde_json::from_str(json).unwrap();
        assert_eq!(hub.hub_type, "hubs");
        assert_eq!(hub.id, "b.hub-id");
        assert_eq!(hub.attributes.name, "Test Hub");
    }

    #[test]
    fn test_project_deserialization() {
        let json = r#"{
            "type": "projects",
            "id": "b.project-id",
            "attributes": {
                "name": "Test Project"
            }
        }"#;

        let project: Project = serde_json::from_str(json).unwrap();
        assert_eq!(project.project_type, "projects");
        assert_eq!(project.attributes.name, "Test Project");
    }

    #[test]
    fn test_folder_deserialization() {
        let json = r#"{
            "type": "folders",
            "id": "urn:adsk.wipprod:folder.id",
            "attributes": {
                "name": "Project Files"
            }
        }"#;

        let folder: Folder = serde_json::from_str(json).unwrap();
        assert_eq!(folder.folder_type, "folders");
        assert_eq!(folder.attributes.name, "Project Files");
    }

    #[test]
    fn test_item_deserialization() {
        let json = r#"{
            "type": "items",
            "id": "urn:adsk.wipprod:dm.lineage:item-id",
            "attributes": {
                "displayName": "model.rvt"
            }
        }"#;

        let item: Item = serde_json::from_str(json).unwrap();
        assert_eq!(item.item_type, "items");
        assert_eq!(item.attributes.display_name, "model.rvt");
    }

    #[test]
    fn test_version_deserialization() {
        let json = r#"{
            "type": "versions",
            "id": "urn:adsk.wipprod:fs.file:version-id",
            "attributes": {
                "name": "model.rvt",
                "displayName": "model.rvt",
                "versionNumber": 1
            }
        }"#;

        let version: Version = serde_json::from_str(json).unwrap();
        assert_eq!(version.version_type, "versions");
        assert_eq!(version.attributes.version_number, Some(1));
    }

    #[test]
    fn test_create_folder_request_serialization() {
        let request = CreateFolderRequest {
            jsonapi: JsonApiVersion {
                version: "1.0".to_string(),
            },
            data: CreateFolderData {
                data_type: "folders".to_string(),
                attributes: CreateFolderAttributes {
                    name: "New Folder".to_string(),
                    extension: CreateFolderExtension {
                        ext_type: "folders:autodesk.bim360:Folder".to_string(),
                        version: "1.0".to_string(),
                    },
                },
                relationships: CreateFolderRelationships {
                    parent: CreateFolderParent {
                        data: CreateFolderParentData {
                            data_type: "folders".to_string(),
                            id: "parent-folder-id".to_string(),
                        },
                    },
                },
            },
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["jsonapi"]["version"], "1.0");
        assert_eq!(json["data"]["type"], "folders");
        assert_eq!(json["data"]["attributes"]["name"], "New Folder");
    }

    #[test]
    fn test_hub_with_region() {
        let json = r#"{
            "type": "hubs",
            "id": "b.hub-id",
            "attributes": {
                "name": "Test Hub",
                "region": "US"
            }
        }"#;

        let hub: Hub = serde_json::from_str(json).unwrap();
        assert_eq!(hub.attributes.region, Some("US".to_string()));
    }

    #[test]
    fn test_project_with_scopes() {
        let json = r#"{
            "type": "projects",
            "id": "b.project-id",
            "attributes": {
                "name": "Test Project",
                "scopes": ["docs:read", "docs:write"]
            }
        }"#;

        let project: Project = serde_json::from_str(json).unwrap();
        assert!(project.attributes.scopes.is_some());
        let scopes = project.attributes.scopes.unwrap();
        assert_eq!(scopes.len(), 2);
    }

    #[test]
    fn test_folder_with_display_name() {
        let json = r#"{
            "type": "folders",
            "id": "folder-id",
            "attributes": {
                "name": "folder",
                "displayName": "Project Files"
            }
        }"#;

        let folder: Folder = serde_json::from_str(json).unwrap();
        assert_eq!(
            folder.attributes.display_name,
            Some("Project Files".to_string())
        );
    }

    #[test]
    fn test_item_with_extension() {
        let json = r#"{
            "type": "items",
            "id": "item-id",
            "attributes": {
                "displayName": "model.rvt",
                "extension": {
                    "type": "items:autodesk.bim360:File",
                    "version": "1.0"
                }
            }
        }"#;

        let item: Item = serde_json::from_str(json).unwrap();
        assert!(item.attributes.extension.is_some());
        let ext = item.attributes.extension.unwrap();
        assert_eq!(ext.extension_type, Some("items:autodesk.bim360:File".to_string()));
    }

    #[test]
    fn test_version_with_storage_size() {
        let json = r#"{
            "type": "versions",
            "id": "version-id",
            "attributes": {
                "name": "model.rvt",
                "displayName": "model.rvt",
                "versionNumber": 2,
                "storageSize": 1048576
            }
        }"#;

        let version: Version = serde_json::from_str(json).unwrap();
        assert_eq!(version.attributes.storage_size, Some(1048576));
        assert_eq!(version.attributes.version_number, Some(2));
    }
}

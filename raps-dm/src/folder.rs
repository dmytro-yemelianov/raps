// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Folder operations

use crate::types::*;
use raps_kernel::{AuthClient, Config, HttpClient, RapsError, Result};

/// Folder client for Data Management operations
pub struct FolderClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
    project_url: String,
    data_url: String,
}

impl FolderClient {
    /// Create new folder client
    pub fn new(
        http: HttpClient,
        auth: AuthClient,
        config: Config,
        project_url: String,
        data_url: String,
    ) -> Self {
        Self {
            http,
            auth,
            config,
            project_url,
            data_url,
        }
    }

    /// Get top folders for a project
    pub async fn get_top_folders(&self, hub_id: &str, project_id: &str) -> Result<Vec<Folder>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/hubs/{}/projects/{}/topFolders",
            self.project_url, hub_id, project_id
        );

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to get top folders".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to get top folders ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let api_response: JsonApiResponse<Vec<Folder>> =
            response.json().await.map_err(|e| RapsError::Internal {
                message: format!("Failed to parse folders response: {}", e),
            })?;

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
            self.data_url, project_id, folder_id
        );

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to list folder contents".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!(
                    "Failed to list folder contents ({}): {}",
                    status, error_text
                ),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let api_response: JsonApiResponse<Vec<serde_json::Value>> =
            response.json().await.map_err(|e| RapsError::Internal {
                message: format!("Failed to parse folder contents: {}", e),
            })?;

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
        let url = format!("{}/projects/{}/folders", self.data_url, project_id);

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
            .http
            .inner()
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/vnd.api+json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to create folder".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to create folder ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let api_response: JsonApiResponse<Folder> =
            response.json().await.map_err(|e| RapsError::Internal {
                message: format!("Failed to parse folder response: {}", e),
            })?;

        Ok(api_response.data)
    }
}

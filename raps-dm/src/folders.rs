// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Folder operations for the Data Management API.

use anyhow::{Context, Result};

use crate::types::*;
use crate::{DataManagementClient, MAX_PAGINATION_PAGES};

impl DataManagementClient {
    /// List folder contents
    ///
    /// Follows pagination links to return complete result set (max 100 pages).
    pub async fn list_folder_contents(
        &self,
        project_id: &str,
        folder_id: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let token = self.auth.get_3leg_token().await?;
        let mut next_url = Some(format!(
            "{}/projects/{}/folders/{}/contents",
            self.config.data_url(),
            project_id,
            folder_id
        ));
        let mut all_items = Vec::new();

        for _page in 0..MAX_PAGINATION_PAGES {
            let url = match next_url.take() {
                Some(u) => u,
                None => break,
            };

            let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
                self.http_client.get(&url).bearer_auth(&token)
            })
            .await?;

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

            all_items.extend(api_response.data);

            next_url = api_response
                .links
                .and_then(|l| l.next)
                .map(|link| link.href().to_string());
        }

        Ok(all_items)
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
                        // BIM360 projects (b. prefix) require bim360 extension type
                        ext_type: if project_id.starts_with("b.") {
                            "folders:autodesk.bim360:Folder".to_string()
                        } else {
                            "folders:autodesk.core:Folder".to_string()
                        },
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

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/vnd.api+json")
                .json(&request)
        })
        .await?;

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

    /// Rename a folder
    ///
    /// Updates the folder's name using the JSON:API PATCH endpoint.
    pub async fn rename_folder(
        &self,
        project_id: &str,
        folder_id: &str,
        new_name: &str,
    ) -> Result<Folder> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/folders/{}",
            self.config.data_url(),
            project_id,
            folder_id
        );

        // Build JSON:API PATCH request for updating folder
        let request = serde_json::json!({
            "jsonapi": {
                "version": "1.0"
            },
            "data": {
                "type": "folders",
                "id": folder_id,
                "attributes": {
                    "name": new_name
                }
            }
        });

        // Log request in verbose/debug mode
        tracing::info!(method = "PATCH", url = %raps_kernel::logging::redact_secrets(&url), "HTTP request");

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .patch(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/vnd.api+json")
                .json(&request)
        })
        .await?;

        // Log response in verbose/debug mode
        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to rename folder ({status}): {error_text}");
        }

        let api_response: JsonApiResponse<Folder> = response
            .json()
            .await
            .context("Failed to parse folder response")?;

        Ok(api_response.data)
    }

    /// Delete a folder from a project
    ///
    /// This removes the folder from the project.
    pub async fn delete_folder(&self, project_id: &str, folder_id: &str) -> Result<()> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/folders/{}",
            self.config.data_url(),
            project_id,
            folder_id
        );

        // Log request in verbose/debug mode
        tracing::info!(method = "DELETE", url = %raps_kernel::logging::redact_secrets(&url), "HTTP request");

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        // Log response in verbose/debug mode
        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete folder ({status}): {error_text}");
        }

        Ok(())
    }
}

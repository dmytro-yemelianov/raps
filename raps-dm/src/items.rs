// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Item (file) operations for the Data Management API.

use anyhow::{Context, Result};

use crate::types::*;
use crate::{DataManagementClient, MAX_PAGINATION_PAGES};

impl DataManagementClient {
    /// Get item details
    pub async fn get_item(&self, project_id: &str, item_id: &str) -> Result<Item> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}",
            self.config.data_url(),
            project_id,
            item_id
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

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
        let mut next_url = Some(format!(
            "{}/projects/{}/items/{}/versions",
            self.config.data_url(),
            project_id,
            item_id
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
                anyhow::bail!("Failed to get item versions ({status}): {error_text}");
            }

            let api_response: JsonApiResponse<Vec<Version>> = response
                .json()
                .await
                .context("Failed to parse versions response")?;

            all_items.extend(api_response.data);

            next_url = api_response
                .links
                .and_then(|l| l.next)
                .map(|link| link.href().to_string());
        }

        Ok(all_items)
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

    /// Delete an item from a project
    ///
    /// This removes the item (file lineage) from the project folder.
    /// Note: This does not delete the underlying OSS object.
    pub async fn delete_item(&self, project_id: &str, item_id: &str) -> Result<()> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}",
            self.config.data_url(),
            project_id,
            item_id
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
            anyhow::bail!("Failed to delete item ({status}): {error_text}");
        }

        Ok(())
    }

    /// Rename an item (update display name)
    ///
    /// Updates the item's display name without changing the file content.
    pub async fn rename_item(
        &self,
        project_id: &str,
        item_id: &str,
        new_name: &str,
    ) -> Result<Item> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}",
            self.config.data_url(),
            project_id,
            item_id
        );

        // Build JSON:API PATCH request for updating item
        let request = serde_json::json!({
            "jsonapi": {
                "version": "1.0"
            },
            "data": {
                "type": "items",
                "id": item_id,
                "attributes": {
                    "displayName": new_name
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
            anyhow::bail!("Failed to rename item ({status}): {error_text}");
        }

        let api_response: JsonApiResponse<Item> = response
            .json()
            .await
            .context("Failed to parse item response")?;

        Ok(api_response.data)
    }
}

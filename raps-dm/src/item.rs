// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Item operations

use crate::types::*;
use raps_kernel::{AuthClient, Config, HttpClient, RapsError, Result};

/// Item client for Data Management operations
pub struct ItemClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
    data_url: String,
}

impl ItemClient {
    /// Create new item client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config, data_url: String) -> Self {
        Self {
            http,
            auth,
            config,
            data_url,
        }
    }

    /// Get item details
    pub async fn get_item(&self, project_id: &str, item_id: &str) -> Result<Item> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}",
            self.data_url, project_id, item_id
        );

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to get item".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to get item ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let api_response: JsonApiResponse<Item> =
            response.json().await.map_err(|e| RapsError::Internal {
                message: format!("Failed to parse item response: {}", e),
            })?;

        Ok(api_response.data)
    }

    /// Get item versions
    pub async fn get_item_versions(&self, project_id: &str, item_id: &str) -> Result<Vec<Version>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/items/{}/versions",
            self.data_url, project_id, item_id
        );

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to get item versions".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to get item versions ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let api_response: JsonApiResponse<Vec<Version>> =
            response.json().await.map_err(|e| RapsError::Internal {
                message: format!("Failed to parse versions response: {}", e),
            })?;

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
        let url = format!("{}/projects/{}/items", self.data_url, project_id);

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
            .http
            .inner()
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/vnd.api+json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to create item from storage".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!(
                    "Failed to create item from storage ({}): {}",
                    status, error_text
                ),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let api_response: JsonApiResponse<Item> =
            response.json().await.map_err(|e| RapsError::Internal {
                message: format!("Failed to parse item response: {}", e),
            })?;

        Ok(api_response.data)
    }
}

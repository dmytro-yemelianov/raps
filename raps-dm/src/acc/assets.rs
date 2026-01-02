// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC Assets API

use raps_kernel::{AuthClient, Config, HttpClient, RapsError, Result};
use serde::{Deserialize, Serialize};

/// Asset information
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    /// Unique asset ID
    pub id: String,
    /// Category ID
    pub category_id: Option<String>,
    /// Status ID
    pub status_id: Option<String>,
    /// Client asset ID
    pub client_asset_id: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Barcode
    pub barcode: Option<String>,
    /// Created timestamp
    pub created_at: Option<String>,
    /// Updated timestamp
    pub updated_at: Option<String>,
}

/// Assets client for ACC Assets API
pub struct AssetsClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
}

impl AssetsClient {
    /// Create a new assets client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config) -> Self {
        Self { http, auth, config }
    }

    /// List assets in a project
    pub async fn list(&self, project_id: &str) -> Result<Vec<Asset>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "https://developer.api.autodesk.com/construction/assets/v1/projects/{}/assets",
            project_id
        );

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to list assets".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to list assets ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        #[derive(Deserialize)]
        struct AssetsResponse {
            results: Vec<Asset>,
        }

        let resp: AssetsResponse = response.json().await.map_err(|e| RapsError::Internal {
            message: format!("Failed to parse assets response: {}", e),
        })?;

        Ok(resp.results)
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Reality Capture API
//!
//! Provides photogrammetry capabilities for creating 3D models from photos.

use raps_kernel::{AuthClient, Config, HttpClient, RapsError, Result};
use serde::{Deserialize, Serialize};

/// Photoscene information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Photoscene {
    /// Photoscene ID
    pub id: String,
    /// Scene name
    pub name: String,
    /// Current status
    pub status: String,
    /// Progress percentage (0-100)
    pub progress: Option<i32>,
}

/// Reality Capture client
pub struct RealityCaptureClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
}

impl RealityCaptureClient {
    /// Create a new Reality Capture client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config) -> Self {
        Self { http, auth, config }
    }

    /// Create a new photoscene
    pub async fn create_photoscene(&self, name: &str) -> Result<Photoscene> {
        let token = self.auth.get_token().await?;
        let url = "https://developer.api.autodesk.com/photo-to-3d/v1/photoscene";

        let response = self
            .http
            .inner()
            .post(url)
            .bearer_auth(&token)
            .form(&[("scenename", name)])
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to create photoscene".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to create photoscene ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        #[derive(Deserialize)]
        struct PhotosceneResponse {
            #[serde(rename = "Photoscene")]
            photoscene: PhotosceneData,
        }

        #[derive(Deserialize)]
        struct PhotosceneData {
            photosceneid: String,
        }

        let resp: PhotosceneResponse = response.json().await.map_err(|e| RapsError::Internal {
            message: format!("Failed to parse photoscene response: {}", e),
        })?;

        Ok(Photoscene {
            id: resp.photoscene.photosceneid,
            name: name.to_string(),
            status: "created".to_string(),
            progress: Some(0),
        })
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Design Automation API
//!
//! Provides access to Autodesk Design Automation APIs:
//! - Engines (AutoCAD, Revit, Inventor, 3ds Max)
//! - App Bundles
//! - Activities
//! - Work Items

use raps_kernel::{AuthClient, Config, HttpClient, Result, RapsError};
use serde::{Deserialize, Serialize};

/// Engine information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Engine {
    /// Engine ID
    pub id: String,
    /// Engine description
    pub description: Option<String>,
}

/// App Bundle information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppBundle {
    /// App bundle ID
    pub id: String,
    /// Version number
    pub version: i32,
    /// Description
    pub description: Option<String>,
}

/// Activity information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Activity {
    /// Activity ID
    pub id: String,
    /// Version number
    pub version: i32,
    /// Description
    pub description: Option<String>,
}

/// Work Item status
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkItem {
    /// Work item ID
    pub id: String,
    /// Current status
    pub status: String,
    /// Report URL
    pub report_url: Option<String>,
}

/// Design Automation client
pub struct DesignAutomationClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
}

impl DesignAutomationClient {
    /// Create a new Design Automation client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config) -> Self {
        Self { http, auth, config }
    }

    /// List available engines
    pub async fn list_engines(&self) -> Result<Vec<Engine>> {
        let token = self.auth.get_token().await?;
        let url = "https://developer.api.autodesk.com/da/us-east/v3/engines";

        let response = self.http.inner()
            .get(url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to list engines".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to list engines ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        #[derive(Deserialize)]
        struct EnginesResponse {
            data: Vec<String>,
        }

        let resp: EnginesResponse = response.json().await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse engines response: {}", e),
            })?;

        Ok(resp.data.into_iter().map(|id| Engine { id, description: None }).collect())
    }
}

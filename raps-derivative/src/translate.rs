// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Translation operations

use crate::types::*;
use raps_kernel::{AuthClient, Config, HttpClient, RapsError, Result};

/// Translation client for Model Derivative operations
pub struct TranslateClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
    base_url: String,
}

impl TranslateClient {
    /// Create new translation client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config, base_url: String) -> Self {
        Self {
            http,
            auth,
            config,
            base_url,
        }
    }

    /// Start a translation job
    pub async fn translate(
        &self,
        urn: &str,
        format: OutputFormat,
        root_filename: Option<&str>,
    ) -> Result<TranslationResponse> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/designdata/job", self.base_url);

        let request = TranslationRequest {
            input: TranslationInput {
                urn: urn.to_string(),
                compressed_urn: None,
                root_filename: root_filename.map(|s| s.to_string()),
            },
            output: TranslationOutput {
                destination: OutputDestination {
                    region: "us".to_string(),
                },
                formats: vec![OutputFormatSpec {
                    format_type: format.type_name().to_string(),
                    views: if matches!(format, OutputFormat::Svf2 | OutputFormat::Svf) {
                        Some(vec!["2d".to_string(), "3d".to_string()])
                    } else {
                        None
                    },
                }],
            },
        };

        let response = self
            .http
            .inner()
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .header("x-ads-force", "true")
            .json(&request)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to start translation".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to start translation ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let translation_response: TranslationResponse =
            response.json().await.map_err(|e| RapsError::Internal {
                message: format!("Failed to parse translation response: {}", e),
            })?;

        Ok(translation_response)
    }
}

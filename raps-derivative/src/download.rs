// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Download operations

use crate::manifest::ManifestClient;
use crate::types::*;
use futures_util::StreamExt;
use raps_kernel::{AuthClient, Config, HttpClient, RapsError, Result};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Download client for Model Derivative operations
pub struct DownloadClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
    base_url: String,
}

impl DownloadClient {
    /// Create new download client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config, base_url: String) -> Self {
        Self {
            http,
            auth,
            config,
            base_url,
        }
    }

    /// Download a derivative to a local file
    pub async fn download_derivative(
        &self,
        source_urn: &str,
        derivative_urn: &str,
        output_path: &Path,
    ) -> Result<u64> {
        let token = self.auth.get_token().await?;

        // The derivative URN needs to be URL-encoded
        let encoded_derivative_urn = urlencoding::encode(derivative_urn);
        let url = format!(
            "{}/designdata/{}/manifest/{}",
            self.base_url, source_urn, encoded_derivative_urn
        );

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to download derivative".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to download derivative ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let total_size = response.content_length().unwrap_or(0);

        // Create parent directories if needed
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| RapsError::Internal {
                    message: format!("Failed to create output directory: {}", e),
                })?;
        }

        // Stream download
        let mut file = File::create(output_path)
            .await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to create output file: {}", e),
            })?;

        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| RapsError::Network {
                message: format!("Error while downloading: {}", e),
                source: Some(e),
            })?;
            file.write_all(&chunk)
                .await
                .map_err(|e| RapsError::Internal {
                    message: format!("Failed to write to file: {}", e),
                })?;
            downloaded += chunk.len() as u64;
        }

        Ok(downloaded)
    }

    /// Download all derivatives matching a format
    pub async fn download_derivatives_by_format(
        &self,
        source_urn: &str,
        format: &str,
        output_dir: &Path,
        downloadables: &[DownloadableDerivative],
    ) -> Result<Vec<(String, u64)>> {
        let filtered = ManifestClient::filter_by_format(downloadables, format);

        if filtered.is_empty() {
            return Err(RapsError::NotFound {
                resource: format!("derivatives with format '{}'", format),
            });
        }

        let mut results = Vec::new();

        for derivative in filtered {
            let output_path = output_dir.join(&derivative.name);
            let size = self
                .download_derivative(source_urn, &derivative.urn, &output_path)
                .await?;
            results.push((derivative.name, size));
        }

        Ok(results)
    }
}

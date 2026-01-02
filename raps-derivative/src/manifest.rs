// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Manifest operations

use crate::types::*;
use raps_kernel::{AuthClient, Config, HttpClient, RapsError, Result};

/// Manifest client for Model Derivative operations
pub struct ManifestClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
    base_url: String,
}

impl ManifestClient {
    /// Create new manifest client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config, base_url: String) -> Self {
        Self {
            http,
            auth,
            config,
            base_url,
        }
    }

    /// Get the manifest (translation status and available derivatives)
    pub async fn get_manifest(&self, urn: &str) -> Result<Manifest> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/designdata/{}/manifest", self.base_url, urn);

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to get manifest".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to get manifest ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let manifest: Manifest = response.json().await.map_err(|e| RapsError::Internal {
            message: format!("Failed to parse manifest response: {}", e),
        })?;

        Ok(manifest)
    }

    /// Delete manifest (and all derivatives)
    pub async fn delete_manifest(&self, urn: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/designdata/{}/manifest", self.base_url, urn);

        let response = self
            .http
            .inner()
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to delete manifest".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to delete manifest ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        Ok(())
    }

    /// Check translation status and return progress percentage
    pub async fn get_status(&self, urn: &str) -> Result<(String, String)> {
        let manifest = self.get_manifest(urn).await?;
        Ok((manifest.status, manifest.progress))
    }

    /// Get list of downloadable derivatives from manifest
    pub async fn list_downloadable_derivatives(
        &self,
        urn: &str,
    ) -> Result<Vec<DownloadableDerivative>> {
        let manifest = self.get_manifest(urn).await?;
        let mut downloadables = Vec::new();

        for derivative in &manifest.derivatives {
            Self::collect_downloadables(derivative, &derivative.output_type, &mut downloadables);
        }

        Ok(downloadables)
    }

    /// Recursively collect downloadable items from derivative tree
    fn collect_downloadables(
        derivative: &Derivative,
        output_type: &str,
        downloadables: &mut Vec<DownloadableDerivative>,
    ) {
        for child in &derivative.children {
            Self::collect_downloadables_from_child(child, output_type, downloadables);
        }
    }

    /// Recursively collect downloadable items from child nodes
    fn collect_downloadables_from_child(
        child: &DerivativeChild,
        output_type: &str,
        downloadables: &mut Vec<DownloadableDerivative>,
    ) {
        // Check if this child has a URN (is downloadable)
        if let Some(ref urn) = child.urn {
            let name = child.name.clone().unwrap_or_else(|| {
                // Generate name from GUID and type
                format!(
                    "{}.{}",
                    &child.guid[..8.min(child.guid.len())],
                    output_type.to_lowercase()
                )
            });

            downloadables.push(DownloadableDerivative {
                guid: child.guid.clone(),
                name,
                output_type: output_type.to_string(),
                role: child.role.clone(),
                urn: urn.clone(),
                mime: child.mime.clone(),
                size: child.size,
            });
        }

        // Recurse into children
        for grandchild in &child.children {
            Self::collect_downloadables_from_child(grandchild, output_type, downloadables);
        }
    }

    /// Filter derivatives by format (output type)
    pub fn filter_by_format(
        derivatives: &[DownloadableDerivative],
        format: &str,
    ) -> Vec<DownloadableDerivative> {
        let target_format = format.to_ascii_lowercase();

        derivatives
            .iter()
            .filter(|d| d.output_type.to_ascii_lowercase() == target_format)
            .cloned()
            .collect()
    }

    /// Filter derivatives by GUID
    pub fn filter_by_guid(
        derivatives: &[DownloadableDerivative],
        guid: &str,
    ) -> Option<DownloadableDerivative> {
        derivatives.iter().find(|d| d.guid == guid).cloned()
    }
}

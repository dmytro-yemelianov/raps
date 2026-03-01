// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Translation operations for the Model Derivative API.

use anyhow::{Context, Result};
use std::path::Path;

use crate::DerivativeClient;
use crate::types::*;

impl DerivativeClient {
    /// Start a translation job
    pub async fn translate(
        &self,
        urn: &str,
        format: OutputFormat,
        root_filename: Option<&str>,
        region: MdRegion,
        force: bool,
    ) -> Result<TranslationResponse> {
        let token = self.auth.get_token().await?;
        let job_url = format!("{}/designdata/job", self.config.derivative_url());

        let request = TranslationRequest {
            input: TranslationInput {
                urn: urn.to_string(),
                compressed_urn: None,
                root_filename: root_filename.map(|s| s.to_string()),
            },
            output: TranslationOutput {
                destination: OutputDestination {
                    region: region.to_string().to_lowercase(),
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

        // Log request in verbose/debug mode
        tracing::info!(method = "POST", url = %raps_kernel::logging::redact_secrets(&job_url), "HTTP request");

        // Use retry logic for translation requests
        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            let mut req = self
                .http_client
                .post(&job_url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .header("x-ads-region", region.to_string());
            if force {
                req = req.header("x-ads-force", "true");
            }
            req.json(&request)
        })
        .await?;

        // Log response in verbose/debug mode
        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&job_url), "HTTP response");

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to start translation ({status}): {error_text}");
        }

        let translation_response: TranslationResponse = response
            .json()
            .await
            .context("Failed to parse translation response")?;

        Ok(translation_response)
    }

    /// Get the manifest (translation status and available derivatives)
    pub async fn get_manifest(&self, urn: &str) -> Result<Manifest> {
        let token = self.auth.get_token().await?;
        let manifest_url = format!(
            "{}/designdata/{}/manifest",
            self.config.derivative_url(),
            urn
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&manifest_url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get manifest ({status}): {error_text}");
        }

        let manifest: Manifest = response
            .json()
            .await
            .context("Failed to parse manifest response")?;

        Ok(manifest)
    }

    /// Delete manifest (and all derivatives)
    #[allow(dead_code)]
    pub async fn delete_manifest(&self, urn: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let manifest_url = format!(
            "{}/designdata/{}/manifest",
            self.config.derivative_url(),
            urn
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&manifest_url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete manifest ({status}): {error_text}");
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

    /// Download all derivatives matching a format
    pub async fn download_derivatives_by_format(
        &self,
        source_urn: &str,
        format: &str,
        output_dir: &Path,
    ) -> Result<Vec<(String, u64)>> {
        let downloadables = self.list_downloadable_derivatives(source_urn).await?;
        let filtered = Self::filter_by_format(&downloadables, format);

        if filtered.is_empty() {
            anyhow::bail!("No derivatives found with format '{format}'");
        }

        let mut results = Vec::new();

        for derivative in filtered {
            let output_path = raps_kernel::security::safe_join(output_dir, &derivative.name)?;
            let size = self
                .download_derivative(source_urn, &derivative.urn, &output_path)
                .await?;
            results.push((derivative.name, size));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_by_format() {
        let derivatives = vec![
            DownloadableDerivative {
                guid: "guid1".to_string(),
                name: "model.obj".to_string(),
                output_type: "obj".to_string(),
                role: "3d".to_string(),
                urn: "urn1".to_string(),
                mime: None,
                size: Some(1024),
            },
            DownloadableDerivative {
                guid: "guid2".to_string(),
                name: "model.stl".to_string(),
                output_type: "stl".to_string(),
                role: "3d".to_string(),
                urn: "urn2".to_string(),
                mime: None,
                size: None,
            },
        ];

        let filtered = DerivativeClient::filter_by_format(&derivatives, "obj");
        assert_eq!(filtered.len(), 1);

        let filtered = DerivativeClient::filter_by_format(&derivatives, "OBJ");
        assert_eq!(filtered.len(), 1);

        let filtered = DerivativeClient::filter_by_format(&derivatives, "ifc");
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_by_guid() {
        let derivatives = vec![DownloadableDerivative {
            guid: "guid1".to_string(),
            name: "model.obj".to_string(),
            output_type: "obj".to_string(),
            role: "3d".to_string(),
            urn: "urn1".to_string(),
            mime: None,
            size: None,
        }];

        let found = DerivativeClient::filter_by_guid(&derivatives, "guid1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "model.obj");

        let not_found = DerivativeClient::filter_by_guid(&derivatives, "nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_filter_by_format_empty_list() {
        let derivatives: Vec<DownloadableDerivative> = vec![];
        let filtered = DerivativeClient::filter_by_format(&derivatives, "obj");
        assert!(filtered.is_empty());
    }
}

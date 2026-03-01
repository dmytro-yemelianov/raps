// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! AppBundle operations for the Design Automation API.

use anyhow::{Context, Result};
use serde::Serialize;

use raps_kernel::http;

use crate::DesignAutomationClient;
use crate::types::*;

impl DesignAutomationClient {
    /// List all app bundles
    pub async fn list_appbundles(&self) -> Result<Vec<String>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles", self.config.da_url());

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list appbundles ({status}): {error_text}");
        }

        let paginated: PaginatedResponse<String> = response
            .json()
            .await
            .context("Failed to parse appbundles response")?;

        Ok(paginated.data)
    }

    /// Create a new app bundle
    pub async fn create_appbundle(
        &self,
        id: &str,
        engine: &str,
        description: Option<&str>,
    ) -> Result<AppBundleDetails> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles", self.config.da_url());

        let request = CreateAppBundleRequest {
            id: id.to_string(),
            engine: engine.to_string(),
            description: description.map(|s| s.to_string()),
        };

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&request)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create appbundle ({status}): {error_text}");
        }

        let appbundle: AppBundleDetails = response
            .json()
            .await
            .context("Failed to parse appbundle response")?;

        Ok(appbundle)
    }

    /// Create an alias for an app bundle
    pub async fn create_appbundle_alias(
        &self,
        bundle_id: &str,
        alias: &str,
        version: i32,
    ) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles/{}/aliases", self.config.da_url(), bundle_id);

        #[derive(Serialize)]
        struct AliasRequest {
            id: String,
            version: i32,
        }

        let request = AliasRequest {
            id: alias.to_string(),
            version,
        };

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&request)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create appbundle alias ({status}): {error_text}");
        }

        Ok(())
    }

    /// Delete an app bundle
    pub async fn delete_appbundle(&self, id: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/appbundles/{}", self.config.da_url(), id);

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete appbundle ({status}): {error_text}");
        }

        Ok(())
    }

    /// Upload an app bundle archive (.zip) using pre-signed S3 URL
    ///
    /// After creating an app bundle, the response includes `upload_parameters`
    /// with an `endpoint_url` and `form_data` fields. This method POSTs the
    /// archive file as multipart/form-data to that pre-signed URL.
    ///
    /// # Arguments
    /// * `upload_params` - The upload parameters from the create_appbundle response
    /// * `file_path` - Path to the .zip archive to upload
    pub async fn upload_appbundle(
        &self,
        upload_params: &UploadParameters,
        file_path: &std::path::Path,
    ) -> Result<()> {
        let endpoint_url = upload_params
            .endpoint_url
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Upload parameters missing endpoint URL"))?;

        // Validate file exists and is a zip
        if !file_path.exists() {
            anyhow::bail!("File not found: {}", file_path.display());
        }

        let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if extension != "zip" {
            anyhow::bail!(
                "Expected .zip archive, got .{} ({})",
                extension,
                file_path.display()
            );
        }

        // Read the file
        let file_bytes = tokio::fs::read(file_path)
            .await
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("bundle.zip")
            .to_string();

        // Build multipart form with form_data fields + the file
        let mut form = reqwest::multipart::Form::new();

        // Add all form_data fields first (required by S3 pre-signed POST)
        if let Some(ref form_data) = upload_params.form_data {
            for (key, value) in form_data {
                form = form.text(key.clone(), value.clone());
            }
        }

        // Add the file as the last field (S3 requires "file" to be last)
        let file_part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name(file_name)
            .mime_str("application/octet-stream")?;
        form = form.part("file", file_part);

        // POST to the pre-signed URL (no auth header needed -- S3 pre-signed)
        let response = self
            .http_client
            .post(endpoint_url)
            .multipart(form)
            .send()
            .await
            .context("Failed to upload app bundle archive")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to upload app bundle ({status}): {error_text}");
        }

        Ok(())
    }
}

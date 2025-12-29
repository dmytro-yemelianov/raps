// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Model Derivative API module
//!
//! This module is now an adapter that wraps raps-derivative service crate
//! to maintain backward compatibility with existing commands.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use super::AuthClient;
use crate::config::Config;

// Re-export types from raps-derivative for backward compatibility
pub use raps_derivative::types::*;

/// Supported output formats for translation (re-exported for compatibility)
pub use raps_derivative::types::OutputFormat;

/// Model Derivative API client (adapter wrapping raps-derivative service crate with cached clients)
#[derive(Clone)]
pub struct DerivativeClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
    // Cached kernel clients (lazy-initialized)
    kernel_config: Arc<Mutex<Option<raps_kernel::Config>>>,
    kernel_http: Arc<Mutex<Option<raps_kernel::HttpClient>>>,
    kernel_auth: Arc<Mutex<Option<raps_kernel::AuthClient>>>,
}

impl DerivativeClient {
    /// Create a new Derivative client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, crate::http::HttpClientConfig::default())
    }

    /// Create a new Derivative client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: crate::http::HttpClientConfig,
    ) -> Self {
        let http_client = http_config
            .create_client()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            auth,
            http_client,
            kernel_config: Arc::new(Mutex::new(None)),
            kernel_http: Arc::new(Mutex::new(None)),
            kernel_auth: Arc::new(Mutex::new(None)),
        }
    }

    /// Get or create kernel config (cached)
    async fn get_kernel_config(&self) -> Result<raps_kernel::Config> {
        let mut config = self.kernel_config.lock().await;
        if config.is_none() {
            *config = Some(raps_kernel::Config {
                client_id: self.config.client_id.clone(),
                client_secret: self.config.client_secret.clone(),
                base_url: self.config.base_url.clone(),
                callback_url: self.config.callback_url.clone(),
                da_nickname: self.config.da_nickname.clone(),
            });
        }
        Ok(config.as_ref().unwrap().clone())
    }

    /// Get or create kernel HTTP client (cached)
    async fn get_kernel_http(&self) -> Result<raps_kernel::HttpClient> {
        let mut http = self.kernel_http.lock().await;
        if http.is_none() {
            let config = raps_kernel::HttpClientConfig {
                timeout: std::time::Duration::from_secs(120),
                connect_timeout: std::time::Duration::from_secs(30),
                max_retries: 3,
                retry_base_delay: std::time::Duration::from_secs(1),
                retry_max_delay: std::time::Duration::from_secs(60),
                retry_jitter: true,
            };
            *http = Some(raps_kernel::HttpClient::new(config)
                .map_err(|e| anyhow::anyhow!("Failed to create kernel HTTP client: {}", e))?);
        }
        Ok(http.as_ref().unwrap().clone())
    }

    /// Get or create kernel auth client (cached)
    async fn get_kernel_auth(&self) -> Result<raps_kernel::AuthClient> {
        let mut auth = self.kernel_auth.lock().await;
        if auth.is_none() {
            let kernel_config = self.get_kernel_config().await?;
            *auth = Some(raps_kernel::AuthClient::new(kernel_config)
                .map_err(|e| anyhow::anyhow!("Failed to create kernel auth client: {}", e))?);
        }
        Ok(auth.as_ref().unwrap().clone())
    }

    /// Start a translation job
    pub async fn translate(
        &self,
        urn: &str,
        output_format: OutputFormat,
        root_filename: Option<&str>,
    ) -> Result<TranslationResponse> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let derivative_url = kernel_config.derivative_url();

        let translate_client = raps_derivative::TranslateClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            derivative_url,
        );

        translate_client.translate(urn, output_format, root_filename).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get translation manifest
    pub async fn get_manifest(&self, urn: &str) -> Result<Manifest> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let derivative_url = kernel_config.derivative_url();

        let manifest_client = raps_derivative::ManifestClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            derivative_url,
        );

        manifest_client.get_manifest(urn).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Delete a manifest
    pub async fn delete_manifest(&self, urn: &str) -> Result<()> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let derivative_url = kernel_config.derivative_url();

        let manifest_client = raps_derivative::ManifestClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            derivative_url,
        );

        manifest_client.delete_manifest(urn).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get translation status
    pub async fn get_status(&self, urn: &str) -> Result<(String, String)> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let derivative_url = kernel_config.derivative_url();

        let manifest_client = raps_derivative::ManifestClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            derivative_url,
        );

        manifest_client.get_status(urn).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// List downloadable derivatives
    pub async fn list_downloadable_derivatives(
        &self,
        urn: &str,
    ) -> Result<Vec<DownloadableDerivative>> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let derivative_url = kernel_config.derivative_url();

        let manifest_client = raps_derivative::ManifestClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            derivative_url,
        );

        manifest_client.list_downloadable_derivatives(urn).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Download a derivative by GUID
    pub async fn download_derivative(
        &self,
        urn: &str,
        guid: &str,
        output_path: &Path,
    ) -> Result<u64> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let derivative_url = kernel_config.derivative_url();

        let download_client = raps_derivative::DownloadClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            derivative_url,
        );

        download_client.download_derivative(urn, guid, output_path).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Download derivatives by format
    pub async fn download_derivatives_by_format(
        &self,
        urn: &str,
        format: &str,
        output_dir: &Path,
    ) -> Result<Vec<(String, u64)>> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let derivative_url = kernel_config.derivative_url();

        // Get downloadables first
        let manifest_client = raps_derivative::ManifestClient::new(
            kernel_http.clone(),
            kernel_auth.clone(),
            kernel_config.clone(),
            derivative_url.clone(),
        );
        let downloadables = manifest_client.list_downloadable_derivatives(urn).await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let download_client = raps_derivative::DownloadClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            derivative_url,
        );

        download_client.download_derivatives_by_format(urn, format, output_dir, &downloadables).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Filter derivatives by format (helper method for commands)
    pub fn filter_by_format(
        downloadables: &[DownloadableDerivative],
        format: &str,
    ) -> Vec<DownloadableDerivative> {
        raps_derivative::ManifestClient::filter_by_format(downloadables, format)
    }

    /// Filter derivatives by GUID (helper method for commands)
    pub fn filter_by_guid(
        downloadables: &[DownloadableDerivative],
        guid: &str,
    ) -> Option<DownloadableDerivative> {
        raps_derivative::ManifestClient::filter_by_guid(downloadables, guid)
    }
}

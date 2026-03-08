// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! HTTP client for the RAPS marketplace API.

use anyhow::{Context, Result};
use reqwest::Client;
use raps_kernel::marketplace::{Plugin, ValidateResponse, VersionInfo};

const API_BASE: &str = "https://api.rapscli.xyz";
const USER_AGENT: &str = concat!("raps-cli/", env!("CARGO_PKG_VERSION"));

/// HTTP client for the marketplace API.
pub struct MarketplaceClient {
    client: Client,
    api_base: String,
}

impl MarketplaceClient {
    /// Create a new client targeting the production marketplace API.
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self {
            client,
            api_base: API_BASE.to_string(),
        })
    }

    /// Create a client targeting a custom API base URL (for testing).
    pub fn with_base_url(base_url: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Self {
            client,
            api_base: base_url.into(),
        })
    }

    /// Fetch a single plugin by slug from the public catalog.
    pub async fn get_plugin(&self, slug: &str) -> Result<Plugin> {
        let url = format!("{}/plugins/{}", self.api_base, slug);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;
        if !resp.status().is_success() {
            anyhow::bail!("Plugin '{}' not found (HTTP {})", slug, resp.status());
        }
        resp.json::<Plugin>().await.context("Failed to parse plugin response")
    }

    /// List all published plugins from the public catalog.
    pub async fn list_plugins(&self) -> Result<Vec<Plugin>> {
        let url = format!("{}/plugins", self.api_base);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send request")?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to list plugins (HTTP {})", resp.status());
        }
        resp.json::<Vec<Plugin>>().await.context("Failed to parse plugins response")
    }

    /// Get version history for a plugin.
    /// Returns an empty vec for now (endpoint not yet implemented on the server).
    pub async fn get_versions(&self, _slug: &str) -> Result<Vec<VersionInfo>> {
        Ok(vec![])
    }

    /// Validate a license key and return the entitlements.
    pub async fn validate_license(&self, license_key: &str) -> Result<ValidateResponse> {
        let url = format!("{}/license/validate", self.api_base);
        let resp = self
            .client
            .post(&url)
            .bearer_auth(license_key)
            .send()
            .await
            .context("Failed to send license validation request")?;
        if resp.status() == 401 {
            anyhow::bail!("Invalid or expired license key");
        }
        if !resp.status().is_success() {
            anyhow::bail!("License validation failed (HTTP {})", resp.status());
        }
        resp.json::<ValidateResponse>().await.context("Failed to parse validation response")
    }

    /// Download a plugin binary for the given platform.
    ///
    /// Returns `(bytes, sha256_hex, signature_hex, version)` from response headers and body.
    pub async fn download_plugin(
        &self,
        slug: &str,
        platform: &str,
        license_key: &str,
    ) -> Result<(Vec<u8>, String, String, String)> {
        let url = format!(
            "{}/license/plugins/{}/download?platform={}",
            self.api_base, slug, platform
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(license_key)
            .send()
            .await
            .context("Failed to send download request")?;

        if resp.status() == 401 {
            anyhow::bail!("License key not authorized for plugin '{}'", slug);
        }
        if resp.status() == 403 {
            anyhow::bail!("Your subscription does not include plugin '{}'", slug);
        }
        if !resp.status().is_success() {
            anyhow::bail!("Download failed (HTTP {})", resp.status());
        }

        let sha256 = resp
            .headers()
            .get("x-sha256")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let signature = resp
            .headers()
            .get("x-ed25519-signature")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let version = resp
            .headers()
            .get("x-plugin-version")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let bytes = resp.bytes().await.context("Failed to read response body")?.to_vec();
        Ok((bytes, sha256, signature, version))
    }
}

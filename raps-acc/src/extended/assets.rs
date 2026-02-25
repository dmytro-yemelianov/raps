// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Asset operations for the ACC Extended API.

use anyhow::{Context, Result};

use raps_kernel::http;

use super::types::*;
use super::AccClient;

impl AccClient {
    /// List assets in a project
    pub async fn list_assets(&self, project_id: &str) -> Result<Vec<Asset>> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/assets",
            self.config.assets_url(),
            project_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list assets ({status}): {error_text}");
        }

        let assets_response: AssetsResponse = response
            .json()
            .await
            .context("Failed to parse assets response")?;

        Ok(assets_response.results)
    }

    /// Get a specific asset by ID
    pub async fn get_asset(&self, project_id: &str, asset_id: &str) -> Result<Asset> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/assets/{}",
            self.config.assets_url(),
            project_id,
            asset_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get asset ({status}): {error_text}");
        }

        let asset: Asset = response
            .json()
            .await
            .context("Failed to parse asset response")?;
        Ok(asset)
    }

    /// Create a new asset
    pub async fn create_asset(
        &self,
        project_id: &str,
        request: CreateAssetRequest,
    ) -> Result<Asset> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/assets",
            self.config.assets_url(),
            project_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .json(&request)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create asset ({status}): {error_text}");
        }

        let asset: Asset = response
            .json()
            .await
            .context("Failed to parse asset response")?;
        Ok(asset)
    }

    /// Update an existing asset
    pub async fn update_asset(
        &self,
        project_id: &str,
        asset_id: &str,
        request: UpdateAssetRequest,
    ) -> Result<Asset> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/assets/{}",
            self.config.assets_url(),
            project_id,
            asset_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .patch(&url)
                .bearer_auth(&token)
                .json(&request)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to update asset ({status}): {error_text}");
        }

        let asset: Asset = response
            .json()
            .await
            .context("Failed to parse asset response")?;
        Ok(asset)
    }

    /// Delete an asset
    pub async fn delete_asset(&self, project_id: &str, asset_id: &str) -> Result<()> {
        let token = self.auth.get_3leg_token().await?;
        let url = format!(
            "{}/projects/{}/assets/{}",
            self.config.assets_url(),
            project_id,
            asset_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete asset ({status}): {error_text}");
        }

        Ok(())
    }
}

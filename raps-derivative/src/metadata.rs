// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Metadata operations for the Model Derivative API.

use anyhow::{Context, Result};
use raps_kernel::error::RapsError;

use crate::DerivativeClient;
use crate::types::*;

impl DerivativeClient {
    /// Get metadata (list of model views/viewables) for a translated model
    pub async fn get_metadata(
        &self,
        urn: &str,
        region: Option<MdRegion>,
    ) -> Result<MetadataResponse> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/designdata/{}/metadata",
            self.config.derivative_url(),
            urn
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            let mut req = self.http_client.get(&url).bearer_auth(&token);
            if let Some(region) = region {
                req = req.header("x-ads-region", region.to_string());
            }
            req
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        response
            .json()
            .await
            .context("Failed to parse metadata response")
    }

    /// Get object tree hierarchy for a specific model view
    pub async fn get_object_tree(
        &self,
        urn: &str,
        model_guid: &str,
        region: Option<MdRegion>,
    ) -> Result<ObjectTreeResponse> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/designdata/{}/metadata/{}",
            self.config.derivative_url(),
            urn,
            model_guid
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            let mut req = self.http_client.get(&url).bearer_auth(&token);
            if let Some(region) = region {
                req = req.header("x-ads-region", region.to_string());
            }
            req
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        response
            .json()
            .await
            .context("Failed to parse object tree response")
    }

    /// Get all properties for a specific model view
    pub async fn get_properties(
        &self,
        urn: &str,
        model_guid: &str,
        object_id: Option<i64>,
        region: Option<MdRegion>,
    ) -> Result<PropertiesResponse> {
        let token = self.auth.get_token().await?;
        let mut url = format!(
            "{}/designdata/{}/metadata/{}/properties",
            self.config.derivative_url(),
            urn,
            model_guid
        );

        if let Some(id) = object_id {
            url = format!("{}?objectid={}", url, id);
        }

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            let mut req = self.http_client.get(&url).bearer_auth(&token);
            if let Some(region) = region {
                req = req.header("x-ads-region", region.to_string());
            }
            req
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        response
            .json()
            .await
            .context("Failed to parse properties response")
    }

    /// Query specific properties by object IDs (POST endpoint)
    pub async fn query_properties(
        &self,
        urn: &str,
        model_guid: &str,
        query: PropertyQuery,
        region: Option<MdRegion>,
    ) -> Result<PropertiesResponse> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/designdata/{}/metadata/{}/properties:query",
            self.config.derivative_url(),
            urn,
            model_guid
        );

        let query_json =
            serde_json::to_value(&query).context("Failed to serialize property query")?;

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            let mut req = self
                .http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&query_json);
            if let Some(region) = region {
                req = req.header("x-ads-region", region.to_string());
            }
            req
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        response
            .json()
            .await
            .context("Failed to parse query properties response")
    }
}

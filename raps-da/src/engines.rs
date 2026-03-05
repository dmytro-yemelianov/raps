// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Engine operations for the Design Automation API.

use anyhow::{Context, Result};
use raps_kernel::error::RapsError;

use raps_kernel::http;

use crate::DesignAutomationClient;
use crate::types::*;

impl DesignAutomationClient {
    /// Get the configured nickname (or "default")
    pub fn nickname(&self) -> &str {
        self.config.da_nickname.as_deref().unwrap_or("default")
    }

    /// Fetch the effective nickname from the DA API.
    ///
    /// Returns the configured nickname if set, otherwise calls
    /// `GET /forgeapps/me` to get the actual owner name (usually the client_id).
    pub async fn effective_nickname(&self) -> Result<String> {
        if let Some(ref nick) = self.config.da_nickname {
            return Ok(nick.clone());
        }
        let token = self.auth.get_token().await?;
        let url = format!("{}/forgeapps/me", self.config.da_url());
        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            // Response is a plain string (the nickname) wrapped in quotes
            let trimmed = text.trim().trim_matches('"');
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        } else {
            return Err(RapsError::from_response(response).await.into());
        }
        Ok("default".to_string())
    }

    /// List available engines
    ///
    /// Returns a list of engine IDs (e.g., "Autodesk.Revit+2024").
    /// Use `get_engine` to fetch full details for a specific engine.
    pub async fn list_engines(&self) -> Result<Vec<String>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/engines", self.config.da_url());

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let paginated: PaginatedResponse<String> = response
            .json()
            .await
            .context("Failed to parse engines response")?;

        Ok(paginated.data)
    }

    /// List all engines with pagination, returning structured Engine objects.
    ///
    /// The API returns engine IDs as strings. This method parses the ID to
    /// extract product name and version as the description.
    pub async fn list_engines_detailed(&self) -> Result<Vec<Engine>> {
        let token = self.auth.get_token().await?;
        let base_url = format!("{}/engines", self.config.da_url());
        let mut all_engines = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let url = match &page_token {
                Some(tok) => format!("{base_url}?page={tok}"),
                None => base_url.clone(),
            };

            let token_clone = token.clone();
            let response = http::send_with_retry(&self.config.http_config, || {
                self.http_client.get(&url).bearer_auth(&token_clone)
            })
            .await?;

            tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

            if !response.status().is_success() {
                return Err(RapsError::from_response(response).await.into());
            }

            let paginated: PaginatedResponse<String> = response
                .json()
                .await
                .context("Failed to parse engines response")?;

            // Convert string IDs to Engine structs, parsing description from the ID.
            // Format: "Autodesk.ProductName+VersionNumber"
            for id in paginated.data {
                let description = id
                    .split('.')
                    .next_back()
                    .map(|s| s.replace('+', " "))
                    .unwrap_or_default();
                all_engines.push(Engine {
                    id,
                    description: Some(description),
                    product_version: None,
                });
            }

            match paginated.pagination_token {
                Some(tok) if !tok.is_empty() => page_token = Some(tok),
                _ => break,
            }
        }

        Ok(all_engines)
    }
}

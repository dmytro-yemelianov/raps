// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! 2-legged OAuth (client credentials) flow

use anyhow::{Context, Result};
use std::time::{Duration, Instant};

use super::AuthClient;
use super::types::{CachedToken, TokenResponse};

impl AuthClient {
    /// Get a valid 2-legged access token
    pub async fn get_token(&self) -> Result<String> {
        // Check if we have a valid cached token
        {
            let cache = self.cached_2leg_token.read().await;
            if let Some(ref token) = *cache
                && token.is_valid()
            {
                return Ok(token.access_token.clone());
            }
        }

        // Fetch new token
        let new_token = self.fetch_2leg_token().await?;

        // Cache the new token
        {
            let mut cache = self.cached_2leg_token.write().await;
            *cache = Some(CachedToken {
                access_token: new_token.access_token.clone(),
                expires_at: Instant::now() + Duration::from_secs(new_token.expires_in),
            });
        }

        Ok(new_token.access_token)
    }

    /// Fetch a new 2-legged token
    async fn fetch_2leg_token(&self) -> Result<TokenResponse> {
        self.config.require_credentials()?;

        let url = self.config.auth_url();

        let params = [
            ("grant_type", "client_credentials"),
            (
                "scope",
                "data:read data:write data:create bucket:read bucket:create bucket:delete code:all",
            ),
        ];

        let _auth_start = std::time::Instant::now();
        let response = self
            .http_client
            .post(&url)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&params)
            .send()
            .await
            .context("Failed to send authentication request")?;
        crate::profiler::record_http_request(_auth_start.elapsed());

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Authentication failed with status {}: {}",
                status,
                crate::logging::redact_secrets(&error_text)
            );
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        Ok(token_response)
    }

    /// Test 2-legged authentication
    pub async fn test_auth(&self) -> Result<()> {
        self.get_token().await?;
        Ok(())
    }

    /// Clear the cached 2-legged token
    #[allow(dead_code)]
    pub async fn clear_cache(&self) {
        let mut cache = self.cached_2leg_token.write().await;
        *cache = None;
    }
}

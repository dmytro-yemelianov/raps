// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Two-legged OAuth 2.0 (Client Credentials) flow for APS
//!
//! This flow is used for server-to-server authentication where no user
//! interaction is required. The client authenticates using its client ID
//! and client secret.

use crate::config::Config;
use crate::error::{RapsError, Result};
use crate::http::HttpClient;
use super::types::{TokenResponse, Scopes};

/// Two-legged OAuth 2.0 authentication
///
/// Implements the client credentials flow for server-to-server authentication.
pub struct TwoLeggedAuth<'a> {
    config: &'a Config,
    http: &'a HttpClient,
}

impl<'a> TwoLeggedAuth<'a> {
    /// Create a new two-legged auth handler
    pub fn new(config: &'a Config, http: &'a HttpClient) -> Self {
        Self { config, http }
    }

    /// Fetch a new access token using client credentials
    pub async fn get_token(&self) -> Result<TokenResponse> {
        self.get_token_with_scopes(&Scopes::two_legged_default()).await
    }

    /// Fetch a new access token with custom scopes
    pub async fn get_token_with_scopes(&self, scopes: &[&str]) -> Result<TokenResponse> {
        let url = self.config.auth_url();
        let scope_string = Scopes::join(scopes);

        let params = [
            ("grant_type", "client_credentials"),
            ("scope", &scope_string),
        ];

        let response = self
            .http
            .inner()
            .post(&url)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&params)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to send authentication request".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Authentication failed with status {}: {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse token response: {}", e),
            })?;

        Ok(token_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_string_generation() {
        let scopes = Scopes::two_legged_default();
        let scope_string = Scopes::join(&scopes);
        
        // Should contain all default scopes
        assert!(scope_string.contains("data:read"));
        assert!(scope_string.contains("data:write"));
        assert!(scope_string.contains("bucket:create"));
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Three-legged OAuth 2.0 (Authorization Code) flow for APS
//!
//! This flow is used when user authorization is required. It involves:
//! 1. Redirecting the user to Autodesk login
//! 2. User grants permission
//! 3. Autodesk redirects back with an authorization code
//! 4. Exchange the code for access/refresh tokens
//!
//! Note: The actual browser-based login UI is implemented in the CLI layer.
//! This module provides the core token exchange and refresh logic.

use crate::config::Config;
use crate::error::{ExitCode, RapsError, Result};
use crate::http::HttpClient;
use super::types::{TokenResponse, StoredToken, Scopes};

/// Three-legged OAuth 2.0 authentication
///
/// Implements the authorization code flow for user authentication.
pub struct ThreeLeggedAuth<'a> {
    config: &'a Config,
    http: &'a HttpClient,
}

impl<'a> ThreeLeggedAuth<'a> {
    /// Create a new three-legged auth handler
    pub fn new(config: &'a Config, http: &'a HttpClient) -> Self {
        Self { config, http }
    }

    /// Get the authorization URL for user login
    ///
    /// Returns the URL to redirect the user to for authentication.
    pub fn authorization_url(&self, state: Option<&str>) -> String {
        let callback = &self.config.callback_url;
        
        let scopes = Scopes::join(&Scopes::three_legged_default());
        
        let mut url = format!(
            "https://developer.api.autodesk.com/authentication/v2/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri={}&\
            scope={}",
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(callback),
            urlencoding::encode(&scopes),
        );

        if let Some(state) = state {
            url.push_str(&format!("&state={}", urlencoding::encode(state)));
        }

        url
    }

    /// Exchange an authorization code for tokens
    pub async fn exchange_code(&self, code: &str) -> Result<TokenResponse> {
        let url = self.config.auth_url();

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.config.callback_url.as_str()),
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
                message: "Failed to exchange authorization code".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Auth {
                message: format!("Code exchange failed ({}): {}", status, error_text),
                code: ExitCode::AuthFailure,
                source: None,
            });
        }

        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse token response: {}", e),
            })?;

        Ok(token)
    }

    /// Refresh an expired access token using a refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse> {
        let url = self.config.auth_url();

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
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
                message: "Failed to refresh token".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Auth {
                message: format!("Token refresh failed ({}): {}", status, error_text),
                code: ExitCode::AuthFailure,
                source: None,
            });
        }

        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse refresh response: {}", e),
            })?;

        Ok(token)
    }

    /// Convert a token response to a stored token
    pub fn to_stored_token(response: &TokenResponse, original_refresh: Option<&str>) -> StoredToken {
        let mut stored = StoredToken::from_response(response);
        
        // If no new refresh token was provided, keep the original
        if stored.refresh_token.is_none() {
            stored.refresh_token = original_refresh.map(String::from);
        }
        
        stored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_url_format() {
        let config = Config {
            client_id: "test_client".to_string(),
            client_secret: "test_secret".to_string(),
            callback_url: "http://localhost:3000/callback".to_string(),
            ..Default::default()
        };
        let http_config = crate::http::HttpClientConfig::default();
        let http = crate::http::HttpClient::new(http_config).expect("http client");

        let auth = ThreeLeggedAuth::new(&config, &http);
        let url = auth.authorization_url(Some("test_state"));

        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=test_client"));
        assert!(url.contains("state=test_state"));
        assert!(url.contains("redirect_uri="));
    }

    #[test]
    fn test_to_stored_token_preserves_refresh() {
        let response = TokenResponse {
            access_token: "new_access".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: None, // No new refresh token
            scope: Some("data:read".to_string()),
        };

        let stored = ThreeLeggedAuth::to_stored_token(&response, Some("original_refresh"));
        assert_eq!(stored.refresh_token, Some("original_refresh".to_string()));
    }

    #[test]
    fn test_to_stored_token_uses_new_refresh() {
        let response = TokenResponse {
            access_token: "new_access".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some("new_refresh".to_string()),
            scope: Some("data:read".to_string()),
        };

        let stored = ThreeLeggedAuth::to_stored_token(&response, Some("original_refresh"));
        assert_eq!(stored.refresh_token, Some("new_refresh".to_string()));
    }
}

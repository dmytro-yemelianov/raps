// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Authentication client for APS OAuth 2.0

use crate::config::Config;
use crate::error::{RapsError, Result};
use crate::http::{HttpClient, HttpClientConfig};
use crate::storage::{StorageBackend, TokenStorage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// User profile information from /userinfo endpoint
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct UserInfo {
    /// The unique APS ID of the user
    pub sub: String,
    /// Full name
    pub name: Option<String>,
    /// First name
    pub given_name: Option<String>,
    /// Last name
    pub family_name: Option<String>,
    /// Preferred username
    pub preferred_username: Option<String>,
    /// Email address
    pub email: Option<String>,
    /// Whether email is verified
    pub email_verified: Option<bool>,
    /// Profile URL
    pub profile: Option<String>,
    /// Profile picture URL
    pub picture: Option<String>,
}

/// OAuth 2.0 token response from APS
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenResponse {
    /// Access token string
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Expiration time in seconds
    pub expires_in: u64,
    /// Refresh token (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// OAuth scopes granted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Device code response from APS Device Authorization endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    /// Device code for polling
    pub device_code: String,
    /// User code to display
    pub user_code: String,
    /// Verification URI
    pub verification_uri: String,
    /// Complete verification URI with user code
    pub verification_uri_complete: Option<String>,
    /// Expiration time in seconds
    pub expires_in: u64,
    /// Polling interval in seconds
    pub interval: Option<u64>,
}

/// Stored token with metadata for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    /// Access token string
    pub access_token: String,
    /// Refresh token (if available)
    pub refresh_token: Option<String>,
    /// Expiration timestamp (Unix epoch seconds)
    pub expires_at: i64,
    /// OAuth scopes granted
    pub scopes: Vec<String>,
}

impl StoredToken {
    /// Check if token is still valid (not expired)
    pub fn is_valid(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        // Consider expired 60 seconds before actual expiry
        self.expires_at > now + 60
    }
}

/// Cached token with expiry tracking (for 2-legged)
#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

impl CachedToken {
    fn is_valid(&self) -> bool {
        self.expires_at > Instant::now() + Duration::from_secs(60)
    }
}

/// Authentication client for APS
///
/// Handles OAuth 2.0 token acquisition for both 2-legged and 3-legged flows.
#[derive(Clone)]
pub struct AuthClient {
    config: Config,
    http: HttpClient,
    cached_2leg_token: Arc<RwLock<Option<CachedToken>>>,
    cached_3leg_token: Arc<RwLock<Option<StoredToken>>>,
}

impl AuthClient {
    /// Create a new authentication client
    pub fn new(config: Config) -> Result<Self> {
        Self::new_with_http_config(config, HttpClientConfig::default())
    }

    /// Create a new authentication client with custom HTTP config
    pub fn new_with_http_config(config: Config, http_config: HttpClientConfig) -> Result<Self> {
        // Try to load stored 3-legged token synchronously
        let stored_token = Self::load_stored_token_static(&config);

        // Create HTTP client
        let http = HttpClient::new(http_config)?;

        Ok(Self {
            config,
            http,
            cached_2leg_token: Arc::new(RwLock::new(None)),
            cached_3leg_token: Arc::new(RwLock::new(stored_token)),
        })
    }

    /// Get token storage instance
    fn token_storage(&self) -> TokenStorage {
        let backend = StorageBackend::from_env();
        TokenStorage::new(backend)
    }

    /// Load token from persistent storage (static version for initialization)
    fn load_stored_token_static(_config: &Config) -> Option<StoredToken> {
        let backend = StorageBackend::from_env();
        let storage = TokenStorage::new(backend);
        storage.load().ok().flatten()
    }

    /// Save token to persistent storage
    fn save_token(&self, token: &StoredToken) -> Result<()> {
        let storage = self.token_storage();
        storage.save(token).map_err(|e| RapsError::Storage {
            message: format!("Failed to save token: {}", e),
            source: Some(anyhow::anyhow!("{}", e)),
        })
    }

    /// Delete stored token
    pub fn delete_stored_token(&self) -> Result<()> {
        let storage = self.token_storage();
        storage.delete().map_err(|e| RapsError::Storage {
            message: format!("Failed to delete token: {}", e),
            source: Some(anyhow::anyhow!("{}", e)),
        })
    }

    /// Get a valid 2-legged access token
    pub async fn get_token(&self) -> Result<String> {
        // Check if we have a valid cached token
        {
            let cache = self.cached_2leg_token.read().await;
            if let Some(ref token) = *cache {
                if token.is_valid() {
                    return Ok(token.access_token.clone());
                }
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

    /// Get a valid 3-legged access token (requires prior login)
    pub async fn get_3leg_token(&self) -> Result<String> {
        // Check cached token
        let refresh_token_to_use: Option<String>;
        {
            let cache = self.cached_3leg_token.read().await;
            if let Some(ref token) = *cache {
                if token.is_valid() {
                    return Ok(token.access_token.clone());
                }
                // Get refresh token for later use
                refresh_token_to_use = token.refresh_token.clone();
            } else {
                refresh_token_to_use = None;
            }
        }

        // Try to refresh if we have a refresh token
        if let Some(refresh) = refresh_token_to_use {
            return self.refresh_token(refresh).await;
        }

        Err(RapsError::Auth {
            message: "Not logged in. Please run 'raps auth login' first.".to_string(),
            code: crate::error::ExitCode::AuthFailure,
            source: None,
        })
    }

    /// Check if user is logged in with 3-legged OAuth
    pub async fn is_logged_in(&self) -> bool {
        let cache = self.cached_3leg_token.read().await;
        if let Some(ref token) = *cache {
            if token.is_valid() {
                return true;
            }
            // Check if we can refresh
            if token.refresh_token.is_some() {
                return true;
            }
        }
        false
    }

    /// Fetch a new 2-legged token
    async fn fetch_2leg_token(&self) -> Result<TokenResponse> {
        let url = self.config.auth_url();

        let params = [
            ("grant_type", "client_credentials"),
            (
                "scope",
                "data:read data:write data:create bucket:read bucket:create bucket:delete code:all",
            ),
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

    /// Get config reference
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get user profile information (requires 3-legged auth with user:read or user-profile:read scope)
    pub async fn get_user_info(&self) -> Result<UserInfo> {
        let token = self.get_3leg_token().await?;
        self.get_user_info_with_token(&token).await
    }

    /// Get user info with a provided token (for validation)
    async fn get_user_info_with_token(&self, token: &str) -> Result<UserInfo> {
        let url = "https://api.userprofile.autodesk.com/userinfo";
        let response = self
            .http
            .inner()
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to fetch user info".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to validate token ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let user: UserInfo = response
            .json()
            .await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse user info: {}", e),
            })?;

        Ok(user)
    }

    /// Get token expiry timestamp
    pub async fn get_token_expiry(&self) -> Option<i64> {
        let cache = self.cached_3leg_token.read().await;
        cache.as_ref().map(|t| t.expires_at)
    }

    /// Refresh an expired access token
    async fn refresh_token(&self, refresh_token: String) -> Result<String> {
        let url = self.config.auth_url();

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
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
            // Refresh failed, clear stored token
            self.delete_stored_token().ok();
            let mut cache = self.cached_3leg_token.write().await;
            *cache = None;
            return Err(RapsError::Auth {
                message: "Token refresh failed. Please login again with 'raps auth login'".to_string(),
                code: crate::error::ExitCode::AuthFailure,
                source: None,
            });
        }

        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse refresh response: {}", e),
            })?;

        // Update stored token
        let stored = StoredToken {
            access_token: token.access_token.clone(),
            refresh_token: token.refresh_token.or(Some(refresh_token)),
            expires_at: chrono::Utc::now().timestamp() + token.expires_in as i64,
            scopes: vec![], // Preserve from original
        };

        self.save_token(&stored)?;

        {
            let mut cache = self.cached_3leg_token.write().await;
            *cache = Some(stored);
        }

        Ok(token.access_token)
    }

    // TODO: Implement login, login_device, login_with_token, logout
    // These require additional dependencies and UI interactions that may be better
    // placed in the CLI layer. For now, we provide the core token management.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cached_token_validity() {
        let token = CachedToken {
            access_token: "test".to_string(),
            expires_at: Instant::now() + Duration::from_secs(3600),
        };
        assert!(token.is_valid());

        let expired_token = CachedToken {
            access_token: "test".to_string(),
            expires_at: Instant::now() - Duration::from_secs(1),
        };
        assert!(!expired_token.is_valid());
    }

    #[test]
    fn test_cached_token_near_expiry() {
        // Token expiring in less than 60 seconds should be invalid
        let token = CachedToken {
            access_token: "test".to_string(),
            expires_at: Instant::now() + Duration::from_secs(30),
        };
        assert!(!token.is_valid());

        // Token expiring in more than 60 seconds should be valid
        let token = CachedToken {
            access_token: "test".to_string(),
            expires_at: Instant::now() + Duration::from_secs(120),
        };
        assert!(token.is_valid());
    }

    #[test]
    fn test_stored_token_validity() {
        let now = chrono::Utc::now().timestamp();

        // Valid token (expires in 1 hour)
        let token = StoredToken {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: now + 3600,
            scopes: vec!["data:read".to_string()],
        };
        assert!(token.is_valid());

        // Expired token
        let expired_token = StoredToken {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: now - 100,
            scopes: vec!["data:read".to_string()],
        };
        assert!(!expired_token.is_valid());

        // Token expiring soon (within 60 seconds) should be invalid
        let soon_expiring = StoredToken {
            access_token: "test".to_string(),
            refresh_token: Some("refresh".to_string()),
            expires_at: now + 30,
            scopes: vec!["data:read".to_string()],
        };
        assert!(!soon_expiring.is_valid());
    }

    #[test]
    fn test_stored_token_without_refresh() {
        let now = chrono::Utc::now().timestamp();
        let token = StoredToken {
            access_token: "test".to_string(),
            refresh_token: None,
            expires_at: now + 3600,
            scopes: vec!["data:read".to_string()],
        };
        // Should still be valid if not expired
        assert!(token.is_valid());
    }

    #[test]
    fn test_token_response_serialization() {
        let token = TokenResponse {
            access_token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some("refresh_token".to_string()),
            scope: None,
        };

        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("test_token"));
        assert!(json.contains("Bearer"));
        assert!(json.contains("refresh_token"));

        let deserialized: TokenResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.access_token, "test_token");
        assert_eq!(deserialized.token_type, "Bearer");
        assert_eq!(deserialized.expires_in, 3600);
        assert_eq!(
            deserialized.refresh_token,
            Some("refresh_token".to_string())
        );
    }

    #[test]
    fn test_token_response_without_refresh() {
        let token = TokenResponse {
            access_token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: None,
            scope: None,
        };

        let json = serde_json::to_string(&token).unwrap();
        // refresh_token should be omitted when None
        assert!(!json.contains("refresh_token"));

        let deserialized: TokenResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.refresh_token, None);
    }
}

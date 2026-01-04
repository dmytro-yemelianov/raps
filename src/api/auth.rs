// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Authentication module for APS OAuth 2.0
//!
//! Implements both 2-legged (client credentials) and 3-legged (authorization code) OAuth flows.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tiny_http::{Response, Server};
use tokio::sync::RwLock;

use crate::config::Config;
use crate::logging;
use crate::storage::{StorageBackend, TokenStorage};

/// User profile information from /userinfo endpoint
#[derive(Debug, Clone, Deserialize)]
// API response structs may contain fields we don't use - this is expected for external API contracts
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
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Device code response from APS Device Authorization endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: Option<u64>, // Polling interval in seconds
}

/// Stored token with metadata for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: i64, // Unix timestamp
    pub scopes: Vec<String>,
}

impl StoredToken {
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
    http_client: reqwest::Client,
    cached_2leg_token: Arc<RwLock<Option<CachedToken>>>,
    cached_3leg_token: Arc<RwLock<Option<StoredToken>>>,
}

impl AuthClient {
    /// Create a new authentication client
    pub fn new(config: Config) -> Self {
        Self::new_with_http_config(config, crate::http::HttpClientConfig::default())
    }

    /// Create a new authentication client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        http_config: crate::http::HttpClientConfig,
    ) -> Self {
        // Try to load stored 3-legged token synchronously
        let stored_token = Self::load_stored_token_static(&config);

        // Create HTTP client with configured timeouts
        let http_client = http_config
            .create_client()
            .unwrap_or_else(|_| reqwest::Client::new()); // Fallback to default if config fails

        Self {
            config,
            http_client,
            cached_2leg_token: Arc::new(RwLock::new(None)),
            cached_3leg_token: Arc::new(RwLock::new(stored_token)),
        }
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
        storage.save(token)
    }

    /// Load token from persistent storage
    #[allow(dead_code)]
    fn load_stored_token(&self) -> Result<StoredToken> {
        let storage = self.token_storage();
        storage
            .load()?
            .ok_or_else(|| anyhow::anyhow!("No stored token found"))
    }

    /// Delete stored token
    pub fn delete_stored_token(&self) -> Result<()> {
        let storage = self.token_storage();
        storage.delete()
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

        anyhow::bail!("Not logged in. Please run 'raps auth login' first.")
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
            .http_client
            .post(&url)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&params)
            .send()
            .await
            .context("Failed to send authentication request")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Authentication failed with status {}: {}",
                status,
                error_text
            );
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        Ok(token_response)
    }

    /// Login with 3-legged OAuth using device code flow (headless-friendly)
    pub async fn login_device(&self, scopes: &[&str]) -> Result<StoredToken> {
        let url = format!("{}/authentication/v2/device", self.config.base_url);

        // Request device code
        let params = [("client_id", &self.config.client_id)];
        let response = self
            .http_client
            .post(&url)
            .form(&params)
            .send()
            .await
            .context("Failed to request device code")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Device code request failed ({}): {}", status, error_text);
        }

        let device_response: DeviceCodeResponse = response
            .json()
            .await
            .context("Failed to parse device code response")?;

        // Display instructions to user
        println!("\n{}", "Device Code Authentication".bold().cyan());
        println!("{}", "─".repeat(50));
        println!(
            "  {} {}",
            "User Code:".bold(),
            device_response.user_code.bold().yellow()
        );
        println!(
            "  {} {}",
            "Verification URL:".bold(),
            device_response.verification_uri.cyan()
        );
        if let Some(ref complete_url) = device_response.verification_uri_complete {
            println!("  {} {}", "Complete URL:".bold(), complete_url.cyan());
        }
        println!(
            "\n{}",
            "Please visit the URL above and enter the user code to authorize.".dimmed()
        );
        println!(
            "{}",
            format!(
                "Waiting for authorization (expires in {} seconds)...",
                device_response.expires_in
            )
            .dimmed()
        );
        println!("{}", "─".repeat(50));

        // Poll for token
        let poll_interval = Duration::from_secs(device_response.interval.unwrap_or(5));
        let expires_at = Instant::now() + Duration::from_secs(device_response.expires_in);
        let mut last_poll = Instant::now();

        loop {
            // Check if expired
            if Instant::now() >= expires_at {
                anyhow::bail!("Device code expired. Please try again.");
            }

            // Wait for polling interval
            let elapsed = last_poll.elapsed();
            if elapsed < poll_interval {
                tokio::time::sleep(poll_interval - elapsed).await;
            }
            last_poll = Instant::now();

            // Poll for token
            let token_url = self.config.auth_url();
            let poll_params = [
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &device_response.device_code),
                ("client_id", &self.config.client_id),
                ("client_secret", &self.config.client_secret),
            ];

            let poll_response = self
                .http_client
                .post(&token_url)
                .form(&poll_params)
                .send()
                .await
                .context("Failed to poll for token")?;

            if poll_response.status().is_success() {
                let token: TokenResponse = poll_response
                    .json()
                    .await
                    .context("Failed to parse token response")?;

                println!("\n{} Authorization successful!", "✓".green().bold());

                // Store the token
                let stored = StoredToken {
                    access_token: token.access_token.clone(),
                    refresh_token: token.refresh_token.clone(),
                    expires_at: chrono::Utc::now().timestamp() + token.expires_in as i64,
                    scopes: scopes.iter().map(|s| s.to_string()).collect(),
                };

                self.save_token(&stored)?;

                // Update cache
                {
                    let mut cache = self.cached_3leg_token.write().await;
                    *cache = Some(stored.clone());
                }

                return Ok(stored);
            } else {
                // Check error response
                let error_text = poll_response.text().await.unwrap_or_default();
                if error_text.contains("authorization_pending") {
                    // Still waiting, continue polling
                    print!(".");
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                    continue;
                } else if error_text.contains("slow_down") {
                    // Slow down polling
                    tokio::time::sleep(poll_interval * 2).await;
                    continue;
                } else if error_text.contains("expired_token") {
                    anyhow::bail!("Device code expired. Please try again.");
                } else {
                    anyhow::bail!("Token polling failed: {}", error_text);
                }
            }
        }
    }

    /// Login with a provided access token (for CI/CD scenarios)
    pub async fn login_with_token(
        &self,
        access_token: String,
        refresh_token: Option<String>,
        expires_in: u64,
        scopes: Vec<String>,
    ) -> Result<StoredToken> {
        // Validate token by fetching user info
        let user_info = self.get_user_info_with_token(&access_token).await?;

        println!(
            "{} Token validated for user: {}",
            "✓".green().bold(),
            user_info.email.as_deref().unwrap_or("unknown")
        );

        // Store the token
        let stored = StoredToken {
            access_token: access_token.clone(),
            refresh_token,
            expires_at: chrono::Utc::now().timestamp() + expires_in as i64,
            scopes,
        };

        self.save_token(&stored)?;

        // Update cache
        {
            let mut cache = self.cached_3leg_token.write().await;
            *cache = Some(stored.clone());
        }

        Ok(stored)
    }

    /// Get user info with a provided token (for validation)
    async fn get_user_info_with_token(&self, token: &str) -> Result<UserInfo> {
        let url = "https://api.userprofile.autodesk.com/userinfo";
        let response = self
            .http_client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to fetch user info")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to validate token ({}): {}", status, error_text);
        }

        let user: UserInfo = response.json().await.context("Failed to parse user info")?;

        Ok(user)
    }

    /// Start 3-legged OAuth login flow
    pub async fn login(&self, scopes: &[&str]) -> Result<StoredToken> {
        let state = uuid::Uuid::new_v4().to_string();
        let scope = scopes.join(" ");

        // Parse port from callback URL or default to DEFAULT_CALLBACK_PORT
        let preferred_port = match url::Url::parse(&self.config.callback_url) {
            Ok(u) => u.port().unwrap_or(crate::config::DEFAULT_CALLBACK_PORT),
            Err(_) => crate::config::DEFAULT_CALLBACK_PORT,
        };

        // Fallback ports (RAPS in leet speak + common alternatives)
        // 12495 = RAPS (R=12, A=4, P=9, S=5)
        // 7495 = RAPS alternative (7 looks like backwards R)
        // 9247 = Another leet variation
        let fallback_ports: Vec<u16> = vec![
            preferred_port, // Try user's preferred port first
            12495,          // 🌼 RAPS in leet (R=12, A=4, P=9, S=5)
            7495,           // 🌼 RAPS alternative
            9247,           // 🌼 RAPS variation
            3000,           // Common dev port
            5000,           // Common dev port
        ];

        // Try to bind to a port
        let mut server = None;
        let mut actual_port = preferred_port;

        for &port in &fallback_ports {
            match Server::http(format!("127.0.0.1:{}", port)) {
                Ok(s) => {
                    server = Some(s);
                    actual_port = port;
                    break;
                }
                Err(e) => {
                    if logging::debug() {
                        println!("Port {} unavailable: {}", port, e);
                    }
                    continue;
                }
            }
        }

        let server = server.ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to start callback server. Tried ports: {:?}. \
                 This usually means:\n\
                 1. All ports are in use by other applications\n\
                 2. Windows Firewall or antivirus is blocking localhost connections\n\
                 3. Hyper-V has reserved these ports\n\
                 \n\
                 Try:\n\
                 - Close other applications using these ports\n\
                 - Set APS_CALLBACK_URL=http://localhost:<custom-port>/callback\n\
                 - Run 'netsh interface ipv4 show excludedportrange protocol=tcp' to check reserved ports",
                fallback_ports
            )
        })?;

        println!("Callback server started on port {}", actual_port);
        if actual_port != preferred_port {
            println!(
                "  (Using fallback port {} - preferred port {} was unavailable)",
                actual_port, preferred_port
            );
        }

        // Build callback URL with the actual port we bound to
        let actual_callback_url = format!("http://localhost:{}/callback", actual_port);

        // Build authorization URL
        let auth_url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
            self.config.authorize_url(),
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(&actual_callback_url),
            urlencoding::encode(&scope),
            urlencoding::encode(&state)
        );

        println!("Opening browser for authentication...");
        println!("If the browser doesn't open, visit this URL:");
        println!("{}", auth_url);

        // Open browser
        if webbrowser::open(&auth_url).is_err() {
            println!("Failed to open browser automatically.");
        }

        println!("\nWaiting for authentication callback...");

        // Wait for callback - may receive multiple requests (favicon, etc.)
        #[allow(unused_assignments)]
        let mut auth_code: Option<String> = None;

        loop {
            let request = server
                .recv()
                .map_err(|e| anyhow::anyhow!("Failed to receive callback: {}", e))?;

            let url = request.url().to_string();
            println!("Received request: {}", url);

            // Skip non-callback requests (like favicon)
            if !url.starts_with("/callback") && !url.contains("code=") {
                let response = Response::from_string("Not found").with_status_code(404);
                request.respond(response).ok();
                continue;
            }

            // Parse the callback URL for code and state
            let parsed = url::Url::parse(&format!("http://localhost{}", url))?;
            let params: std::collections::HashMap<_, _> = parsed.query_pairs().collect();

            // Check for error
            if let Some(error) = params.get("error") {
                let desc = params
                    .get("error_description")
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let response = Response::from_string(format!(
                    "<html><body><h1>Login Failed</h1><p>{}: {}</p></body></html>",
                    error, desc
                ))
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap(),
                );
                request.respond(response).ok();
                anyhow::bail!("Authorization error: {error} - {desc}");
            }

            // Check state
            let returned_state = params
                .get("state")
                .ok_or_else(|| anyhow::anyhow!("Missing state parameter"))?;
            if returned_state != &state {
                let response = Response::from_string("State mismatch").with_status_code(400);
                request.respond(response).ok();
                anyhow::bail!("State mismatch - possible CSRF attack");
            }

            // Get authorization code
            if let Some(code) = params.get("code") {
                auth_code = Some(code.to_string());

                // Send success response to browser IMMEDIATELY
                let response = Response::from_string(
                    "<html><body><h1>Login Successful!</h1><p>You can close this window and return to the terminal.</p></body></html>"
                ).with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap()
                );
                request.respond(response).ok();
                break;
            }
        }

        let code = auth_code.ok_or_else(|| anyhow::anyhow!("No authorization code received"))?;

        println!("Authorization code received, exchanging for token...");

        // Exchange code for tokens
        let token = self.exchange_code(&code).await?;

        // Store the token
        let stored = StoredToken {
            access_token: token.access_token.clone(),
            refresh_token: token.refresh_token.clone(),
            expires_at: chrono::Utc::now().timestamp() + token.expires_in as i64,
            scopes: scopes.iter().map(|s| s.to_string()).collect(),
        };

        self.save_token(&stored)?;

        // Update cache
        {
            let mut cache = self.cached_3leg_token.write().await;
            *cache = Some(stored.clone());
        }

        Ok(stored)
    }

    /// Exchange authorization code for tokens
    async fn exchange_code(&self, code: &str) -> Result<TokenResponse> {
        let url = self.config.auth_url();

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.config.callback_url),
        ];

        let response = self
            .http_client
            .post(&url)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&params)
            .send()
            .await
            .context("Failed to exchange authorization code")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Token exchange failed ({}): {}", status, error_text);
        }

        let token: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        Ok(token)
    }

    /// Refresh an expired access token
    async fn refresh_token(&self, refresh_token: String) -> Result<String> {
        let url = self.config.auth_url();

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
        ];

        let response = self
            .http_client
            .post(&url)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&params)
            .send()
            .await
            .context("Failed to refresh token")?;

        if !response.status().is_success() {
            // Refresh failed, clear stored token
            self.delete_stored_token().ok();
            let mut cache = self.cached_3leg_token.write().await;
            *cache = None;
            anyhow::bail!("Token refresh failed. Please login again with 'raps auth login'");
        }

        let token: TokenResponse = response
            .json()
            .await
            .context("Failed to parse refresh response")?;

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

    /// Logout - clear stored tokens
    pub async fn logout(&self) -> Result<()> {
        self.delete_stored_token()?;
        let mut cache = self.cached_3leg_token.write().await;
        *cache = None;
        Ok(())
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

    /// Get config reference
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get user profile information (requires 3-legged auth with user:read or user-profile:read scope)
    pub async fn get_user_info(&self) -> Result<UserInfo> {
        let token = self.get_3leg_token().await?;
        self.get_user_info_with_token(&token).await
    }

    /// Get token expiry timestamp
    pub async fn get_token_expiry(&self) -> Option<i64> {
        let cache = self.cached_3leg_token.read().await;
        cache.as_ref().map(|t| t.expires_at)
    }
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

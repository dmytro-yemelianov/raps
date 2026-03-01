// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! 3-legged OAuth (authorization code) flow

use anyhow::{Context, Result};
use std::time::Duration;
use tiny_http::{Response, Server};

use super::types::TokenResponse;
use super::AuthClient;
use crate::config::DEFAULT_CALLBACK_PORT;
use crate::types::StoredToken;

impl AuthClient {
    /// Get a valid 3-legged access token (requires prior login)
    ///
    /// Uses Mutex-based coordination to ensure only one refresh occurs at a time.
    /// Concurrent callers wait and receive the newly refreshed token.
    pub async fn get_3leg_token(&self) -> Result<String> {
        loop {
            let refresh_token_to_use: Option<String>;
            {
                let cache = self.cached_3leg_token.lock().await;
                if let Some(ref token) = cache.token {
                    if token.is_valid() {
                        return Ok(token.access_token.clone());
                    }
                    if cache.refreshing {
                        // Another task is already refreshing; drop lock and wait
                        drop(cache);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                    refresh_token_to_use = token.refresh_token.clone();
                } else {
                    refresh_token_to_use = None;
                }
            }

            // Try to refresh if we have a refresh token
            if let Some(refresh) = refresh_token_to_use {
                // Mark as refreshing
                {
                    let mut cache = self.cached_3leg_token.lock().await;
                    cache.refreshing = true;
                }
                let result = self.refresh_token(refresh).await;
                // Always reset the refreshing flag, even on error
                if result.is_err() {
                    let mut cache = self.cached_3leg_token.lock().await;
                    cache.refreshing = false;
                }
                return result;
            }

            anyhow::bail!("Not logged in. Please run 'raps auth login' first.")
        }
    }

    /// Check if user is logged in with 3-legged OAuth
    pub async fn is_logged_in(&self) -> bool {
        let cache = self.cached_3leg_token.lock().await;
        if let Some(ref token) = cache.token {
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

    /// Start 3-legged OAuth login flow
    pub async fn login(&self, scopes: &[&str]) -> Result<StoredToken> {
        self.config.require_credentials()?;

        let state = uuid::Uuid::new_v4().to_string();
        let scope = scopes.join(" ");

        // Parse port from callback URL or default to DEFAULT_CALLBACK_PORT
        let preferred_port = match url::Url::parse(&self.config.callback_url) {
            Ok(u) => u.port().unwrap_or(DEFAULT_CALLBACK_PORT),
            Err(_) => DEFAULT_CALLBACK_PORT,
        };

        // Fallback ports (RAPS in leet speak + common alternatives)
        let fallback_ports: Vec<u16> = vec![preferred_port, 12495, 7495, 9247, 3000, 5000];

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
                    if crate::logging::debug() {
                        println!("Port {} unavailable: {}", port, e);
                    }
                    continue;
                }
            }
        }

        let server = server.ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to start callback server. Tried ports: {:?}.",
                fallback_ports
            )
        })?;

        tracing::info!(port = actual_port, "Callback server started");
        if actual_port != preferred_port {
            tracing::info!(
                fallback_port = actual_port,
                preferred_port,
                "Using fallback port"
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

        eprintln!("Opening browser for authentication...");
        eprintln!("If the browser doesn't open, visit this URL:");
        eprintln!("{}", auth_url);

        // Open browser
        if webbrowser::open(&auth_url).is_err() {
            eprintln!("Failed to open browser automatically.");
        }

        eprintln!("\nWaiting for authentication callback...");

        // Wait for callback
        #[allow(unused_assignments)]
        let mut auth_code: Option<String> = None;

        let server = std::sync::Arc::new(server);
        loop {
            let server_clone = server.clone();
            let request = tokio::task::spawn_blocking(move || server_clone.recv())
                .await
                .context("Callback server thread panicked")?
                .map_err(|e| anyhow::anyhow!("Failed to receive callback: {}", e))?;

            let url = request.url().to_string();
            tracing::debug!("Received callback request");

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
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..])
                        .expect("Content-Type: text/html is a valid header"),
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

                // Send success response to browser
                let response = Response::from_string(
                    "<html><body><h1>Login Successful!</h1><p>You can close this window and return to the terminal.</p></body></html>"
                ).with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).expect("Content-Type: text/html is a valid header")
                );
                request.respond(response).ok();
                break;
            }
        }

        let code = auth_code.ok_or_else(|| anyhow::anyhow!("No authorization code received"))?;

        println!("Authorization code received, exchanging for token...");

        // Exchange code for tokens (must use the actual callback URL that was sent in the authorize request)
        let token = self.exchange_code(&code, &actual_callback_url).await?;

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
            let mut cache = self.cached_3leg_token.lock().await;
            cache.token = Some(stored.clone());
        }

        Ok(stored)
    }

    /// Exchange authorization code for tokens
    async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<TokenResponse> {
        let url = self.config.auth_url();

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ];

        let _auth_start = std::time::Instant::now();
        let response = self
            .http_client
            .post(&url)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&params)
            .send()
            .await
            .context("Failed to exchange authorization code")?;
        crate::profiler::record_http_request(_auth_start.elapsed());

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            let redacted = crate::logging::redact_secrets(&error_text);
            anyhow::bail!("Token exchange failed ({status}): {redacted}");
        }

        let token: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        Ok(token)
    }

    /// Refresh an expired access token
    ///
    /// On failure: preserves cached token (does not clear it), resets refreshing flag.
    /// On success: updates cached token, resets refreshing flag.
    async fn refresh_token(&self, refresh_token: String) -> Result<String> {
        self.config.require_credentials()?;

        let url = self.config.auth_url();

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
        ];

        let _auth_start = std::time::Instant::now();
        let response = self
            .http_client
            .post(&url)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&params)
            .send()
            .await
            .context("Failed to refresh token")?;
        crate::profiler::record_http_request(_auth_start.elapsed());

        if !response.status().is_success() {
            // Refresh failed -- preserve cached token, just reset refreshing flag
            {
                let mut cache = self.cached_3leg_token.lock().await;
                cache.refreshing = false;
            }
            anyhow::bail!("Token refresh failed. Please login again with 'raps auth login'");
        }

        let token: TokenResponse = response
            .json()
            .await
            .context("Failed to parse refresh response")?;

        // Update stored token, preserving scopes from the original
        let original_scopes = {
            let cache = self.cached_3leg_token.lock().await;
            cache
                .token
                .as_ref()
                .map(|t| t.scopes.clone())
                .unwrap_or_default()
        };
        let stored = StoredToken {
            access_token: token.access_token.clone(),
            refresh_token: token.refresh_token.or(Some(refresh_token)),
            expires_at: chrono::Utc::now().timestamp() + token.expires_in as i64,
            scopes: original_scopes,
        };

        self.save_token(&stored)?;

        {
            let mut cache = self.cached_3leg_token.lock().await;
            cache.token = Some(stored);
            cache.refreshing = false;
        }

        Ok(token.access_token)
    }

    /// Logout - clear stored tokens
    pub async fn logout(&self) -> Result<()> {
        self.delete_stored_token()?;
        let mut cache = self.cached_3leg_token.lock().await;
        cache.token = None;
        cache.refreshing = false;
        Ok(())
    }

    /// Get user profile information (requires 3-legged auth with user:read or user-profile:read scope)
    pub async fn get_user_info(&self) -> Result<super::types::UserInfo> {
        let token = self.get_3leg_token().await?;
        self.get_user_info_with_token(&token).await
    }

    /// Get token expiry timestamp
    pub async fn get_token_expiry(&self) -> Option<i64> {
        let cache = self.cached_3leg_token.lock().await;
        cache.token.as_ref().map(|t| t.expires_at)
    }
}

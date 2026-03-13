// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Authentication module for APS OAuth 2.0
//!
//! Implements both 2-legged (client credentials) and 3-legged (authorization code) OAuth flows.

mod device_code;
mod three_leg;
mod token_ops;
mod two_leg;
pub mod types;

#[cfg(test)]
mod tests;

pub use types::*;

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::http::HttpClientConfig;
use anyhow::Context;
use types::{CachedToken, TokenCache};

/// Authentication client for APS
///
/// Handles OAuth 2.0 token acquisition for both 2-legged and 3-legged flows.
#[derive(Clone)]
pub struct AuthClient {
    pub(crate) config: Config,
    pub(crate) http_client: reqwest::Client,
    pub(crate) cached_2leg_token: Arc<RwLock<Option<CachedToken>>>,
    pub(crate) cached_3leg_token: Arc<tokio::sync::Mutex<TokenCache>>,
    /// Notify waiters when a 3-legged token refresh completes.
    pub(crate) token_refresh_notify: Arc<tokio::sync::Notify>,
}

impl AuthClient {
    /// Create a new authentication client
    pub fn new(config: Config) -> Self {
        Self::new_with_http_config(config, HttpClientConfig::default())
            .expect("default HTTP client configuration must always succeed")
    }

    /// Create a new authentication client with custom HTTP config.
    ///
    /// Returns an error if the HTTP client cannot be built (e.g. invalid proxy URL).
    pub fn new_with_http_config(
        config: Config,
        http_config: HttpClientConfig,
    ) -> anyhow::Result<Self> {
        // Try to load stored 3-legged token synchronously
        let stored_token = Self::load_stored_token_static(&config);

        // Create HTTP client — propagate errors instead of silently falling back.
        let http_client = http_config
            .create_client()
            .context("Failed to initialise HTTP client for authentication")?;

        Ok(Self {
            config,
            http_client,
            cached_2leg_token: Arc::new(RwLock::new(None)),
            cached_3leg_token: Arc::new(tokio::sync::Mutex::new(TokenCache {
                token: stored_token,
                refreshing: false,
            })),
            token_refresh_notify: Arc::new(tokio::sync::Notify::new()),
        })
    }

    /// Get config reference
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Set a 3-legged token for testing purposes
    /// This allows integration tests to simulate a logged-in state
    #[cfg(any(test, feature = "test-utils"))]
    pub async fn set_3leg_token_for_testing(&self, token: crate::types::StoredToken) {
        let mut cache = self.cached_3leg_token.lock().await;
        cache.token = Some(token);
    }

    /// Set a 2-legged token for testing purposes
    /// This allows integration tests to simulate having a valid cached token
    #[cfg(any(test, feature = "test-utils"))]
    pub async fn set_2leg_token_for_testing(&self, access_token: String, expires_in_secs: u64) {
        use std::time::{Duration, Instant};
        let mut cache = self.cached_2leg_token.write().await;
        *cache = Some(CachedToken {
            access_token,
            expires_at: Instant::now() + Duration::from_secs(expires_in_secs),
        });
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Token storage, retrieval, validation, and login-with-token operations

use anyhow::{Context, Result};
use colored::Colorize;

use super::types::UserInfo;
use super::AuthClient;
use crate::config::Config;
use crate::storage::{StorageBackend, TokenStorage};
use crate::types::StoredToken;

impl AuthClient {
    /// Get token storage instance
    pub(crate) fn token_storage(&self) -> TokenStorage {
        let backend = StorageBackend::from_env();
        TokenStorage::new(backend)
    }

    /// Load token from persistent storage (static version for initialization)
    pub(crate) fn load_stored_token_static(_config: &Config) -> Option<StoredToken> {
        let backend = StorageBackend::from_env();
        let storage = TokenStorage::new(backend);
        storage.load().ok().flatten()
    }

    /// Save token to persistent storage
    pub(crate) fn save_token(&self, token: &StoredToken) -> Result<()> {
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
            "OK".green().bold(),
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
            let mut cache = self.cached_3leg_token.lock().await;
            cache.token = Some(stored.clone());
        }

        Ok(stored)
    }

    /// Get user info with a provided token (for validation)
    pub(crate) async fn get_user_info_with_token(&self, token: &str) -> Result<UserInfo> {
        let url = if self.config.base_url != "https://developer.api.autodesk.com" {
            format!("{}/userinfo", self.config.base_url)
        } else {
            "https://api.userprofile.autodesk.com/userinfo".to_string()
        };
        let _auth_start = std::time::Instant::now();
        let response = self
            .http_client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .context("Failed to fetch user info")?;
        crate::profiler::record_http_request(_auth_start.elapsed());

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            let redacted = crate::logging::redact_secrets(&error_text);
            anyhow::bail!("Failed to validate token ({status}): {redacted}");
        }

        let user: UserInfo = response.json().await.context("Failed to parse user info")?;

        Ok(user)
    }
}

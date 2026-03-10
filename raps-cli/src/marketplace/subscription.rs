// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! License validation cache and subscription state management.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{auth::MarketplaceAuth, client::MarketplaceClient};

/// Locally cached license validation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedValidation {
    /// ISO 8601 datetime string — cache is valid until this time.
    pub valid_until: String,
    /// Plugin slugs the license grants access to.
    pub plugins: Vec<String>,
}

impl CachedValidation {
    /// Returns `true` if the cache is still valid (valid_until is in the future).
    pub fn is_valid(&self) -> bool {
        self.valid_until
            .parse::<DateTime<Utc>>()
            .map(|dt| dt > Utc::now())
            .unwrap_or(false)
    }
}

/// Manages license validation with a 7-day local cache.
pub struct SubscriptionManager;

impl SubscriptionManager {
    fn cache_path() -> Option<PathBuf> {
        BaseDirs::new().map(|b| b.cache_dir().join("raps").join("marketplace_license.json"))
    }

    fn load_cache() -> Option<CachedValidation> {
        let path = Self::cache_path()?;
        let data = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn save_cache(cache: &CachedValidation) {
        if let Some(path) = Self::cache_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string(cache) {
                let _ = std::fs::write(&path, json);
            }
        }
    }

    /// Validate a license key, using the local cache if still valid.
    ///
    /// 1. If cache exists and `valid_until` is in the future → return cached result.
    /// 2. Otherwise call `POST /license/validate`, update cache, return result.
    pub async fn validate(key: &str) -> Result<CachedValidation> {
        // Check cache first
        if let Some(cached) = Self::load_cache() {
            if cached.is_valid() {
                return Ok(cached);
            }
        }

        // Cache miss or expired — call the API
        let client = MarketplaceClient::new()?;
        let response = client.validate_license(key).await?;
        let cached = CachedValidation {
            valid_until: response.valid_until,
            plugins: response.plugins,
        };
        Self::save_cache(&cached);
        Ok(cached)
    }

    /// Get the current subscription state from cache (does not make network calls).
    pub fn get_subscription() -> Option<CachedValidation> {
        Self::load_cache().filter(|c| c.is_valid())
    }

    /// Returns `true` if the license grants access to pro features.
    /// A valid subscription with at least one plugin entitlement grants pro access.
    #[allow(dead_code)]
    pub fn can_use_pro() -> bool {
        Self::load_cache()
            .filter(|c| c.is_valid())
            .map(|c| !c.plugins.is_empty())
            .unwrap_or(false)
    }

    /// Returns `true` if the license grants access to a specific pro plugin.
    #[allow(dead_code)]
    pub fn can_use_plugin(slug: &str) -> bool {
        Self::load_cache()
            .filter(|c| c.is_valid())
            .map(|c| c.plugins.iter().any(|p| p == slug))
            .unwrap_or(false)
    }

    /// Register and validate a license key immediately.
    /// Stores the key in the keyring and fetches a fresh validation.
    pub async fn register_license(key: &str) -> Result<CachedValidation> {
        // Validate first (before storing) to catch invalid keys early
        let client = MarketplaceClient::new()?;
        let response = client
            .validate_license(key)
            .await
            .context("License key validation failed — check that the key is correct")?;

        // Key is valid — store it
        MarketplaceAuth::store_license_key(key)?;

        let cached = CachedValidation {
            valid_until: response.valid_until,
            plugins: response.plugins,
        };
        Self::save_cache(&cached);
        Ok(cached)
    }

    /// Clear the local cache (forces re-validation on next use).
    pub fn clear_cache() {
        if let Some(path) = Self::cache_path() {
            let _ = std::fs::remove_file(path);
        }
    }

    /// Format subscription status for display.
    pub fn format_subscription_status(cache: &CachedValidation) -> String {
        let until = cache
            .valid_until
            .parse::<DateTime<Utc>>()
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|_| cache.valid_until.clone());

        if cache.plugins.is_empty() {
            format!("Active (no plugins) — valid until {}", until)
        } else {
            format!(
                "Active ({} plugin{}) — valid until {}",
                cache.plugins.len(),
                if cache.plugins.len() == 1 { "" } else { "s" },
                until
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_validation_expired() {
        let c = CachedValidation {
            valid_until: "2020-01-01T00:00:00Z".to_string(),
            plugins: vec!["acc-bulk".to_string()],
        };
        assert!(!c.is_valid(), "Past date should be expired");
    }

    #[test]
    fn cached_validation_future() {
        let future = (Utc::now() + chrono::Duration::days(3)).to_rfc3339();
        let c = CachedValidation {
            valid_until: future,
            plugins: vec!["acc-bulk".to_string()],
        };
        assert!(c.is_valid(), "Future date should be valid");
    }

    #[test]
    fn format_subscription_status_plural() {
        let future = (Utc::now() + chrono::Duration::days(7)).to_rfc3339();
        let c = CachedValidation {
            valid_until: future,
            plugins: vec!["acc-bulk".to_string(), "acc-reports".to_string()],
        };
        let status = SubscriptionManager::format_subscription_status(&c);
        assert!(status.contains("2 plugins"));
    }
}

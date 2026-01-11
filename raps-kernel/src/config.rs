// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Configuration module for APS CLI
//!
//! Handles loading and managing APS credentials from environment variables or .env file.

use anyhow::{Context, Result};
use std::env;

use crate::http::HttpClientConfig;
use crate::types::{ProfileConfig, ProfilesData};

/// Default callback port for 3-legged OAuth
pub const DEFAULT_CALLBACK_PORT: u16 = 8080;

/// APS Configuration containing client credentials
#[derive(Debug, Clone)]
pub struct Config {
    /// APS Client ID (from APS Developer Portal)
    pub client_id: String,
    /// APS Client Secret (from APS Developer Portal)
    pub client_secret: String,
    /// Base URL for APS API (defaults to production)
    pub base_url: String,
    /// Callback URL for 3-legged OAuth
    pub callback_url: String,
    /// Design Automation nickname (optional)
    #[allow(dead_code)]
    pub da_nickname: Option<String>,
    /// HTTP client configuration
    pub http_config: HttpClientConfig,
}

impl Config {
    /// Load configuration with precedence: flags > env vars > active profile > defaults
    ///
    /// Looks for:
    /// 1. Environment variables (APS_CLIENT_ID, APS_CLIENT_SECRET, etc.)
    /// 2. Active profile configuration (if set)
    /// 3. Defaults
    pub fn from_env() -> Result<Self> {
        // Try to load .env file if it exists (silently ignore if not found)
        let _ = dotenvy::dotenv();

        // Load profile data
        let profile_data = Self::load_profile_data().ok();

        // Determine values with precedence: env vars > profile > defaults
        let client_id = env::var("APS_CLIENT_ID")
            .or_else(|_| {
                profile_data
                    .as_ref()
                    .and_then(|(_, profile)| profile.client_id.clone())
                    .ok_or(env::VarError::NotPresent)
            })
            .context(
                "APS_CLIENT_ID not set. Set it via:\n  - Environment variable: APS_CLIENT_ID\n  - Profile: raps config profile create <name> && raps config set client_id <value>",
            )?;

        let client_secret = env::var("APS_CLIENT_SECRET")
            .or_else(|_| {
                profile_data
                    .as_ref()
                    .and_then(|(_, profile)| profile.client_secret.clone())
                    .ok_or(env::VarError::NotPresent)
            })
            .context(
                "APS_CLIENT_SECRET not set. Set it via:\n  - Environment variable: APS_CLIENT_SECRET\n  - Profile: raps config profile create <name> && raps config set client_secret <value>",
            )?;

        let base_url = env::var("APS_BASE_URL")
            .or_else(|_| {
                profile_data
                    .as_ref()
                    .and_then(|(_, profile)| profile.base_url.clone())
                    .ok_or(env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| "https://developer.api.autodesk.com".to_string());

        let callback_url = env::var("APS_CALLBACK_URL")
            .or_else(|_| {
                profile_data
                    .as_ref()
                    .and_then(|(_, profile)| profile.callback_url.clone())
                    .ok_or(env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| format!("http://localhost:{}/callback", DEFAULT_CALLBACK_PORT));

        let da_nickname = env::var("APS_DA_NICKNAME").ok().or_else(|| {
            profile_data
                .as_ref()
                .and_then(|(_, profile)| profile.da_nickname.clone())
        });

        Ok(Self {
            client_id,
            client_secret,
            base_url,
            callback_url,
            da_nickname,
            http_config: HttpClientConfig::default(),
        })
    }

    /// Load profile data from disk
    fn load_profile_data() -> Result<(String, ProfileConfig)> {
        let data = load_profiles()?;
        let profile_name = data
            .active_profile
            .ok_or_else(|| anyhow::anyhow!("No active profile"))?;

        let profile = data
            .profiles
            .get(&profile_name)
            .ok_or_else(|| anyhow::anyhow!("Active profile '{}' not found", profile_name))?
            .clone();

        Ok((profile_name, profile))
    }

    /// Get the authentication endpoint URL
    pub fn auth_url(&self) -> String {
        format!("{}/authentication/v2/token", self.base_url)
    }

    /// Get the authorization URL for 3-legged OAuth
    pub fn authorize_url(&self) -> String {
        format!("{}/authentication/v2/authorize", self.base_url)
    }

    /// Get the OSS API base URL
    pub fn oss_url(&self) -> String {
        format!("{}/oss/v2", self.base_url)
    }

    /// Get the Model Derivative API base URL
    pub fn derivative_url(&self) -> String {
        format!("{}/modelderivative/v2", self.base_url)
    }

    /// Get the Data Management API base URL (for hubs/projects)
    pub fn project_url(&self) -> String {
        format!("{}/project/v1", self.base_url)
    }

    /// Get the Data Management API base URL (for folders/items)
    pub fn data_url(&self) -> String {
        format!("{}/data/v1", self.base_url)
    }

    /// Get the Webhooks API base URL
    pub fn webhooks_url(&self) -> String {
        format!("{}/webhooks/v1", self.base_url)
    }

    /// Get the Design Automation API base URL
    pub fn da_url(&self) -> String {
        format!("{}/da/us-east/v3", self.base_url)
    }

    /// Get the ACC Issues API base URL
    pub fn issues_url(&self) -> String {
        format!("{}/construction/issues/v1", self.base_url)
    }

    /// Get the Reality Capture API base URL
    pub fn reality_capture_url(&self) -> String {
        format!("{}/photo-to-3d/v1", self.base_url)
    }
}

/// Load profiles from disk
pub fn load_profiles() -> Result<ProfilesData> {
    let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
        .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

    let profiles_path = proj_dirs.config_dir().join("profiles.json");

    if !profiles_path.exists() {
        return Ok(ProfilesData::default());
    }

    let content =
        std::fs::read_to_string(&profiles_path).context("Failed to read profiles file")?;

    let data: ProfilesData =
        serde_json::from_str(&content).context("Failed to parse profiles file")?;

    Ok(data)
}

/// Save profiles to disk
pub fn save_profiles(data: &ProfilesData) -> Result<()> {
    let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
        .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

    let config_dir = proj_dirs.config_dir();
    std::fs::create_dir_all(config_dir)?;

    let profiles_path = config_dir.join("profiles.json");
    let content = serde_json::to_string_pretty(data)?;
    std::fs::write(&profiles_path, content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        Config {
            client_id: "test_client_id".to_string(),
            client_secret: "test_secret".to_string(),
            base_url: "https://developer.api.autodesk.com".to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        }
    }

    #[test]
    fn test_auth_url() {
        let config = create_test_config();
        let url = config.auth_url();
        assert_eq!(
            url,
            "https://developer.api.autodesk.com/authentication/v2/token"
        );
    }

    #[test]
    fn test_authorize_url() {
        let config = create_test_config();
        let url = config.authorize_url();
        assert_eq!(
            url,
            "https://developer.api.autodesk.com/authentication/v2/authorize"
        );
    }

    #[test]
    fn test_oss_url() {
        let config = create_test_config();
        let url = config.oss_url();
        assert_eq!(url, "https://developer.api.autodesk.com/oss/v2");
    }

    #[test]
    fn test_derivative_url() {
        let config = create_test_config();
        let url = config.derivative_url();
        assert_eq!(url, "https://developer.api.autodesk.com/modelderivative/v2");
    }

    #[test]
    fn test_project_url() {
        let config = create_test_config();
        let url = config.project_url();
        assert_eq!(url, "https://developer.api.autodesk.com/project/v1");
    }

    #[test]
    fn test_data_url() {
        let config = create_test_config();
        let url = config.data_url();
        assert_eq!(url, "https://developer.api.autodesk.com/data/v1");
    }

    #[test]
    fn test_webhooks_url() {
        let config = create_test_config();
        let url = config.webhooks_url();
        assert_eq!(url, "https://developer.api.autodesk.com/webhooks/v1");
    }

    #[test]
    fn test_da_url() {
        let config = create_test_config();
        let url = config.da_url();
        assert_eq!(url, "https://developer.api.autodesk.com/da/us-east/v3");
    }

    #[test]
    fn test_issues_url() {
        let config = create_test_config();
        let url = config.issues_url();
        assert_eq!(
            url,
            "https://developer.api.autodesk.com/construction/issues/v1"
        );
    }

    #[test]
    fn test_reality_capture_url() {
        let config = create_test_config();
        let url = config.reality_capture_url();
        assert_eq!(url, "https://developer.api.autodesk.com/photo-to-3d/v1");
    }

    #[test]
    fn test_custom_base_url() {
        let config = Config {
            client_id: "test".to_string(),
            client_secret: "secret".to_string(),
            base_url: "https://custom.api.example.com".to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        assert!(
            config
                .auth_url()
                .starts_with("https://custom.api.example.com")
        );
        assert!(
            config
                .oss_url()
                .starts_with("https://custom.api.example.com")
        );
    }

    #[test]
    fn test_config_with_da_nickname() {
        let config = Config {
            client_id: "test".to_string(),
            client_secret: "secret".to_string(),
            base_url: "https://developer.api.autodesk.com".to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: Some("my-nickname".to_string()),
            http_config: HttpClientConfig::default(),
        };
        assert_eq!(config.da_nickname, Some("my-nickname".to_string()));
    }

    #[test]
    fn test_all_urls_contain_base_url() {
        let config = create_test_config();
        let base = &config.base_url;

        assert!(config.auth_url().starts_with(base));
        assert!(config.authorize_url().starts_with(base));
        assert!(config.oss_url().starts_with(base));
        assert!(config.derivative_url().starts_with(base));
        assert!(config.project_url().starts_with(base));
        assert!(config.data_url().starts_with(base));
        assert!(config.webhooks_url().starts_with(base));
        assert!(config.da_url().starts_with(base));
        assert!(config.issues_url().starts_with(base));
        assert!(config.reality_capture_url().starts_with(base));
    }
}

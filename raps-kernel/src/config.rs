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

    /// Load configuration leniently — missing client_id/client_secret default to empty strings.
    /// Useful for commands that don't need API credentials (e.g., auth logout, auth status).
    pub fn from_env_lenient() -> Result<Self> {
        let _ = dotenvy::dotenv();

        let profile_data = Self::load_profile_data().ok();

        let client_id = env::var("APS_CLIENT_ID")
            .or_else(|_| {
                profile_data
                    .as_ref()
                    .and_then(|(_, profile)| profile.client_id.clone())
                    .ok_or(env::VarError::NotPresent)
            })
            .unwrap_or_default();

        let client_secret = env::var("APS_CLIENT_SECRET")
            .or_else(|_| {
                profile_data
                    .as_ref()
                    .and_then(|(_, profile)| profile.client_secret.clone())
                    .ok_or(env::VarError::NotPresent)
            })
            .unwrap_or_default();

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

    /// Validate that client credentials are configured.
    ///
    /// Call this before any operation that requires `client_id` / `client_secret`
    /// (2-legged auth, 3-legged login, token refresh). Returns a clear error
    /// telling the user how to set the missing value(s).
    pub fn require_credentials(&self) -> Result<()> {
        if self.client_id.is_empty() && self.client_secret.is_empty() {
            anyhow::bail!(
                "APS_CLIENT_ID and APS_CLIENT_SECRET are not set.\n\
                 Set them via environment variables or a profile:\n  \
                 export APS_CLIENT_ID=<your-client-id>\n  \
                 export APS_CLIENT_SECRET=<your-client-secret>\n  \
                 Or: raps config profile create <name> && raps config set client_id <value>"
            );
        }
        if self.client_id.is_empty() {
            anyhow::bail!(
                "APS_CLIENT_ID is not set.\n\
                 Set it via:\n  \
                 export APS_CLIENT_ID=<your-client-id>\n  \
                 Or: raps config set client_id <value>"
            );
        }
        if self.client_secret.is_empty() {
            anyhow::bail!(
                "APS_CLIENT_SECRET is not set.\n\
                 Set it via:\n  \
                 export APS_CLIENT_SECRET=<your-client-secret>\n  \
                 Or: raps config set client_secret <value>"
            );
        }
        Ok(())
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

    /// Get the RFI API base URL
    pub fn rfi_url(&self) -> String {
        format!("{}/construction/rfis/v2", self.base_url)
    }

    /// Get the Assets API base URL
    pub fn assets_url(&self) -> String {
        format!("{}/construction/assets/v1", self.base_url)
    }

    /// Get the Submittals API base URL
    pub fn submittals_url(&self) -> String {
        format!("{}/construction/submittals/v1", self.base_url)
    }

    /// Get the Checklists API base URL
    pub fn checklists_url(&self) -> String {
        format!("{}/construction/checklists/v1", self.base_url)
    }

    /// Get the AEC Data Model GraphQL API endpoint
    pub fn aec_graphql_url(&self) -> String {
        format!("{}/aec/graphql", self.base_url)
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

/// Resolved context values from profile + environment
#[derive(Debug, Clone, Default)]
pub struct ContextConfig {
    pub hub_id: Option<String>,
    pub project_id: Option<String>,
    pub account_id: Option<String>,
}

impl ContextConfig {
    /// Load context from env vars > active profile
    pub fn load() -> Self {
        let profile_data = Config::load_profile_data().ok();

        let hub_id = std::env::var("APS_HUB_ID").ok().or_else(|| {
            profile_data
                .as_ref()
                .and_then(|(_, p)| p.context_hub_id.clone())
        });

        let project_id = std::env::var("APS_PROJECT_ID").ok().or_else(|| {
            profile_data
                .as_ref()
                .and_then(|(_, p)| p.context_project_id.clone())
        });

        let account_id = std::env::var("APS_ACCOUNT_ID").ok().or_else(|| {
            profile_data
                .as_ref()
                .and_then(|(_, p)| p.context_account_id.clone())
        });

        Self {
            hub_id,
            project_id,
            account_id,
        }
    }

    /// Resolve hub ID from explicit flag, context, or None
    pub fn resolve_hub_id(&self, explicit: Option<String>) -> Option<String> {
        explicit.or_else(|| self.hub_id.clone())
    }

    /// Resolve project ID from explicit flag, context, or None
    pub fn resolve_project_id(&self, explicit: Option<String>) -> Option<String> {
        explicit.or_else(|| self.project_id.clone())
    }

    /// Resolve account ID from explicit flag, context, or None
    pub fn resolve_account_id(&self, explicit: Option<String>) -> Option<String> {
        explicit.or_else(|| self.account_id.clone())
    }
}

/// Save profiles to disk
pub fn save_profiles(data: &ProfilesData) -> Result<()> {
    let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
        .ok_or_else(|| anyhow::anyhow!("Failed to get project directories"))?;

    let config_dir = proj_dirs.config_dir();
    crate::security::create_dir_restricted(config_dir)?;

    let profiles_path = config_dir.join("profiles.json");
    let content = serde_json::to_string_pretty(data)?;
    std::fs::write(&profiles_path, content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

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

    #[rstest]
    #[case(Config::auth_url as fn(&Config) -> String, "/authentication/v2/token")]
    #[case(Config::authorize_url as fn(&Config) -> String, "/authentication/v2/authorize")]
    #[case(Config::oss_url as fn(&Config) -> String, "/oss/v2")]
    #[case(Config::derivative_url as fn(&Config) -> String, "/modelderivative/v2")]
    #[case(Config::project_url as fn(&Config) -> String, "/project/v1")]
    #[case(Config::data_url as fn(&Config) -> String, "/data/v1")]
    #[case(Config::webhooks_url as fn(&Config) -> String, "/webhooks/v1")]
    #[case(Config::da_url as fn(&Config) -> String, "/da/us-east/v3")]
    #[case(Config::issues_url as fn(&Config) -> String, "/construction/issues/v1")]
    #[case(Config::reality_capture_url as fn(&Config) -> String, "/photo-to-3d/v1")]
    #[case(Config::rfi_url as fn(&Config) -> String, "/construction/rfis/v2")]
    #[case(Config::assets_url as fn(&Config) -> String, "/construction/assets/v1")]
    #[case(Config::submittals_url as fn(&Config) -> String, "/construction/submittals/v1")]
    #[case(Config::checklists_url as fn(&Config) -> String, "/construction/checklists/v1")]
    fn test_url_builder(#[case] method: fn(&Config) -> String, #[case] suffix: &str) {
        let config = create_test_config();
        assert_eq!(
            method(&config),
            format!("https://developer.api.autodesk.com{suffix}")
        );
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
        assert!(config.rfi_url().starts_with(base));
        assert!(config.assets_url().starts_with(base));
        assert!(config.submittals_url().starts_with(base));
        assert!(config.checklists_url().starts_with(base));
        assert!(config.aec_graphql_url().starts_with(base));
    }

    #[test]
    fn test_default_callback_port() {
        assert_eq!(DEFAULT_CALLBACK_PORT, 8080);
    }

    #[test]
    fn test_default_callback_url_format() {
        let config = create_test_config();
        assert!(config.callback_url.contains("localhost"));
        assert!(config.callback_url.contains("callback"));
    }

    // ==================== Credential Validation Tests ====================

    #[test]
    fn test_require_credentials_both_set() {
        let config = create_test_config();
        assert!(config.require_credentials().is_ok());
    }

    #[test]
    fn test_require_credentials_both_empty() {
        let config = Config {
            client_id: "".to_string(),
            client_secret: "".to_string(),
            base_url: "https://developer.api.autodesk.com".to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let err = config.require_credentials().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("APS_CLIENT_ID"));
        assert!(msg.contains("APS_CLIENT_SECRET"));
    }

    #[test]
    fn test_require_credentials_missing_client_id() {
        let config = Config {
            client_id: "".to_string(),
            client_secret: "test_secret".to_string(),
            base_url: "https://developer.api.autodesk.com".to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let err = config.require_credentials().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("APS_CLIENT_ID"));
        assert!(!msg.contains("APS_CLIENT_SECRET"));
    }

    #[test]
    fn test_require_credentials_missing_client_secret() {
        let config = Config {
            client_id: "test_client_id".to_string(),
            client_secret: "".to_string(),
            base_url: "https://developer.api.autodesk.com".to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let err = config.require_credentials().unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("APS_CLIENT_ID"));
        assert!(msg.contains("APS_CLIENT_SECRET"));
    }

    // ==================== ContextConfig Tests ====================

    #[test]
    fn test_context_config_default() {
        let ctx = ContextConfig::default();
        assert!(ctx.hub_id.is_none());
        assert!(ctx.project_id.is_none());
        assert!(ctx.account_id.is_none());
    }

    #[test]
    fn test_context_config_resolve_with_explicit() {
        let ctx = ContextConfig {
            hub_id: Some("stored-hub".to_string()),
            project_id: Some("stored-proj".to_string()),
            account_id: None,
        };
        // Explicit value takes priority
        assert_eq!(
            ctx.resolve_hub_id(Some("explicit-hub".to_string())),
            Some("explicit-hub".to_string())
        );
        // Falls back to stored
        assert_eq!(
            ctx.resolve_project_id(None),
            Some("stored-proj".to_string())
        );
        // None when neither
        assert_eq!(ctx.resolve_account_id(None), None);
    }
}

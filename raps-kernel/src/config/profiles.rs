// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Profile management for configuration

use crate::error::{RapsError, Result};
use serde::{Deserialize, Serialize};
use std::env;

/// Default callback port for 3-legged OAuth
pub const DEFAULT_CALLBACK_PORT: u16 = 8080;

/// APS Configuration containing client credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub da_nickname: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            base_url: "https://developer.api.autodesk.com".to_string(),
            callback_url: format!("http://localhost:{}/callback", DEFAULT_CALLBACK_PORT),
            da_nickname: None,
        }
    }
}

impl Config {
    /// Load configuration with precedence: env vars > active profile > defaults
    pub fn from_env() -> Result<Self> {
        // Try to load .env file if it exists (silently ignore if not found)
        let _ = dotenvy::dotenv();

        let client_id = env::var("APS_CLIENT_ID").map_err(|_| RapsError::Config {
            message: "APS_CLIENT_ID not set. Set it via environment variable or profile."
                .to_string(),
        })?;

        let client_secret = env::var("APS_CLIENT_SECRET").map_err(|_| RapsError::Config {
            message: "APS_CLIENT_SECRET not set. Set it via environment variable or profile."
                .to_string(),
        })?;

        let base_url = env::var("APS_BASE_URL")
            .unwrap_or_else(|_| "https://developer.api.autodesk.com".to_string());

        let callback_url = env::var("APS_CALLBACK_URL")
            .unwrap_or_else(|_| format!("http://localhost:{}/callback", DEFAULT_CALLBACK_PORT));

        let da_nickname = env::var("APS_DA_NICKNAME").ok();

        Ok(Self {
            client_id,
            client_secret,
            base_url,
            callback_url,
            da_nickname,
        })
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
}

/// Profile configuration (stub - full implementation in raps CLI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Profile name
    pub name: String,
    /// APS Client ID (optional, can be overridden by env var)
    pub client_id: Option<String>,
    /// APS Client Secret (optional, can be overridden by env var)
    pub client_secret: Option<String>,
    /// Base URL for APS API (optional)
    pub base_url: Option<String>,
    /// Callback URL for 3-legged OAuth (optional)
    pub callback_url: Option<String>,
    /// Design Automation nickname (optional)
    pub da_nickname: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_callback_port() {
        assert_eq!(DEFAULT_CALLBACK_PORT, 8080);
    }

    #[test]
    #[ignore = "Requires isolated environment (no APS_CLIENT_ID set)"]
    fn test_config_from_env_missing_credentials() {
        // Save original values
        let orig_client_id = std::env::var("APS_CLIENT_ID").ok();
        let orig_client_secret = std::env::var("APS_CLIENT_SECRET").ok();

        // Clear env vars for this test
        unsafe {
            std::env::remove_var("APS_CLIENT_ID");
            std::env::remove_var("APS_CLIENT_SECRET");
        }

        let result = Config::from_env();
        assert!(
            result.is_err(),
            "Expected error when credentials are missing"
        );
        if let Err(RapsError::Config { message }) = result {
            assert!(message.contains("APS_CLIENT_ID") || message.contains("APS_CLIENT_SECRET"));
        }

        // Restore original values
        unsafe {
            if let Some(val) = orig_client_id {
                std::env::set_var("APS_CLIENT_ID", val);
            }
            if let Some(val) = orig_client_secret {
                std::env::set_var("APS_CLIENT_SECRET", val);
            }
        }
    }

    #[test]
    #[ignore = "Requires isolated environment (tests run in parallel and share env vars)"]
    fn test_config_from_env_with_defaults() {
        // Save original values
        let orig_client_id = std::env::var("APS_CLIENT_ID").ok();
        let orig_client_secret = std::env::var("APS_CLIENT_SECRET").ok();
        let orig_base_url = std::env::var("APS_BASE_URL").ok();
        let orig_callback_url = std::env::var("APS_CALLBACK_URL").ok();
        let orig_da_nickname = std::env::var("APS_DA_NICKNAME").ok();

        unsafe {
            std::env::set_var("APS_CLIENT_ID", "test_client_id");
            std::env::set_var("APS_CLIENT_SECRET", "test_client_secret");
            std::env::remove_var("APS_BASE_URL");
            std::env::remove_var("APS_CALLBACK_URL");
            std::env::remove_var("APS_DA_NICKNAME");
        }

        let config = Config::from_env();
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.client_id, "test_client_id");
        assert_eq!(config.client_secret, "test_client_secret");
        assert_eq!(config.base_url, "https://developer.api.autodesk.com");
        assert!(config.callback_url.contains("localhost"));
        assert!(config.callback_url.contains("8080"));
        assert_eq!(config.da_nickname, None);

        // Restore original values
        unsafe {
            if let Some(val) = orig_client_id {
                std::env::set_var("APS_CLIENT_ID", val);
            } else {
                std::env::remove_var("APS_CLIENT_ID");
            }
            if let Some(val) = orig_client_secret {
                std::env::set_var("APS_CLIENT_SECRET", val);
            } else {
                std::env::remove_var("APS_CLIENT_SECRET");
            }
            if let Some(val) = orig_base_url {
                std::env::set_var("APS_BASE_URL", val);
            } else {
                std::env::remove_var("APS_BASE_URL");
            }
            if let Some(val) = orig_callback_url {
                std::env::set_var("APS_CALLBACK_URL", val);
            } else {
                std::env::remove_var("APS_CALLBACK_URL");
            }
            if let Some(val) = orig_da_nickname {
                std::env::set_var("APS_DA_NICKNAME", val);
            } else {
                std::env::remove_var("APS_DA_NICKNAME");
            }
        }
    }

    #[test]
    #[ignore = "Requires isolated environment (tests run in parallel and share env vars)"]
    fn test_config_from_env_with_custom_urls() {
        // Save original values
        let orig_client_id = std::env::var("APS_CLIENT_ID").ok();
        let orig_client_secret = std::env::var("APS_CLIENT_SECRET").ok();
        let orig_base_url = std::env::var("APS_BASE_URL").ok();
        let orig_callback_url = std::env::var("APS_CALLBACK_URL").ok();
        let orig_da_nickname = std::env::var("APS_DA_NICKNAME").ok();

        unsafe {
            std::env::set_var("APS_CLIENT_ID", "test_client_id");
            std::env::set_var("APS_CLIENT_SECRET", "test_client_secret");
            std::env::set_var("APS_BASE_URL", "https://custom.com");
            std::env::set_var("APS_CALLBACK_URL", "https://custom.com/callback");
            std::env::set_var("APS_DA_NICKNAME", "test-nickname");
        }

        let config = Config::from_env();
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.base_url, "https://custom.com");
        assert_eq!(config.callback_url, "https://custom.com/callback");
        assert_eq!(config.da_nickname, Some("test-nickname".to_string()));

        // Restore original values
        unsafe {
            if let Some(val) = orig_client_id {
                std::env::set_var("APS_CLIENT_ID", val);
            } else {
                std::env::remove_var("APS_CLIENT_ID");
            }
            if let Some(val) = orig_client_secret {
                std::env::set_var("APS_CLIENT_SECRET", val);
            } else {
                std::env::remove_var("APS_CLIENT_SECRET");
            }
            if let Some(val) = orig_base_url {
                std::env::set_var("APS_BASE_URL", val);
            } else {
                std::env::remove_var("APS_BASE_URL");
            }
            if let Some(val) = orig_callback_url {
                std::env::set_var("APS_CALLBACK_URL", val);
            } else {
                std::env::remove_var("APS_CALLBACK_URL");
            }
            if let Some(val) = orig_da_nickname {
                std::env::set_var("APS_DA_NICKNAME", val);
            } else {
                std::env::remove_var("APS_DA_NICKNAME");
            }
        }
    }

    #[test]
    fn test_profile_serialize() {
        let profile = Profile {
            name: "test".to_string(),
            client_id: Some("id".to_string()),
            client_secret: Some("secret".to_string()),
            base_url: None,
            callback_url: None,
            da_nickname: None,
        };
        let json = serde_json::to_string(&profile);
        assert!(json.is_ok());
    }

    #[test]
    fn test_profile_deserialize() {
        let json = r#"{
            "name": "test",
            "client_id": "id",
            "client_secret": "secret"
        }"#;
        let profile: std::result::Result<Profile, _> = serde_json::from_str(json);
        assert!(profile.is_ok());
        let profile = profile.unwrap();
        assert_eq!(profile.name, "test");
        assert_eq!(profile.client_id, Some("id".to_string()));
    }
}

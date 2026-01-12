// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Shared types used across the kernel

use serde::{Deserialize, Serialize};

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

/// Profile configuration for credential management
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub base_url: Option<String>,
    pub callback_url: Option<String>,
    pub da_nickname: Option<String>,
    #[serde(default = "default_use_keychain")]
    pub use_keychain: bool,
}

fn default_use_keychain() -> bool {
    true
}

/// Profiles data containing all profiles and active profile name
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfilesData {
    pub active_profile: Option<String>,
    #[serde(default)]
    pub profiles: std::collections::HashMap<String, ProfileConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== StoredToken Tests ====================

    #[test]
    fn test_stored_token_serialization() {
        let token = StoredToken {
            access_token: "test_access_token".to_string(),
            refresh_token: Some("test_refresh_token".to_string()),
            expires_at: 1700000000,
            scopes: vec!["data:read".to_string(), "data:write".to_string()],
        };

        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("test_access_token"));
        assert!(json.contains("test_refresh_token"));
        assert!(json.contains("1700000000"));
        assert!(json.contains("data:read"));
    }

    #[test]
    fn test_stored_token_deserialization() {
        let json = r#"{
            "access_token": "my_token",
            "refresh_token": "my_refresh",
            "expires_at": 1700000000,
            "scopes": ["scope1", "scope2"]
        }"#;

        let token: StoredToken = serde_json::from_str(json).unwrap();
        assert_eq!(token.access_token, "my_token");
        assert_eq!(token.refresh_token, Some("my_refresh".to_string()));
        assert_eq!(token.expires_at, 1700000000);
        assert_eq!(token.scopes, vec!["scope1", "scope2"]);
    }

    #[test]
    fn test_stored_token_without_refresh_token() {
        let json = r#"{
            "access_token": "my_token",
            "refresh_token": null,
            "expires_at": 1700000000,
            "scopes": []
        }"#;

        let token: StoredToken = serde_json::from_str(json).unwrap();
        assert_eq!(token.refresh_token, None);
        assert!(token.scopes.is_empty());
    }

    #[test]
    fn test_stored_token_is_valid_future_expiry() {
        let future_timestamp = chrono::Utc::now().timestamp() + 3600; // 1 hour from now
        let token = StoredToken {
            access_token: "token".to_string(),
            refresh_token: None,
            expires_at: future_timestamp,
            scopes: vec![],
        };
        assert!(token.is_valid());
    }

    #[test]
    fn test_stored_token_is_valid_past_expiry() {
        let past_timestamp = chrono::Utc::now().timestamp() - 3600; // 1 hour ago
        let token = StoredToken {
            access_token: "token".to_string(),
            refresh_token: None,
            expires_at: past_timestamp,
            scopes: vec![],
        };
        assert!(!token.is_valid());
    }

    #[test]
    fn test_stored_token_is_valid_buffer() {
        // Token that expires in 30 seconds should be considered invalid (60s buffer)
        let soon_timestamp = chrono::Utc::now().timestamp() + 30;
        let token = StoredToken {
            access_token: "token".to_string(),
            refresh_token: None,
            expires_at: soon_timestamp,
            scopes: vec![],
        };
        assert!(!token.is_valid());
    }

    #[test]
    fn test_stored_token_is_valid_just_outside_buffer() {
        // Token that expires in 120 seconds should be valid
        let timestamp = chrono::Utc::now().timestamp() + 120;
        let token = StoredToken {
            access_token: "token".to_string(),
            refresh_token: None,
            expires_at: timestamp,
            scopes: vec![],
        };
        assert!(token.is_valid());
    }

    // ==================== ProfileConfig Tests ====================

    #[test]
    fn test_profile_config_default() {
        let config = ProfileConfig::default();
        assert!(config.client_id.is_none());
        assert!(config.client_secret.is_none());
        assert!(config.base_url.is_none());
        assert!(config.callback_url.is_none());
        assert!(config.da_nickname.is_none());
        // Note: Default derive gives bool::default() = false for use_keychain
        // The serde default only applies during deserialization
        assert!(!config.use_keychain);
    }

    #[test]
    fn test_profile_config_serialization() {
        let config = ProfileConfig {
            client_id: Some("my_client_id".to_string()),
            client_secret: Some("my_secret".to_string()),
            base_url: Some("https://custom.api.com".to_string()),
            callback_url: Some("http://localhost:3000/callback".to_string()),
            da_nickname: Some("my-nickname".to_string()),
            use_keychain: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("my_client_id"));
        assert!(json.contains("my_secret"));
        assert!(json.contains("https://custom.api.com"));
        assert!(json.contains("my-nickname"));
    }

    #[test]
    fn test_profile_config_deserialization() {
        let json = r#"{
            "client_id": "test_id",
            "client_secret": "test_secret",
            "base_url": "https://api.example.com",
            "use_keychain": false
        }"#;

        let config: ProfileConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.client_id, Some("test_id".to_string()));
        assert_eq!(config.client_secret, Some("test_secret".to_string()));
        assert_eq!(config.base_url, Some("https://api.example.com".to_string()));
        assert!(!config.use_keychain);
    }

    #[test]
    fn test_profile_config_deserialization_defaults() {
        // Missing use_keychain should default to true
        let json = r#"{"client_id": "test"}"#;
        let config: ProfileConfig = serde_json::from_str(json).unwrap();
        assert!(config.use_keychain);
    }

    // ==================== ProfilesData Tests ====================

    #[test]
    fn test_profiles_data_default() {
        let data = ProfilesData::default();
        assert!(data.active_profile.is_none());
        assert!(data.profiles.is_empty());
    }

    #[test]
    fn test_profiles_data_serialization() {
        let mut profiles = std::collections::HashMap::new();
        profiles.insert(
            "default".to_string(),
            ProfileConfig {
                client_id: Some("id1".to_string()),
                ..ProfileConfig::default()
            },
        );
        profiles.insert(
            "production".to_string(),
            ProfileConfig {
                client_id: Some("id2".to_string()),
                ..ProfileConfig::default()
            },
        );

        let data = ProfilesData {
            active_profile: Some("default".to_string()),
            profiles,
        };

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("default"));
        assert!(json.contains("production"));
        assert!(json.contains("id1"));
        assert!(json.contains("id2"));
    }

    #[test]
    fn test_profiles_data_deserialization() {
        let json = r#"{
            "active_profile": "dev",
            "profiles": {
                "dev": {"client_id": "dev_id"},
                "prod": {"client_id": "prod_id"}
            }
        }"#;

        let data: ProfilesData = serde_json::from_str(json).unwrap();
        assert_eq!(data.active_profile, Some("dev".to_string()));
        assert_eq!(data.profiles.len(), 2);
        assert_eq!(
            data.profiles.get("dev").unwrap().client_id,
            Some("dev_id".to_string())
        );
    }

    #[test]
    fn test_profiles_data_empty_profiles() {
        let json = r#"{"active_profile": null}"#;
        let data: ProfilesData = serde_json::from_str(json).unwrap();
        assert!(data.active_profile.is_none());
        assert!(data.profiles.is_empty());
    }

    #[test]
    fn test_stored_token_roundtrip() {
        let original = StoredToken {
            access_token: "access123".to_string(),
            refresh_token: Some("refresh456".to_string()),
            expires_at: 1700000000,
            scopes: vec!["read".to_string(), "write".to_string()],
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: StoredToken = serde_json::from_str(&json).unwrap();

        assert_eq!(original.access_token, deserialized.access_token);
        assert_eq!(original.refresh_token, deserialized.refresh_token);
        assert_eq!(original.expires_at, deserialized.expires_at);
        assert_eq!(original.scopes, deserialized.scopes);
    }

    #[test]
    fn test_profile_config_roundtrip() {
        let original = ProfileConfig {
            client_id: Some("client".to_string()),
            client_secret: Some("secret".to_string()),
            base_url: Some("https://api.com".to_string()),
            callback_url: Some("http://localhost/callback".to_string()),
            da_nickname: Some("nickname".to_string()),
            use_keychain: false,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ProfileConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(original.client_id, deserialized.client_id);
        assert_eq!(original.client_secret, deserialized.client_secret);
        assert_eq!(original.base_url, deserialized.base_url);
        assert_eq!(original.callback_url, deserialized.callback_url);
        assert_eq!(original.da_nickname, deserialized.da_nickname);
        assert_eq!(original.use_keychain, deserialized.use_keychain);
    }
}

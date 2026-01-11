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

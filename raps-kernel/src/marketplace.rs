// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Marketplace types for RAPS Pro plugin distribution.

use serde::{Deserialize, Serialize};

/// Tier classification for RAPS plugins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginTier {
    Free,
    Pro,
}

/// A plugin listed in the marketplace catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub price_monthly_cents: u32,
    pub price_yearly_cents: u32,
    pub latest_version: Option<String>,
    #[serde(default)]
    pub published: bool,
}

/// Response from `POST /license/validate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResponse {
    /// ISO 8601 datetime string — cache until this point.
    pub valid_until: String,
    /// Slugs of plugins the license grants access to.
    pub plugins: Vec<String>,
}

/// A locally installed marketplace plugin entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installation {
    pub slug: String,
    pub version: String,
    pub platform: String,
    /// Hex-encoded SHA-256 of the installed binary.
    pub sha256: String,
    /// Hex-encoded Ed25519 signature.
    pub signature: String,
    /// Absolute path to the installed binary.
    pub install_path: String,
}

/// Metadata returned in download response headers.
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub version: String,
    pub sha256: String,
    pub signature: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_tier_serde_roundtrip() {
        let tier = PluginTier::Pro;
        let json = serde_json::to_string(&tier).unwrap();
        assert_eq!(json, "\"pro\"");
        let back: PluginTier = serde_json::from_str(&json).unwrap();
        assert_eq!(back, PluginTier::Pro);
    }

    #[test]
    fn validate_response_deserializes() {
        let json = r#"{"valid_until":"2026-03-15T00:00:00Z","plugins":["acc-bulk"]}"#;
        let resp: ValidateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.plugins, vec!["acc-bulk"]);
        assert!(resp.valid_until.contains("2026"));
    }
}

// SPDX-License-Identifier: Commercial
// Copyright 2024-2025 Dmytro Yemelianov

//! License validation and enforcement
//!
//! Validates Pro licenses and enforces feature gating.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Pro license information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    /// License key
    pub key: String,
    /// Organization name
    pub organization: String,
    /// Expiration date (ISO 8601)
    pub expires_at: String,
    /// Number of seats licensed
    pub seats: u32,
    /// Enabled features
    pub features: Vec<String>,
}

impl License {
    /// Check if license is expired
    pub fn is_expired(&self) -> bool {
        if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&self.expires_at) {
            expires < chrono::Utc::now()
        } else {
            true // Invalid date = expired
        }
    }

    /// Check if a specific feature is enabled
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature || f == "*")
    }
}

/// License validation errors
#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    /// No license file found
    #[error("No Pro license found. Contact sales@rapscli.xyz for licensing.")]
    NotFound,

    /// License file is invalid
    #[error("Invalid license format: {0}")]
    InvalidFormat(String),

    /// License has expired
    #[error("License expired on {0}. Contact sales@rapscli.xyz to renew.")]
    Expired(String),

    /// License validation failed
    #[error("License validation failed: {0}")]
    ValidationFailed(String),

    /// Feature not included in license
    #[error("Feature '{0}' is not included in your license. Contact sales@rapscli.xyz to upgrade.")]
    FeatureNotLicensed(String),
}

/// License validator
pub struct LicenseValidator {
    license_path: Option<PathBuf>,
}

impl LicenseValidator {
    /// Create a new license validator
    pub fn new() -> Self {
        Self { license_path: None }
    }

    /// Create with a specific license file path
    pub fn with_path(path: PathBuf) -> Self {
        Self { license_path: Some(path) }
    }

    /// Get the default license file path
    fn default_license_path() -> Option<PathBuf> {
        // Check environment variable first
        if let Ok(path) = std::env::var("RAPS_LICENSE_FILE") {
            return Some(PathBuf::from(path));
        }

        // Check home directory
        if let Some(home) = home_dir() {
            let license_path = home.join(".raps").join("license.json");
            if license_path.exists() {
                return Some(license_path);
            }
        }

        // Check config directory
        if let Some(config) = config_dir() {
            let license_path = config.join("raps").join("license.json");
            if license_path.exists() {
                return Some(license_path);
            }
        }

        None
    }

    /// Validate the license
    pub fn validate(&self) -> Result<License, LicenseError> {
        let path = self.license_path
            .clone()
            .or_else(Self::default_license_path)
            .ok_or(LicenseError::NotFound)?;

        let content = std::fs::read_to_string(&path)
            .map_err(|_| LicenseError::NotFound)?;

        let license: License = serde_json::from_str(&content)
            .map_err(|e| LicenseError::InvalidFormat(e.to_string()))?;

        // Check expiration
        if license.is_expired() {
            return Err(LicenseError::Expired(license.expires_at.clone()));
        }

        // TODO: Add cryptographic signature validation
        // For now, just check the basic structure

        Ok(license)
    }

    /// Validate and check for a specific feature
    pub fn validate_feature(&self, feature: &str) -> Result<License, LicenseError> {
        let license = self.validate()?;
        
        if !license.has_feature(feature) {
            return Err(LicenseError::FeatureNotLicensed(feature.to_string()));
        }

        Ok(license)
    }
}

impl Default for LicenseValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for directory paths
fn home_dir() -> Option<std::path::PathBuf> {
    directories::BaseDirs::new().map(|bd| bd.home_dir().to_path_buf())
}

fn config_dir() -> Option<std::path::PathBuf> {
    directories::BaseDirs::new().map(|bd| bd.config_dir().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_not_found() {
        let validator = LicenseValidator::with_path(PathBuf::from("/nonexistent/license.json"));
        let result = validator.validate();
        assert!(matches!(result, Err(LicenseError::NotFound)));
    }

    #[test]
    fn test_license_feature_check() {
        let license = License {
            key: "test-key".to_string(),
            organization: "Test Org".to_string(),
            expires_at: "2099-12-31T23:59:59Z".to_string(),
            seats: 10,
            features: vec!["analytics".to_string(), "audit".to_string()],
        };

        assert!(license.has_feature("analytics"));
        assert!(license.has_feature("audit"));
        assert!(!license.has_feature("compliance"));
    }

    #[test]
    fn test_license_wildcard_feature() {
        let license = License {
            key: "enterprise-key".to_string(),
            organization: "Enterprise Org".to_string(),
            expires_at: "2099-12-31T23:59:59Z".to_string(),
            seats: 100,
            features: vec!["*".to_string()],
        };

        assert!(license.has_feature("analytics"));
        assert!(license.has_feature("anything"));
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! APS API endpoint URLs configuration

use serde::{Deserialize, Serialize};
use std::env;

/// APS API endpoint URLs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApsEndpoints {
    /// Authentication API endpoint
    pub auth: String,
    /// Object Storage Service (OSS) API endpoint
    pub oss: String,
    /// Model Derivative API endpoint
    pub derivative: String,
    /// Data Management API endpoint
    pub data_management: String,
    /// Webhooks API endpoint
    pub webhooks: String,
    /// Design Automation API endpoint
    pub design_automation: String,
    /// Construction Issues API endpoint
    pub issues: String,
    /// RFI (Request for Information) API endpoint
    pub rfi: String,
    /// Reality Capture API endpoint
    pub reality_capture: String,
}

impl Default for ApsEndpoints {
    fn default() -> Self {
        Self {
            auth: "https://developer.api.autodesk.com/authentication/v2".into(),
            oss: "https://developer.api.autodesk.com/oss/v2".into(),
            derivative: "https://developer.api.autodesk.com/modelderivative/v2".into(),
            data_management: "https://developer.api.autodesk.com/data/v1".into(),
            webhooks: "https://developer.api.autodesk.com/webhooks/v1".into(),
            design_automation: "https://developer.api.autodesk.com/da/us-east/v3".into(),
            issues: "https://developer.api.autodesk.com/construction/issues/v1".into(),
            rfi: "https://developer.api.autodesk.com/construction/rfis/v1".into(),
            reality_capture: "https://developer.api.autodesk.com/photo-to-3d/v1".into(),
        }
    }
}

impl ApsEndpoints {
    /// Load endpoints from environment variables (with defaults)
    pub fn from_env() -> Self {
        Self {
            auth: env::var("APS_AUTH_URL")
                .unwrap_or_else(|_| "https://developer.api.autodesk.com/authentication/v2".into()),
            oss: env::var("APS_OSS_URL")
                .unwrap_or_else(|_| "https://developer.api.autodesk.com/oss/v2".into()),
            derivative: env::var("APS_DERIVATIVE_URL").unwrap_or_else(|_| {
                "https://developer.api.autodesk.com/modelderivative/v2".into()
            }),
            data_management: env::var("APS_DM_URL").unwrap_or_else(|_| {
                "https://developer.api.autodesk.com/data/v1".into()
            }),
            webhooks: env::var("APS_WEBHOOKS_URL").unwrap_or_else(|_| {
                "https://developer.api.autodesk.com/webhooks/v1".into()
            }),
            design_automation: env::var("APS_DA_URL").unwrap_or_else(|_| {
                "https://developer.api.autodesk.com/da/us-east/v3".into()
            }),
            issues: env::var("APS_ISSUES_URL").unwrap_or_else(|_| {
                "https://developer.api.autodesk.com/construction/issues/v1".into()
            }),
            rfi: env::var("APS_RFI_URL").unwrap_or_else(|_| {
                "https://developer.api.autodesk.com/construction/rfis/v1".into()
            }),
            reality_capture: env::var("APS_REALITY_URL").unwrap_or_else(|_| {
                "https://developer.api.autodesk.com/photo-to-3d/v1".into()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aps_endpoints_default() {
        let endpoints = ApsEndpoints::default();
        assert!(endpoints.auth.contains("authentication"));
        assert!(endpoints.oss.contains("oss"));
        assert!(endpoints.derivative.contains("modelderivative"));
        assert!(endpoints.data_management.contains("data"));
        assert!(endpoints.webhooks.contains("webhooks"));
        assert!(endpoints.design_automation.contains("da"));
        assert!(endpoints.issues.contains("issues"));
        assert!(endpoints.rfi.contains("rfis"));
        assert!(endpoints.reality_capture.contains("photo-to-3d"));
    }

    #[test]
    fn test_aps_endpoints_from_env_defaults() {
        // Clear any existing env vars for this test
        let endpoints = ApsEndpoints::from_env();
        assert!(!endpoints.auth.is_empty());
        assert!(!endpoints.oss.is_empty());
        assert!(!endpoints.derivative.is_empty());
    }

    #[test]
    fn test_aps_endpoints_serialize() {
        let endpoints = ApsEndpoints::default();
        let json = serde_json::to_string(&endpoints);
        assert!(json.is_ok());
    }

    #[test]
    fn test_aps_endpoints_deserialize() {
        let json = r#"{
            "auth": "https://test.com/auth",
            "oss": "https://test.com/oss",
            "derivative": "https://test.com/derivative",
            "data_management": "https://test.com/dm",
            "webhooks": "https://test.com/webhooks",
            "design_automation": "https://test.com/da",
            "issues": "https://test.com/issues",
            "rfi": "https://test.com/rfi",
            "reality_capture": "https://test.com/reality"
        }"#;
        let endpoints: std::result::Result<ApsEndpoints, _> = serde_json::from_str(json);
        assert!(endpoints.is_ok());
        let endpoints = endpoints.unwrap();
        assert_eq!(endpoints.auth, "https://test.com/auth");
    }
}

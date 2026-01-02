// SPDX-License-Identifier: Commercial
// Copyright 2024-2025 Dmytro Yemelianov

//! RAPS Pro Tier - Enterprise features
//!
//! This crate provides Pro tier features on top of Community:
//!
//! ## Modules
//!
//! - [`license`] - License validation and enforcement
//! - [`analytics`] - Usage analytics and metrics
//! - [`audit`] - Audit logging for compliance
//! - [`compliance`] - Compliance reporting (SOC2, GDPR)
//! - [`multitenant`] - Multi-tenant project management
//! - [`sso`] - SSO/SAML integration
//!
//! ## License Required
//!
//! This crate requires a valid Pro license. Contact sales@rapscli.xyz for licensing.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod analytics;
pub mod audit;
pub mod compliance;
pub mod license;
pub mod multitenant;
pub mod sso;

// Re-exports
pub use analytics::AnalyticsClient;
pub use audit::AuditLog;
pub use compliance::ComplianceReporter;
pub use license::{License, LicenseError, LicenseValidator};
pub use multitenant::TenantManager;
pub use sso::SsoClient;

/// Check if a valid Pro license is available
pub fn is_licensed() -> bool {
    license::LicenseValidator::new()
        .validate()
        .is_ok()
}

/// Require a valid Pro license or return an error
pub fn require_license() -> Result<License, LicenseError> {
    license::LicenseValidator::new().validate()
}

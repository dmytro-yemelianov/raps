// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! RAPS Enterprise features
//!
//! This crate provides enterprise features:
//!
//! ## Modules
//!
//! - [`analytics`] - Usage analytics and metrics
//! - [`audit`] - Audit logging for compliance
//! - [`compliance`] - Compliance reporting (SOC2, GDPR)
//! - [`multitenant`] - Multi-tenant project management
//! - [`sso`] - SSO/SAML integration
//!

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod analytics;
pub mod audit;
pub mod compliance;
pub mod multitenant;
pub mod sso;

// Re-exports
pub use analytics::AnalyticsClient;
pub use audit::AuditLog;
pub use compliance::ComplianceReporter;
pub use multitenant::TenantManager;
pub use sso::SsoClient;

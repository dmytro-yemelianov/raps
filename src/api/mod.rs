//! API modules for interacting with Autodesk Platform Services
//!
//! This module contains clients for:
//! - Authentication (OAuth 2.0, both 2-legged and 3-legged)
//! - Object Storage Service (OSS)
//! - Model Derivative
//! - Data Management (Hubs, Projects, Folders, Items)
//! - Webhooks
//! - Design Automation
//! - ACC/BIM 360 (Issues, RFIs, Assets, Submittals, Checklists)
//! - Reality Capture

pub mod acc;
pub mod auth;
pub mod data_management;
pub mod derivative;
pub mod design_automation;
pub mod issues;
pub mod oss;
pub mod reality_capture;
pub mod webhooks;

pub use acc::AccClient;
pub use auth::AuthClient;
pub use data_management::DataManagementClient;
pub use derivative::DerivativeClient;
pub use design_automation::DesignAutomationClient;
pub use issues::IssuesClient;
pub use oss::OssClient;
pub use reality_capture::RealityCaptureClient;
pub use webhooks::WebhooksClient;

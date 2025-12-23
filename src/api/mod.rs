//! API modules for interacting with Autodesk Platform Services
//! 
//! This module contains clients for:
//! - Authentication (OAuth 2.0, both 2-legged and 3-legged)
//! - Object Storage Service (OSS)
//! - Model Derivative
//! - Data Management (Hubs, Projects, Folders, Items)
//! - Webhooks
//! - Design Automation
//! - ACC/BIM 360 (Issues, RFIs)
//! - Reality Capture

pub mod auth;
pub mod oss;
pub mod derivative;
pub mod data_management;
pub mod webhooks;
pub mod design_automation;
pub mod issues;
pub mod reality_capture;

pub use auth::AuthClient;
pub use oss::OssClient;
pub use derivative::DerivativeClient;
pub use data_management::DataManagementClient;
pub use webhooks::WebhooksClient;
pub use design_automation::DesignAutomationClient;
pub use issues::IssuesClient;
pub use reality_capture::RealityCaptureClient;

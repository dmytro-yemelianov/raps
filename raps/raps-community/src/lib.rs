// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! RAPS Community Tier - Extended features
//!
//! This crate provides Community tier features on top of the Core tier:
//!
//! ## Modules
//!
//! - [`acc`] - Autodesk Construction Cloud modules (Issues, RFIs, Assets, Submittals, Checklists)
//! - [`da`] - Design Automation (Engines, Activities, Work Items)
//! - [`reality`] - Reality Capture (Photogrammetry)
//! - [`webhooks`] - Webhook subscriptions
//! - [`pipeline`] - YAML/JSON workflow automation
//! - [`plugin`] - Plugin system (external commands, hooks, aliases)
//!
//! ## Feature Flags
//!
//! All features are enabled by default. Individual features can be disabled:
//! ```toml
//! [dependencies]
//! raps-community = { version = "0.1", default-features = false, features = ["acc", "webhooks"] }
//! ```

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod acc;
pub mod da;
pub mod pipeline;
pub mod plugin;
pub mod reality;
pub mod webhooks;

// Re-exports for convenience
pub use acc::{AccClient, Asset, Checklist, Issue, Rfi, Submittal};
pub use da::DesignAutomationClient;
pub use pipeline::PipelineRunner;
pub use plugin::PluginManager;
pub use reality::RealityCaptureClient;
pub use webhooks::WebhooksClient;

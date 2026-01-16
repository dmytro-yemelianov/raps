// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! CLI command modules
//!
//! Contains subcommand implementations for all APS operations.

pub mod acc;
pub mod admin;
pub mod auth;
pub mod bucket;
pub mod config;
pub mod da;
pub mod demo;
pub mod folder;
pub mod generate;
pub mod hub;
pub mod issue;
pub mod item;
pub mod object;
pub mod pipeline;
pub mod plugin;
pub mod project;
pub mod reality;
pub mod rfi;
pub mod translate;
pub mod webhook;

pub use acc::AccCommands;
pub use admin::AdminCommands;
pub use auth::AuthCommands;
pub use bucket::BucketCommands;
pub use config::ConfigCommands;
pub use da::DaCommands;
pub use demo::DemoCommands;
pub use folder::FolderCommands;
pub use generate::GenerateArgs;
pub use hub::HubCommands;
pub use issue::IssueCommands;
pub use item::ItemCommands;
pub use object::ObjectCommands;
pub use pipeline::PipelineCommands;
pub use plugin::PluginCommands;
pub use project::ProjectCommands;
pub use reality::RealityCommands;
pub use rfi::RfiCommands;
pub use translate::TranslateCommands;
pub use webhook::WebhookCommands;

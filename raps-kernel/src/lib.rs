// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

#![deny(warnings)]
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![cfg_attr(test, allow(unsafe_code))] // Allow unsafe in tests for env var manipulation

//! RAPS Kernel - Minimal trusted core for APS CLI operations.
//!
//! This crate provides the foundational infrastructure:
//! - Authentication (OAuth2 flows)
//! - HTTP client with retry/backoff
//! - Configuration management
//! - Secure credential storage
//! - Domain types and error handling
//!
//! The kernel is designed to be minimal (~2000 LOC), auditable, and highly testable.
//! All higher-level features depend on this core.

pub mod auth;
pub mod config;
pub mod error;
pub mod http;
pub mod storage;
pub use auth::AuthClient;
pub mod logging;
pub mod pipeline;
pub mod plugin;
pub mod types;

// Re-exports for convenience
pub use config::{ApsEndpoints, Config, Profile};
pub use error::{ExitCode, RapsError, Result};
pub use http::{HttpClient, HttpClientConfig, RetryConfig};
pub use pipeline::PipelineRunner;
pub use plugin::PluginManager;
pub use storage::{StorageBackend, TokenStorage};
pub use types::{BucketKey, ObjectKey, Urn};

//! RAPS Kernel - Core functionality for the RAPS CLI
//!
//! This crate provides the foundational components:
//! - Error handling and exit codes
//! - Logging and verbosity control
//! - HTTP client with retry logic
//! - Configuration management
//! - Token storage abstraction
//! - OAuth authentication
//! - Progress bar utilities
//! - Interactive prompt utilities

#![allow(clippy::uninlined_format_args)]

pub mod api_health;
pub mod audit;
pub mod auth;
pub mod cache;
pub mod cache_backend;
#[cfg(feature = "redis")]
pub mod job_queue;
#[cfg(feature = "redis")]
pub mod redis_backend;
pub mod checkpoint;
pub mod circuit_breaker;
pub mod config;
pub mod error;
pub mod http;
pub mod interactive;
pub mod logging;
pub mod metrics;
pub mod output;
pub mod profiler;
pub mod progress;
pub mod prompts;
pub mod rate_budget;
pub mod region;
pub mod response_cache;
pub mod serverless;
pub mod retry_policy;
pub mod security;
pub mod storage;
pub mod types;

#[cfg(feature = "prometheus")]
pub mod prometheus_metrics;

#[cfg(feature = "kubernetes")]
pub mod health_server;

/// Test utilities for mocking API responses
/// Only available when running tests
#[cfg(test)]
pub mod test_utils;

// Re-export commonly used types
pub use auth::AuthClient;
pub use config::{Config, ContextConfig};
pub use error::ExitCode;
pub use http::HttpClientConfig;
pub use output::OutputFormat;
pub use storage::{StorageBackend, TokenStorage};
pub use types::StoredToken;

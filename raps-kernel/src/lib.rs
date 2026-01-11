//! RAPS Kernel - Core functionality for the RAPS CLI
//!
//! This crate provides the foundational components:
//! - Error handling and exit codes
//! - Logging and verbosity control
//! - HTTP client with retry logic
//! - Configuration management
//! - Token storage abstraction
//! - OAuth authentication

#![allow(clippy::uninlined_format_args)]

pub mod auth;
pub mod config;
pub mod error;
pub mod http;
pub mod interactive;
pub mod logging;
pub mod output;
pub mod storage;
pub mod types;

// Re-export commonly used types
pub use auth::AuthClient;
pub use config::Config;
pub use error::ExitCode;
pub use http::HttpClientConfig;
pub use output::OutputFormat;
pub use storage::{StorageBackend, TokenStorage};
pub use types::StoredToken;

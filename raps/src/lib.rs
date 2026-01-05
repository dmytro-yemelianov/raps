// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! RAPS Library - Exposes internal modules for benchmarking and testing
//!
//! This lib.rs exists to allow benchmarks to access internal modules.
//! The main.rs binary uses the same modules.

// Declare modules (they're also declared in main.rs, but that's OK for lib/bin split)
#[path = "api/mod.rs"]
pub mod api;

#[path = "config.rs"]
pub mod config;

#[path = "http.rs"]
pub mod http;

#[path = "storage.rs"]
pub mod storage;

#[path = "error.rs"]
pub mod error;

#[path = "logging.rs"]
pub mod logging;

// Additional modules needed by commands/config
#[path = "output.rs"]
pub mod output;

#[path = "interactive.rs"]
pub mod interactive;

// Modules that output/interactive might need
#[path = "plugins.rs"]
pub mod plugins;

// Commands module needed by config.rs
#[path = "commands/mod.rs"]
pub mod commands;

// Re-exports for convenience
pub use api::AuthClient;
pub use config::Config;
pub use http::HttpClientConfig;

// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! RAPS CLI library
//!
//! This is the main CLI crate that ties together all service crates.
//! The CLI is primarily a binary, but this library module exports
//! components that may be useful for testing or programmatic usage.

pub mod commands;
pub mod mcp;
pub mod plugins;
pub mod shell;
pub mod output;

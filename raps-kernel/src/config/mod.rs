// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Configuration management for RAPS Kernel

pub mod endpoints;
pub mod profiles;

pub use endpoints::ApsEndpoints;
pub use profiles::{Config, Profile};

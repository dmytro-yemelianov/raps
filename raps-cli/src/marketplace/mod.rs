// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

pub mod auth;
pub mod client;
pub mod installer;
pub mod subscription;

pub use auth::MarketplaceAuth;
pub use client::MarketplaceClient;
pub use installer::{detect_platform, PluginInstaller};
pub use subscription::{CachedValidation, SubscriptionManager};

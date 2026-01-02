// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Authentication module for APS OAuth 2.0
//!
//! Provides OAuth 2.0 authentication flows:
//! - **Two-legged (Client Credentials)**: Server-to-server authentication
//! - **Three-legged (Authorization Code)**: User-authorized authentication
//! - **Device Code**: Authentication for devices without browser

pub mod client;
pub mod device_code;
pub mod three_legged;
pub mod two_legged;
pub mod types;

// Re-export main client
pub use client::AuthClient;

// Re-export types
pub use types::{DeviceCodeResponse, Scopes, StoredToken, TokenResponse, UserInfo};

// Re-export flow implementations
pub use device_code::{DeviceCodeAuth, DeviceCodeError};
pub use three_legged::ThreeLeggedAuth;
pub use two_legged::TwoLeggedAuth;

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Authentication module for APS OAuth 2.0

pub mod client;

pub use client::{
    AuthClient, DeviceCodeResponse, StoredToken, TokenResponse, UserInfo,
};

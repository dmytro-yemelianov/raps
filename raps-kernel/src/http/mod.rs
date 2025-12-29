// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! HTTP client with retry logic

pub mod client;
pub mod retry;

pub use client::{HttpClient, HttpClientConfig};
pub use retry::RetryConfig;

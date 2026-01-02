// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! HTTP client with retry logic and middleware

pub mod client;
pub mod middleware;
pub mod retry;

pub use client::{HttpClient, HttpClientConfig};
pub use middleware::{
    apply_modifiers, log_request, log_response, ApsHeaders, BearerAuth, RegionHeader, RequestId,
    RequestModifier,
};
pub use retry::RetryConfig;

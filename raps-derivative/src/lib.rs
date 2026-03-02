// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! Model Derivative API module
//!
//! Handles translation of CAD files and retrieval of derivative manifests.
//! Supports downloading translated derivatives directly from manifest.

mod download;
mod metadata;
mod translations;
pub mod types;

pub use types::*;

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

/// Model Derivative API client
#[derive(Clone)]
pub struct DerivativeClient {
    pub(crate) config: Config,
    pub(crate) auth: AuthClient,
    pub(crate) http_client: reqwest::Client,
}

impl DerivativeClient {
    /// Create a new Model Derivative client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new Model Derivative client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
    ) -> Self {
        // Create HTTP client with configured timeouts
        let http_client = http_config.create_client().unwrap_or_else(|e| {
            tracing::warn!("HTTP client configuration failed, using defaults: {e}");
            reqwest::Client::new()
        });

        Self {
            config,
            auth,
            http_client,
        }
    }
}

/// Integration tests using raps-mock
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;

    fn create_mock_client(mock_url: &str) -> DerivativeClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        DerivativeClient::new(config, auth)
    }

    #[tokio::test]
    async fn test_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }
}

// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! Design Automation API module
//!
//! Handles automation of CAD processing with engines like AutoCAD, Revit, Inventor, 3ds Max.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

mod activities;
mod appbundles;
mod engines;
pub mod types;
mod workitems;

pub use types::*;

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

/// Design Automation API client
#[derive(Clone)]
pub struct DesignAutomationClient {
    pub(crate) config: Config,
    pub(crate) auth: AuthClient,
    pub(crate) http_client: reqwest::Client,
}

impl DesignAutomationClient {
    /// Create a new Design Automation client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new Design Automation client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
    ) -> Self {
        // Create HTTP client with configured timeouts
        let http_client = http_config
            .create_client()
            .unwrap_or_else(|_| reqwest::Client::new()); // Fallback to default if config fails

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

    fn create_mock_da_client(mock_url: &str) -> DesignAutomationClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: Some("test-nickname".to_string()),
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        DesignAutomationClient::new(config, auth)
    }

    #[tokio::test]
    async fn test_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_da_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }

    #[tokio::test]
    async fn test_list_workitems() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_da_client(&server.url);
        let result = client.list_workitems().await;
        let _ = result;
    }
}

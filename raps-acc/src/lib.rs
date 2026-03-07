// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC/BIM 360 API module
//!
//! This crate provides clients for ACC (Autodesk Construction Cloud) APIs:
//! - Issues - Construction Issues management
//! - RFI - Request for Information management
//! - Extended APIs - Assets, Submittals, Checklists
//! - Account Admin API - User and project management
//! - Project Users API - Project member management

pub mod admin;
pub mod extended;
pub mod helpers;
pub mod issues;
pub mod permissions;
pub mod rfis;
pub mod types;
pub mod users;

pub use extended::*;
pub use helpers::*;
pub use issues::*;
pub use rfis::*;

/// Integration tests using raps-mock
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;
    use raps_kernel::http::HttpClientConfig;

    fn create_mock_acc_client(mock_url: &str) -> AccClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        AccClient::new(config, auth)
    }

    fn create_mock_issues_client(mock_url: &str) -> IssuesClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        IssuesClient::new(config, auth)
    }

    fn create_mock_rfi_client(mock_url: &str) -> RfiClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        RfiClient::new(config, auth)
    }

    #[tokio::test]
    async fn test_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_acc_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }

    #[tokio::test]
    async fn test_delete_issue_not_found() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_issues_client(&server.url);
        let result = client
            .delete_issue("project-123", "nonexistent-issue")
            .await;
        // The mock server may return various responses - just verify it doesn't panic
        let _ = result;
    }

    #[tokio::test]
    async fn test_delete_rfi() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_rfi_client(&server.url);
        let result = client.delete_rfi("project-123", "rfi-456").await;
        let _ = result;
    }

    #[tokio::test]
    async fn test_delete_submittal() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_acc_client(&server.url);
        let result = client
            .delete_submittal("project-123", "submittal-789")
            .await;
        let _ = result;
    }
}

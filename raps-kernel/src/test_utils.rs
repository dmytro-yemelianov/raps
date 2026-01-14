// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Test utilities for mocking API responses
//!
//! Provides infrastructure for testing API clients with mocked HTTP responses.
//! This module is only available when running tests.

use crate::auth::AuthClient;
use crate::config::Config;
use crate::http::HttpClientConfig;

/// Test configuration builder for creating test clients
pub struct TestConfig {
    /// Base URL (set to mock server URL)
    pub base_url: String,
    /// Client ID for testing
    pub client_id: String,
    /// Client secret for testing
    pub client_secret: String,
    /// Callback URL for OAuth
    pub callback_url: String,
    /// Design Automation nickname
    pub da_nickname: Option<String>,
}

impl TestConfig {
    /// Create a new test configuration with the given mock server URL
    pub fn new(mock_server_url: &str) -> Self {
        Self {
            base_url: mock_server_url.to_string(),
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: Some("test-nickname".to_string()),
        }
    }

    /// Convert to a Config struct
    pub fn to_config(&self) -> Config {
        Config {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            base_url: self.base_url.clone(),
            callback_url: self.callback_url.clone(),
            da_nickname: self.da_nickname.clone(),
            http_config: HttpClientConfig::default(),
        }
    }

    /// Create an AuthClient for testing
    pub fn create_auth_client(&self) -> AuthClient {
        AuthClient::new(self.to_config())
    }
}

/// Create a temporary directory for test files
pub fn create_temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Create a test file with random content
pub fn create_test_file(dir: &tempfile::TempDir, name: &str, size: usize) -> std::path::PathBuf {
    use std::io::Write;

    let path = dir.path().join(name);
    let mut file = std::fs::File::create(&path).expect("Failed to create test file");

    // Write random bytes
    let content: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
    file.write_all(&content).expect("Failed to write test file");

    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_config_creation() {
        let config = TestConfig::new("http://localhost:8080");
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.client_id, "test-client-id");
    }

    #[test]
    fn test_test_config_to_config() {
        let test_config = TestConfig::new("http://localhost:8080");
        let config = test_config.to_config();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.client_id, "test-client-id");
    }

    #[test]
    fn test_create_temp_dir() {
        let dir = create_temp_dir();
        assert!(dir.path().exists());
    }

    #[test]
    fn test_create_test_file() {
        let dir = create_temp_dir();
        let file_path = create_test_file(&dir, "test.bin", 1024);
        assert!(file_path.exists());
        assert_eq!(std::fs::metadata(&file_path).unwrap().len(), 1024);
    }

    #[tokio::test]
    async fn test_mock_server_basic() {
        use raps_mock::TestServer;

        let server = TestServer::start_default().await.unwrap();
        let config = TestConfig::new(&server.url);

        let client = reqwest::Client::new();
        // Test that server is running by making a request
        // The specific endpoint doesn't matter as raps-mock handles routing
        let response = client
            .get(format!("{}/health", config.base_url))
            .send()
            .await;

        // Server is running (response may be 404 if no /health route, but that's OK)
        assert!(response.is_ok());
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! HTTP client configuration and creation

use crate::error::{RapsError, Result};
use reqwest::Client;
use std::time::Duration;

/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Request timeout
    pub timeout: Duration,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Maximum retries for transient errors
    pub max_retries: u32,
    /// Base delay for exponential backoff
    pub retry_base_delay: Duration,
    /// Maximum delay between retries
    pub retry_max_delay: Duration,
    /// Whether to add jitter to retry delays
    pub retry_jitter: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            connect_timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_base_delay: Duration::from_secs(1),
            retry_max_delay: Duration::from_secs(30),
            retry_jitter: true,
        }
    }
}

impl HttpClientConfig {
    /// Create HTTP client with configured timeouts
    pub fn create_client(&self) -> Result<Client> {
        Client::builder()
            .timeout(self.timeout)
            .connect_timeout(self.connect_timeout)
            .build()
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to create HTTP client: {}", e),
            })
    }
}

/// HTTP client wrapper
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: Client,
    config: HttpClientConfig,
}

impl HttpClient {
    /// Create new HTTP client with config
    pub fn new(config: HttpClientConfig) -> Result<Self> {
        let client = config.create_client()?;
        Ok(Self { client, config })
    }

    /// Get the underlying reqwest client
    pub fn inner(&self) -> &Client {
        &self.client
    }

    /// Get the configuration
    pub fn config(&self) -> &HttpClientConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_client_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.timeout.as_secs(), 120);
        assert_eq!(config.connect_timeout.as_secs(), 30);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_base_delay.as_secs(), 1);
        assert_eq!(config.retry_max_delay.as_secs(), 30);
        assert!(config.retry_jitter);
    }

    #[test]
    fn test_http_client_config_create_client() {
        let config = HttpClientConfig::default();
        let client = config.create_client();
        assert!(client.is_ok());
    }

    #[test]
    fn test_http_client_new() {
        let config = HttpClientConfig::default();
        let http_client = HttpClient::new(config);
        assert!(http_client.is_ok());
    }

    #[test]
    fn test_http_client_inner() {
        let config = HttpClientConfig::default();
        let http_client = HttpClient::new(config).unwrap();
        let _inner = http_client.inner();
        // Just verify it doesn't panic
    }

    #[test]
    fn test_http_client_config_access() {
        let config = HttpClientConfig::default();
        let http_client = HttpClient::new(config.clone()).unwrap();
        let retrieved_config = http_client.config();
        assert_eq!(retrieved_config.timeout, config.timeout);
    }
}

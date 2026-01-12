// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! HTTP client utilities
//!
//! Provides retry logic, timeouts, and HTTP client configuration.

use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Maximum wait time between retries (seconds)
    pub max_wait: u64,
    /// Base delay for exponential backoff (seconds)
    pub base_delay: u64,
    /// Request timeout (seconds)
    pub timeout: u64,
    /// Connect timeout (seconds)
    pub connect_timeout: u64,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            max_wait: 60,
            base_delay: 1,
            timeout: 120,
            connect_timeout: 30,
        }
    }
}

impl HttpClientConfig {
    /// Create HTTP client with configured timeouts
    pub fn create_client(&self) -> Result<Client> {
        Client::builder()
            .timeout(Duration::from_secs(self.timeout))
            .connect_timeout(Duration::from_secs(self.connect_timeout))
            .build()
            .context("Failed to create HTTP client")
    }

    /// Create HTTP client config from CLI flags and environment variables
    /// Precedence: CLI flag > environment variable > default
    pub fn from_cli_and_env(timeout_flag: Option<u64>) -> Self {
        let timeout = timeout_flag
            .or_else(|| {
                std::env::var("RAPS_TIMEOUT")
                    .ok()
                    .and_then(|v| v.parse().ok())
            })
            .unwrap_or(120);

        Self {
            timeout,
            ..Self::default()
        }
    }
}

/// Execute HTTP request with retry logic
pub async fn execute_with_retry<F, T>(config: &HttpClientConfig, mut request_fn: F) -> Result<T>
where
    F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
{
    let mut attempt = 0;

    loop {
        match request_fn().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                // Check if we should retry
                let should_retry = should_retry_error(&err, attempt, config.max_retries);

                if !should_retry {
                    return Err(err);
                }

                attempt += 1;

                // Calculate delay with exponential backoff and jitter
                let delay = calculate_delay(attempt, config.base_delay, config.max_wait);

                // Log retry attempt
                crate::logging::log_verbose(&format!(
                    "Request failed (attempt {}/{}), retrying in {}s...",
                    attempt,
                    config.max_retries,
                    delay.as_secs()
                ));

                sleep(delay).await;
            }
        }
    }
}

/// Determine if an error should be retried
fn should_retry_error(err: &anyhow::Error, attempt: u32, max_retries: u32) -> bool {
    if attempt >= max_retries {
        return false;
    }

    // Check if it's a reqwest error with status code
    if let Some(reqwest_err) = err.downcast_ref::<reqwest::Error>() {
        if reqwest_err.is_status()
            && let Some(status) = reqwest_err.status()
        {
            // Retry on rate limiting (429)
            if status.as_u16() == 429 {
                return true;
            }

            // Retry on server errors (5xx)
            if status.is_server_error() {
                return true;
            }

            // Don't retry on client errors (4xx except 429)
            if status.is_client_error() {
                return false;
            }
        }

        // Retry on network/timeout errors
        if reqwest_err.is_timeout() || reqwest_err.is_connect() || reqwest_err.is_request() {
            return true;
        }
    }

    // Check error string for common patterns
    let error_str = err.to_string().to_lowercase();

    // Retry on rate limiting (429)
    if error_str.contains("429") || error_str.contains("too many requests") {
        return true;
    }

    // Retry on server errors (5xx)
    if error_str.contains("500")
        || error_str.contains("502")
        || error_str.contains("503")
        || error_str.contains("504")
        || error_str.contains("server error")
    {
        return true;
    }

    // Retry on network/timeout errors
    if error_str.contains("timeout")
        || error_str.contains("connection")
        || error_str.contains("network")
    {
        return true;
    }

    // Default: don't retry unknown errors
    false
}

/// Calculate delay with exponential backoff and jitter
fn calculate_delay(attempt: u32, base_delay: u64, max_wait: u64) -> Duration {
    use rand::Rng;

    // Exponential backoff: base_delay * 2^attempt
    let exponential_delay = base_delay * 2_u64.pow(attempt);

    // Cap at max_wait
    let capped_delay = exponential_delay.min(max_wait);

    // Add jitter (random 0-25% of delay)
    let mut rng = rand::thread_rng();
    let jitter = rng.gen_range(0..=(capped_delay / 4));

    Duration::from_secs(capped_delay + jitter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.max_wait, 60);
        assert_eq!(config.base_delay, 1);
        assert_eq!(config.timeout, 120);
        assert_eq!(config.connect_timeout, 30);
    }

    #[test]
    fn test_http_config_create_client() {
        let config = HttpClientConfig::default();
        let client = config.create_client();
        assert!(client.is_ok());
    }

    #[test]
    fn test_http_config_from_cli_flag() {
        let config = HttpClientConfig::from_cli_and_env(Some(60));
        assert_eq!(config.timeout, 60);
        // Other values should be default
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_http_config_from_env() {
        // SAFETY: Test runs with --test-threads=1 or in isolation
        unsafe {
            std::env::set_var("RAPS_TIMEOUT", "90");
        }
        let config = HttpClientConfig::from_cli_and_env(None);
        assert_eq!(config.timeout, 90);
        unsafe {
            std::env::remove_var("RAPS_TIMEOUT");
        }
    }

    #[test]
    fn test_http_config_cli_overrides_env() {
        // SAFETY: Test runs with --test-threads=1 or in isolation
        unsafe {
            std::env::set_var("RAPS_TIMEOUT", "90");
        }
        let config = HttpClientConfig::from_cli_and_env(Some(45));
        assert_eq!(config.timeout, 45);
        unsafe {
            std::env::remove_var("RAPS_TIMEOUT");
        }
    }

    #[test]
    fn test_http_config_invalid_env() {
        // SAFETY: Test runs with --test-threads=1 or in isolation
        unsafe {
            std::env::set_var("RAPS_TIMEOUT", "not_a_number");
        }
        let config = HttpClientConfig::from_cli_and_env(None);
        assert_eq!(config.timeout, 120); // Falls back to default
        unsafe {
            std::env::remove_var("RAPS_TIMEOUT");
        }
    }

    #[test]
    fn test_should_retry_429() {
        let err = anyhow::anyhow!("Request failed with 429 Too Many Requests");
        assert!(should_retry_error(&err, 0, 3));
    }

    #[test]
    fn test_should_retry_500() {
        let err = anyhow::anyhow!("Server error: 500 Internal Server Error");
        assert!(should_retry_error(&err, 0, 3));
    }

    #[test]
    fn test_should_retry_502() {
        let err = anyhow::anyhow!("502 Bad Gateway");
        assert!(should_retry_error(&err, 0, 3));
    }

    #[test]
    fn test_should_retry_503() {
        let err = anyhow::anyhow!("503 Service Unavailable");
        assert!(should_retry_error(&err, 0, 3));
    }

    #[test]
    fn test_should_retry_504() {
        let err = anyhow::anyhow!("504 Gateway Timeout");
        assert!(should_retry_error(&err, 0, 3));
    }

    #[test]
    fn test_should_retry_timeout() {
        let err = anyhow::anyhow!("Request timeout after 30s");
        assert!(should_retry_error(&err, 0, 3));
    }

    #[test]
    fn test_should_retry_connection() {
        let err = anyhow::anyhow!("Connection refused");
        assert!(should_retry_error(&err, 0, 3));
    }

    #[test]
    fn test_should_retry_network() {
        let err = anyhow::anyhow!("Network error occurred");
        assert!(should_retry_error(&err, 0, 3));
    }

    #[test]
    fn test_should_not_retry_400() {
        let err = anyhow::anyhow!("Bad request: 400");
        assert!(!should_retry_error(&err, 0, 3));
    }

    #[test]
    fn test_should_not_retry_401() {
        let err = anyhow::anyhow!("Unauthorized: 401");
        assert!(!should_retry_error(&err, 0, 3));
    }

    #[test]
    fn test_should_not_retry_403() {
        let err = anyhow::anyhow!("Forbidden: 403");
        assert!(!should_retry_error(&err, 0, 3));
    }

    #[test]
    fn test_should_not_retry_404() {
        let err = anyhow::anyhow!("Not found: 404");
        assert!(!should_retry_error(&err, 0, 3));
    }

    #[test]
    fn test_should_not_retry_max_attempts() {
        let err = anyhow::anyhow!("500 Server Error");
        assert!(!should_retry_error(&err, 3, 3)); // At max retries
    }

    #[test]
    fn test_calculate_delay_exponential() {
        // First retry: base_delay * 2^1 = 1 * 2 = 2 seconds
        let delay1 = calculate_delay(1, 1, 60);
        assert!(delay1.as_secs() >= 2);
        assert!(delay1.as_secs() <= 3); // 2 + up to 25% jitter

        // Second retry: base_delay * 2^2 = 1 * 4 = 4 seconds
        let delay2 = calculate_delay(2, 1, 60);
        assert!(delay2.as_secs() >= 4);
        assert!(delay2.as_secs() <= 5);
    }

    #[test]
    fn test_calculate_delay_max_wait() {
        // Very high attempt should be capped at max_wait
        let delay = calculate_delay(10, 1, 60);
        assert!(delay.as_secs() <= 75); // 60 + up to 25% jitter
    }

    #[test]
    fn test_calculate_delay_custom_base() {
        // With base_delay of 2: 2 * 2^1 = 4 seconds
        let delay = calculate_delay(1, 2, 60);
        assert!(delay.as_secs() >= 4);
        assert!(delay.as_secs() <= 5);
    }
}

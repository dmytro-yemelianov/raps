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

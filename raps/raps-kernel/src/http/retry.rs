// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Retry configuration for HTTP operations

use std::time::Duration;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Base delay for exponential backoff
    pub base_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Whether to add jitter to retry delays
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            jitter: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay.as_secs(), 1);
        assert_eq!(config.max_delay.as_secs(), 30);
        assert!(config.jitter);
    }

    #[test]
    fn test_retry_config_custom() {
        let config = RetryConfig {
            max_retries: 5,
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(60),
            jitter: false,
        };
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.base_delay.as_secs(), 2);
        assert_eq!(config.max_delay.as_secs(), 60);
        assert!(!config.jitter);
    }
}

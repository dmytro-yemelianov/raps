// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Retry logic with exponential backoff

use std::time::Duration;

/// Calculate delay for exponential backoff
///
/// # Arguments
/// * `attempt` - Current attempt number (0-based)
/// * `base_delay` - Base delay duration
/// * `max_delay` - Maximum delay cap
///
/// # Returns
/// The delay to wait before the next retry
pub fn exponential_backoff(attempt: u32, base_delay: Duration, max_delay: Duration) -> Duration {
    let delay = base_delay.saturating_mul(2u32.saturating_pow(attempt));
    std::cmp::min(delay, max_delay)
}

/// Determine if an error is retryable based on HTTP status code
pub fn is_retryable_status(status: u16) -> bool {
    matches!(
        status,
        408 |  // Request Timeout
        429 |  // Too Many Requests (rate limit)
        500 |  // Internal Server Error
        502 |  // Bad Gateway
        503 |  // Service Unavailable
        504 // Gateway Timeout
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let base = Duration::from_secs(1);
        let max = Duration::from_secs(60);

        assert_eq!(exponential_backoff(0, base, max), Duration::from_secs(1));
        assert_eq!(exponential_backoff(1, base, max), Duration::from_secs(2));
        assert_eq!(exponential_backoff(2, base, max), Duration::from_secs(4));
        assert_eq!(exponential_backoff(3, base, max), Duration::from_secs(8));
        assert_eq!(exponential_backoff(10, base, max), Duration::from_secs(60)); // Capped at max
    }

    #[test]
    fn test_is_retryable_status() {
        assert!(is_retryable_status(429));
        assert!(is_retryable_status(500));
        assert!(is_retryable_status(503));
        assert!(!is_retryable_status(400));
        assert!(!is_retryable_status(401));
        assert!(!is_retryable_status(404));
    }
}

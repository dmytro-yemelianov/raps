// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Failure-type-aware retry policies.
//!
//! Classifies HTTP failures and selects the optimal recovery strategy
//! for each type, replacing one-size-fits-all exponential backoff.

use std::time::Duration;

// ---------------------------------------------------------------------------
// Failure classification
// ---------------------------------------------------------------------------

/// Classified failure type, determined from HTTP response or error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureType {
    /// HTTP 429 Too Many Requests
    RateLimited,
    /// HTTP 401 Unauthorized
    Unauthorized,
    /// HTTP 500 Internal Server Error
    ServerError,
    /// HTTP 502 Bad Gateway
    BadGateway,
    /// HTTP 503 Service Unavailable
    ServiceUnavailable,
    /// HTTP 504 Gateway Timeout
    GatewayTimeout,
    /// HTTP 408 Request Timeout
    RequestTimeout,
    /// HTTP 409 Conflict (e.g. translation already in progress)
    Conflict,
    /// Network-level error (timeout, connection refused, DNS failure)
    NetworkError,
    /// Translation worker internal failure (from manifest message)
    TranslationInternalFailure,
    /// Translation worker download failure (from manifest message)
    TranslationDownloadFailure,
    /// Region mismatch (wrong endpoint for resource region)
    RegionMismatch,
    /// Unknown retryable failure
    Unknown,
}

impl FailureType {
    /// Classify from an HTTP status code.
    pub fn from_status(status: u16) -> Option<Self> {
        match status {
            401 => Some(FailureType::Unauthorized),
            408 => Some(FailureType::RequestTimeout),
            409 => Some(FailureType::Conflict),
            429 => Some(FailureType::RateLimited),
            500 => Some(FailureType::ServerError),
            502 => Some(FailureType::BadGateway),
            503 => Some(FailureType::ServiceUnavailable),
            504 => Some(FailureType::GatewayTimeout),
            _ => None,
        }
    }

    /// Classify from an error message (e.g. translation manifest).
    pub fn from_message(msg: &str) -> Option<Self> {
        let lower = msg.to_lowercase();
        if lower.contains("translationworker-internalfailure") {
            Some(FailureType::TranslationInternalFailure)
        } else if lower.contains("translationworker-faileddownload") {
            Some(FailureType::TranslationDownloadFailure)
        } else if lower.contains("region") && lower.contains("mismatch") {
            Some(FailureType::RegionMismatch)
        } else {
            None
        }
    }

    /// Whether this failure type should trigger circuit breaker evaluation.
    pub fn triggers_circuit_breaker(self) -> bool {
        matches!(
            self,
            FailureType::ServerError
                | FailureType::BadGateway
                | FailureType::ServiceUnavailable
                | FailureType::GatewayTimeout
                | FailureType::NetworkError
        )
    }
}

impl std::fmt::Display for FailureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureType::RateLimited => write!(f, "rate-limited (429)"),
            FailureType::Unauthorized => write!(f, "unauthorized (401)"),
            FailureType::ServerError => write!(f, "server error (500)"),
            FailureType::BadGateway => write!(f, "bad gateway (502)"),
            FailureType::ServiceUnavailable => write!(f, "service unavailable (503)"),
            FailureType::GatewayTimeout => write!(f, "gateway timeout (504)"),
            FailureType::RequestTimeout => write!(f, "request timeout (408)"),
            FailureType::Conflict => write!(f, "conflict (409)"),
            FailureType::NetworkError => write!(f, "network error"),
            FailureType::TranslationInternalFailure => write!(f, "translation internal failure"),
            FailureType::TranslationDownloadFailure => write!(f, "translation download failure"),
            FailureType::RegionMismatch => write!(f, "region mismatch"),
            FailureType::Unknown => write!(f, "unknown"),
        }
    }
}

// ---------------------------------------------------------------------------
// Retry policy
// ---------------------------------------------------------------------------

/// Action to take before retrying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreRetryAction {
    /// Refresh the auth token before retrying.
    RefreshToken,
    /// Switch to alternate region and retry.
    SwitchRegion,
    /// Re-upload the source file before retrying translation.
    ReUpload,
    /// Check if the operation is already in progress (e.g. translation).
    CheckExisting,
}

/// Backoff strategy for retries.
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// Fixed delay between retries.
    Fixed(Duration),
    /// Exponential backoff: base * 2^attempt, capped at max.
    Exponential { base: Duration, max: Duration },
    /// Use the Retry-After header value; fall back to exponential.
    HeaderBased {
        fallback_base: Duration,
        max: Duration,
    },
}

/// Policy for retrying a specific failure type.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    pub pre_retry: Option<PreRetryAction>,
}

impl RetryPolicy {
    /// Compute delay for the given attempt (0-indexed).
    pub fn delay_for_attempt(
        &self,
        attempt: u32,
        retry_after_header: Option<Duration>,
    ) -> Duration {
        match &self.backoff {
            BackoffStrategy::Fixed(d) => *d,
            BackoffStrategy::Exponential { base, max } => {
                let delay = base.saturating_mul(2u32.saturating_pow(attempt));
                std::cmp::min(delay, *max)
            }
            BackoffStrategy::HeaderBased { fallback_base, max } => {
                if let Some(header_val) = retry_after_header {
                    header_val
                } else {
                    let delay = fallback_base.saturating_mul(2u32.saturating_pow(attempt));
                    std::cmp::min(delay, *max)
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Default policy table
// ---------------------------------------------------------------------------

/// Get the default retry policy for a failure type.
pub fn default_policy(failure: FailureType) -> RetryPolicy {
    match failure {
        FailureType::RateLimited => RetryPolicy {
            max_attempts: 5,
            backoff: BackoffStrategy::HeaderBased {
                fallback_base: Duration::from_secs(1),
                max: Duration::from_secs(60),
            },
            pre_retry: None,
        },
        FailureType::Unauthorized => RetryPolicy {
            max_attempts: 1,
            backoff: BackoffStrategy::Fixed(Duration::from_millis(100)),
            pre_retry: Some(PreRetryAction::RefreshToken),
        },
        FailureType::ServerError => RetryPolicy {
            max_attempts: 5,
            backoff: BackoffStrategy::Exponential {
                base: Duration::from_secs(1),
                max: Duration::from_secs(16),
            },
            pre_retry: None,
        },
        FailureType::BadGateway | FailureType::GatewayTimeout => RetryPolicy {
            max_attempts: 3,
            backoff: BackoffStrategy::Exponential {
                base: Duration::from_secs(2),
                max: Duration::from_secs(30),
            },
            pre_retry: None,
        },
        FailureType::ServiceUnavailable => RetryPolicy {
            max_attempts: 3,
            backoff: BackoffStrategy::Exponential {
                base: Duration::from_secs(5),
                max: Duration::from_secs(60),
            },
            pre_retry: None,
        },
        FailureType::RequestTimeout | FailureType::NetworkError => RetryPolicy {
            max_attempts: 3,
            backoff: BackoffStrategy::Exponential {
                base: Duration::from_secs(1),
                max: Duration::from_secs(30),
            },
            pre_retry: None,
        },
        FailureType::Conflict => RetryPolicy {
            max_attempts: 1,
            backoff: BackoffStrategy::Fixed(Duration::from_secs(5)),
            pre_retry: Some(PreRetryAction::CheckExisting),
        },
        FailureType::TranslationInternalFailure => RetryPolicy {
            max_attempts: 2,
            backoff: BackoffStrategy::Fixed(Duration::from_secs(5)),
            pre_retry: Some(PreRetryAction::ReUpload),
        },
        FailureType::TranslationDownloadFailure => RetryPolicy {
            max_attempts: 1,
            backoff: BackoffStrategy::Fixed(Duration::from_secs(5)),
            pre_retry: Some(PreRetryAction::ReUpload),
        },
        FailureType::RegionMismatch => RetryPolicy {
            max_attempts: 1,
            backoff: BackoffStrategy::Fixed(Duration::from_millis(100)),
            pre_retry: Some(PreRetryAction::SwitchRegion),
        },
        FailureType::Unknown => RetryPolicy {
            max_attempts: 3,
            backoff: BackoffStrategy::Exponential {
                base: Duration::from_secs(1),
                max: Duration::from_secs(30),
            },
            pre_retry: None,
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_status() {
        assert_eq!(
            FailureType::from_status(429),
            Some(FailureType::RateLimited)
        );
        assert_eq!(
            FailureType::from_status(401),
            Some(FailureType::Unauthorized)
        );
        assert_eq!(
            FailureType::from_status(500),
            Some(FailureType::ServerError)
        );
        assert_eq!(
            FailureType::from_status(503),
            Some(FailureType::ServiceUnavailable)
        );
        assert_eq!(FailureType::from_status(200), None);
        assert_eq!(FailureType::from_status(404), None);
    }

    #[test]
    fn test_from_message() {
        assert_eq!(
            FailureType::from_message("TranslationWorker-InternalFailure"),
            Some(FailureType::TranslationInternalFailure)
        );
        assert_eq!(
            FailureType::from_message("TranslationWorker-FailedDownload: could not fetch"),
            Some(FailureType::TranslationDownloadFailure)
        );
        assert_eq!(
            FailureType::from_message("Region mismatch detected"),
            Some(FailureType::RegionMismatch)
        );
        assert_eq!(FailureType::from_message("some random error"), None);
    }

    #[test]
    fn test_exponential_backoff_delay() {
        let policy = default_policy(FailureType::ServerError);
        let d0 = policy.delay_for_attempt(0, None);
        let d1 = policy.delay_for_attempt(1, None);
        let d2 = policy.delay_for_attempt(2, None);

        assert_eq!(d0, Duration::from_secs(1));
        assert_eq!(d1, Duration::from_secs(2));
        assert_eq!(d2, Duration::from_secs(4));
    }

    #[test]
    fn test_header_based_backoff() {
        let policy = default_policy(FailureType::RateLimited);

        // With header
        let d = policy.delay_for_attempt(0, Some(Duration::from_secs(30)));
        assert_eq!(d, Duration::from_secs(30));

        // Without header — falls back to exponential
        let d = policy.delay_for_attempt(0, None);
        assert_eq!(d, Duration::from_secs(1));
    }

    #[test]
    fn test_pre_retry_actions() {
        assert_eq!(
            default_policy(FailureType::Unauthorized).pre_retry,
            Some(PreRetryAction::RefreshToken)
        );
        assert_eq!(
            default_policy(FailureType::RegionMismatch).pre_retry,
            Some(PreRetryAction::SwitchRegion)
        );
        assert_eq!(
            default_policy(FailureType::TranslationInternalFailure).pre_retry,
            Some(PreRetryAction::ReUpload)
        );
        assert_eq!(default_policy(FailureType::ServerError).pre_retry, None);
    }

    #[test]
    fn test_circuit_breaker_trigger() {
        assert!(FailureType::ServerError.triggers_circuit_breaker());
        assert!(FailureType::ServiceUnavailable.triggers_circuit_breaker());
        assert!(FailureType::NetworkError.triggers_circuit_breaker());
        assert!(!FailureType::RateLimited.triggers_circuit_breaker());
        assert!(!FailureType::Unauthorized.triggers_circuit_breaker());
    }
}

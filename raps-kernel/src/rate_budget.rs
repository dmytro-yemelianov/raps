// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Proactive rate limit budget tracking.
//!
//! Tracks rate limit consumption from APS response headers and provides
//! proactive budget checking before sending requests. Falls back to
//! known APS limits when headers are absent.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dashmap::DashMap;

// ---------------------------------------------------------------------------
// Rate status
// ---------------------------------------------------------------------------

/// Current rate limit status for an endpoint group.
#[derive(Debug, Clone)]
pub enum RateStatus {
    /// Plenty of budget remaining.
    Ok { remaining: u32, limit: u32 },
    /// Less than 10% of budget remaining.
    NearLimit { remaining: u32, limit: u32 },
    /// Budget exhausted — must wait.
    Exhausted { retry_after: Duration },
    /// No budget info available (first request or headers missing).
    Unknown,
}

// ---------------------------------------------------------------------------
// Budget entry
// ---------------------------------------------------------------------------

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

struct RateBudget {
    remaining: AtomicU32,
    limit: AtomicU32,
    reset_at: AtomicU64, // unix millis
}

impl RateBudget {
    fn new() -> Self {
        Self {
            remaining: AtomicU32::new(u32::MAX), // unknown = unlimited
            limit: AtomicU32::new(0),
            reset_at: AtomicU64::new(0),
        }
    }

    fn update(&self, remaining: u32, limit: u32, reset_at_millis: u64) {
        self.remaining.store(remaining, Ordering::Relaxed);
        self.limit.store(limit, Ordering::Relaxed);
        self.reset_at.store(reset_at_millis, Ordering::Relaxed);
    }

    fn status(&self) -> RateStatus {
        let remaining = self.remaining.load(Ordering::Relaxed);
        let limit = self.limit.load(Ordering::Relaxed);
        let reset_at = self.reset_at.load(Ordering::Relaxed);
        let now = now_millis();

        // If limit is 0, we haven't received headers yet
        if limit == 0 {
            return RateStatus::Unknown;
        }

        // If past reset time, budget has refreshed
        if now >= reset_at && reset_at > 0 {
            return RateStatus::Ok {
                remaining: limit,
                limit,
            };
        }

        if remaining == 0 {
            let wait = reset_at.saturating_sub(now);
            return RateStatus::Exhausted {
                retry_after: Duration::from_millis(wait),
            };
        }

        let threshold = (limit as f64 * 0.1).ceil() as u32;
        if remaining <= threshold {
            RateStatus::NearLimit { remaining, limit }
        } else {
            RateStatus::Ok { remaining, limit }
        }
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Global rate budget registry, one entry per API endpoint group.
pub struct RateBudgetRegistry {
    budgets: DashMap<String, RateBudget>,
    known_limits: DashMap<String, KnownLimit>,
}

/// Hardcoded fallback limits for APS APIs.
struct KnownLimit {
    limit: u32,
    window: Duration,
}

impl RateBudgetRegistry {
    pub fn new() -> Self {
        let reg = Self {
            budgets: DashMap::new(),
            known_limits: DashMap::new(),
        };

        // APS known rate limits
        reg.known_limits.insert(
            "authentication".to_string(),
            KnownLimit {
                limit: 500,
                window: Duration::from_secs(60),
            },
        );
        reg.known_limits.insert(
            "data-management".to_string(),
            KnownLimit {
                limit: 100,
                window: Duration::from_secs(60),
            },
        );
        reg.known_limits.insert(
            "model-derivative".to_string(),
            KnownLimit {
                limit: 20,
                window: Duration::from_secs(60),
            },
        );
        reg.known_limits.insert(
            "oss".to_string(),
            KnownLimit {
                limit: 500,
                window: Duration::from_secs(60),
            },
        );

        reg
    }

    /// Check current rate status for an endpoint group.
    pub fn check(&self, endpoint: &str) -> RateStatus {
        if let Some(budget) = self.budgets.get(endpoint) {
            let status = budget.status();
            if !matches!(status, RateStatus::Unknown) {
                return status;
            }
        }

        // Fall back to known limits if we have them
        if let Some(known) = self.known_limits.get(endpoint) {
            return RateStatus::Ok {
                remaining: known.limit,
                limit: known.limit,
            };
        }

        RateStatus::Unknown
    }

    /// Update budget from HTTP response headers.
    ///
    /// Parses standard rate limit headers:
    /// - `X-RateLimit-Remaining`
    /// - `X-RateLimit-Limit`
    /// - `X-RateLimit-Reset` (unix timestamp)
    /// - `Retry-After` (seconds, used when exhausted)
    pub fn record_from_headers(&self, endpoint: &str, headers: &reqwest::header::HeaderMap) {
        let remaining = headers
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u32>().ok());

        let limit = headers
            .get("x-ratelimit-limit")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u32>().ok());

        let reset = headers
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(|secs| secs * 1000); // convert to millis

        // Also check Retry-After for 429 responses
        let retry_after = headers
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());

        if let (Some(rem), Some(lim)) = (remaining, limit) {
            let reset_millis = reset.unwrap_or_else(|| {
                // If no reset header, estimate from known window
                let window = self
                    .known_limits
                    .get(endpoint)
                    .map(|k| k.window)
                    .unwrap_or(Duration::from_secs(60));
                now_millis() + window.as_millis() as u64
            });

            let budget = self
                .budgets
                .entry(endpoint.to_string())
                .or_insert_with(RateBudget::new);
            budget.update(rem, lim, reset_millis);

            if rem == 0 {
                tracing::warn!(endpoint, limit = lim, "rate limit exhausted");
            } else if rem <= (lim as f64 * 0.1).ceil() as u32 {
                tracing::debug!(
                    endpoint,
                    remaining = rem,
                    limit = lim,
                    "rate limit near exhaustion"
                );
            }
        } else if let Some(retry_secs) = retry_after {
            // 429 response with only Retry-After
            let budget = self
                .budgets
                .entry(endpoint.to_string())
                .or_insert_with(RateBudget::new);
            let known_limit = self
                .known_limits
                .get(endpoint)
                .map(|k| k.limit)
                .unwrap_or(100);
            budget.update(0, known_limit, now_millis() + retry_secs * 1000);
        }
    }

    /// Clear all tracked budgets.
    pub fn reset_all(&self) {
        self.budgets.clear();
        self.known_limits.clear();
    }

    /// Snapshot of all tracked budgets.
    pub fn snapshot(&self) -> Vec<(String, u32, u32)> {
        self.budgets
            .iter()
            .map(|entry| {
                let name = entry.key().clone();
                let remaining = entry.value().remaining.load(Ordering::Relaxed);
                let limit = entry.value().limit.load(Ordering::Relaxed);
                (name, remaining, limit)
            })
            .collect()
    }
}

impl Default for RateBudgetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Global instance
// ---------------------------------------------------------------------------

static REGISTRY: std::sync::OnceLock<RateBudgetRegistry> = std::sync::OnceLock::new();

/// Get the global rate budget registry.
pub fn registry() -> &'static RateBudgetRegistry {
    REGISTRY.get_or_init(RateBudgetRegistry::new)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    #[test]
    fn test_unknown_without_headers() {
        let reg = RateBudgetRegistry::new();
        // Unknown endpoint
        assert!(matches!(reg.check("unknown-api"), RateStatus::Unknown));
    }

    #[test]
    fn test_known_limit_fallback() {
        let reg = RateBudgetRegistry::new();
        // Known endpoint without any recorded headers
        match reg.check("oss") {
            RateStatus::Ok { remaining, limit } => {
                assert_eq!(remaining, 500);
                assert_eq!(limit, 500);
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn test_record_from_headers() {
        let reg = RateBudgetRegistry::new();

        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-remaining", HeaderValue::from_static("42"));
        headers.insert("x-ratelimit-limit", HeaderValue::from_static("100"));
        headers.insert("x-ratelimit-reset", HeaderValue::from_static("9999999999"));

        reg.record_from_headers("oss", &headers);

        match reg.check("oss") {
            RateStatus::Ok { remaining, limit } => {
                assert_eq!(remaining, 42);
                assert_eq!(limit, 100);
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn test_near_limit_detection() {
        let reg = RateBudgetRegistry::new();

        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-remaining", HeaderValue::from_static("5"));
        headers.insert("x-ratelimit-limit", HeaderValue::from_static("100"));
        // Reset far in the future
        headers.insert("x-ratelimit-reset", HeaderValue::from_static("9999999999"));

        reg.record_from_headers("model-derivative", &headers);

        assert!(matches!(
            reg.check("model-derivative"),
            RateStatus::NearLimit {
                remaining: 5,
                limit: 100
            }
        ));
    }

    #[test]
    fn test_exhausted_detection() {
        let reg = RateBudgetRegistry::new();

        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));
        headers.insert("x-ratelimit-limit", HeaderValue::from_static("100"));
        // Reset far in the future
        headers.insert("x-ratelimit-reset", HeaderValue::from_static("9999999999"));

        reg.record_from_headers("oss", &headers);

        assert!(matches!(reg.check("oss"), RateStatus::Exhausted { .. }));
    }

    #[test]
    fn test_budget_refreshes_after_reset() {
        let reg = RateBudgetRegistry::new();

        {
            let budget = reg
                .budgets
                .entry("test-api".to_string())
                .or_insert_with(RateBudget::new);

            // Set budget exhausted with reset in the past
            budget.update(0, 100, now_millis().saturating_sub(1000));
        } // drop the DashMap ref before calling check()

        // Should report as refreshed
        match reg.check("test-api") {
            RateStatus::Ok { remaining, limit } => {
                assert_eq!(remaining, 100);
                assert_eq!(limit, 100);
            }
            other => panic!("expected Ok after reset, got {other:?}"),
        }
    }
}

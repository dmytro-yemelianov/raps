// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Header-level rate limit state.
//!
//! Parses `X-RateLimit-*` headers from individual responses and computes
//! a proactive throttle delay when remaining quota drops below 10% of the
//! limit.  This complements [`crate::rate_budget`], which tracks the global
//! per-endpoint budget across requests; `RateLimitState` is a lightweight,
//! per-response value used to decide whether to insert a short sleep *before*
//! the next outgoing request.

/// Rate-limit state parsed from a single HTTP response.
#[derive(Debug, Default, Clone)]
pub struct RateLimitState {
    /// The total request quota for the current window (`X-RateLimit-Limit`).
    pub limit: Option<u64>,
    /// Remaining requests in the current window (`X-RateLimit-Remaining`).
    pub remaining: Option<u64>,
    /// Unix timestamp at which the window resets (`X-RateLimit-Reset`).
    pub reset_at: Option<u64>,
}

impl RateLimitState {
    /// Parse rate-limit headers from a response header map.
    pub fn from_headers(headers: &reqwest::header::HeaderMap) -> Self {
        let get = |name: &str| -> Option<u64> {
            headers.get(name)?.to_str().ok()?.parse().ok()
        };
        Self {
            limit: get("x-ratelimit-limit"),
            remaining: get("x-ratelimit-remaining"),
            reset_at: get("x-ratelimit-reset"),
        }
    }

    /// Return a duration to sleep when remaining quota is below 10% of limit.
    ///
    /// Returns `None` when quota is healthy or headers are missing.
    /// The sleep is capped at 30 seconds to avoid blocking indefinitely.
    pub fn throttle_delay(&self) -> Option<std::time::Duration> {
        let limit = self.limit?;
        let remaining = self.remaining?;
        let reset_at = self.reset_at?;

        if limit == 0 {
            return None;
        }

        let pct = remaining as f64 / limit as f64;
        if pct < 0.1 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            let wait = reset_at.saturating_sub(now).min(30);
            if wait > 0 {
                return Some(std::time::Duration::from_secs(wait));
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    fn make_headers(remaining: &str, limit: &str, reset_at: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            "x-ratelimit-remaining",
            HeaderValue::from_str(remaining).unwrap(),
        );
        h.insert(
            "x-ratelimit-limit",
            HeaderValue::from_str(limit).unwrap(),
        );
        h.insert(
            "x-ratelimit-reset",
            HeaderValue::from_str(reset_at).unwrap(),
        );
        h
    }

    #[test]
    fn test_from_headers_parses_all_fields() {
        let h = make_headers("42", "500", "9999999999");
        let state = RateLimitState::from_headers(&h);
        assert_eq!(state.remaining, Some(42));
        assert_eq!(state.limit, Some(500));
        assert_eq!(state.reset_at, Some(9999999999));
    }

    #[test]
    fn test_from_headers_empty() {
        let state = RateLimitState::from_headers(&HeaderMap::new());
        assert!(state.limit.is_none());
        assert!(state.remaining.is_none());
        assert!(state.reset_at.is_none());
    }

    #[test]
    fn test_throttle_delay_healthy_quota() {
        // 90% remaining — no throttle
        let h = make_headers("90", "100", "9999999999");
        let state = RateLimitState::from_headers(&h);
        assert!(state.throttle_delay().is_none());
    }

    #[test]
    fn test_throttle_delay_exactly_at_threshold() {
        // Exactly 10% remaining — threshold is exclusive (< 0.1 required)
        let h = make_headers("10", "100", "9999999999");
        let state = RateLimitState::from_headers(&h);
        assert!(state.throttle_delay().is_none());
    }

    #[test]
    fn test_throttle_delay_below_threshold() {
        // 5% remaining, reset far in the future — should throttle (capped 30s)
        let h = make_headers("5", "100", "9999999999");
        let state = RateLimitState::from_headers(&h);
        let delay = state.throttle_delay();
        assert!(delay.is_some());
        assert!(delay.unwrap().as_secs() <= 30);
    }

    #[test]
    fn test_throttle_delay_zero_remaining() {
        let h = make_headers("0", "100", "9999999999");
        let state = RateLimitState::from_headers(&h);
        let delay = state.throttle_delay();
        assert!(delay.is_some());
        assert!(delay.unwrap().as_secs() <= 30);
    }

    #[test]
    fn test_throttle_delay_reset_in_past() {
        // reset_at already passed — wait = 0, no sleep needed
        let h = make_headers("0", "100", "1"); // unix ts 1 is long past
        let state = RateLimitState::from_headers(&h);
        assert!(state.throttle_delay().is_none());
    }

    #[test]
    fn test_throttle_delay_zero_limit() {
        // limit=0 should not divide
        let mut h = HeaderMap::new();
        h.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));
        h.insert("x-ratelimit-limit", HeaderValue::from_static("0"));
        h.insert("x-ratelimit-reset", HeaderValue::from_static("9999999999"));
        let state = RateLimitState::from_headers(&h);
        assert!(state.throttle_delay().is_none());
    }

    #[test]
    fn test_throttle_delay_missing_headers() {
        let state = RateLimitState::default();
        assert!(state.throttle_delay().is_none());
    }
}

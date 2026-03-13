// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Per-endpoint failure statistics for adaptive retry behaviour.
//!
//! Tracks request counts, failure counts, and latency per endpoint.
//! When an endpoint shows a high failure rate, `backoff_multiplier` returns
//! a value > 1 so callers can increase retry delays proactively.
//! Stats are persisted to `~/.cache/raps/endpoint_stats.json`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Statistics for a single endpoint (identified by normalised key).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EndpointRecord {
    pub requests: u64,
    pub failures: u64,
    pub total_ms: u64,
    /// Unix timestamp (seconds) of the most recent failure.
    pub last_failure_at: Option<i64>,
}

impl EndpointRecord {
    /// Fraction of requests that failed (0.0 – 1.0).
    pub fn failure_rate(&self) -> f64 {
        if self.requests == 0 {
            0.0
        } else {
            self.failures as f64 / self.requests as f64
        }
    }

    /// Average request duration in milliseconds.
    pub fn avg_ms(&self) -> u64 {
        if self.requests == 0 {
            0
        } else {
            self.total_ms / self.requests
        }
    }

    /// Extra delay multiplier based on recent failure rate.
    ///
    /// * > 50 % failure → 4×
    /// * > 25 % failure → 2×
    /// * otherwise       → 1× (no change)
    pub fn backoff_multiplier(&self) -> u32 {
        let rate = self.failure_rate();
        if rate > 0.5 {
            4
        } else if rate > 0.25 {
            2
        } else {
            1
        }
    }
}

/// Collection of per-endpoint records, with load/save support.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct EndpointStats {
    pub records: HashMap<String, EndpointRecord>,
}

impl EndpointStats {
    fn cache_path() -> Option<std::path::PathBuf> {
        let dirs = directories::ProjectDirs::from("com", "autodesk", "raps")?;
        Some(dirs.cache_dir().join("endpoint_stats.json"))
    }

    /// Load stats from the cache file, returning a default instance on any error.
    pub fn load() -> Self {
        let path = match Self::cache_path() {
            Some(p) => p,
            None => return Self::default(),
        };
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    /// Persist stats to the cache file (best-effort; errors are silently ignored).
    pub fn save(&self) {
        if let Some(path) = Self::cache_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, content);
            }
        }
    }

    /// Normalise a URL to a stable endpoint key.
    ///
    /// Strips query parameters and keeps only `host + first 3 path segments`
    /// so that parameterised paths (e.g. `/oss/v2/buckets/{bucket}`) map to
    /// the same key.
    pub fn endpoint_key(url: &str) -> String {
        if let Ok(u) = url::Url::parse(url) {
            let segments: Vec<&str> = u.path_segments().map(|s| s.collect()).unwrap_or_default();
            let prefix = segments
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("/");
            format!("{}/{}", u.host_str().unwrap_or(""), prefix)
        } else {
            url.to_string()
        }
    }

    /// Record the outcome of a single request.
    ///
    /// * `url`         – full request URL (will be normalised to a key)
    /// * `duration_ms` – elapsed time in milliseconds
    /// * `failed`      – `true` for any non-2xx / network error outcome
    pub fn record_request(&mut self, url: &str, duration_ms: u64, failed: bool) {
        let key = Self::endpoint_key(url);
        let rec = self.records.entry(key).or_default();
        rec.requests += 1;
        rec.total_ms += duration_ms;
        if failed {
            rec.failures += 1;
            rec.last_failure_at = Some(chrono::Utc::now().timestamp());
        }
    }

    /// Return the backoff multiplier for the given URL's endpoint.
    ///
    /// Returns `1` (no extra delay) for unknown endpoints.
    pub fn backoff_multiplier(&self, url: &str) -> u32 {
        let key = Self::endpoint_key(url);
        self.records
            .get(&key)
            .map(|r| r.backoff_multiplier())
            .unwrap_or(1)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_rate_no_requests() {
        let rec = EndpointRecord::default();
        assert_eq!(rec.failure_rate(), 0.0);
    }

    #[test]
    fn test_failure_rate_half() {
        let rec = EndpointRecord {
            requests: 4,
            failures: 2,
            ..Default::default()
        };
        assert!((rec.failure_rate() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_backoff_multiplier_low_failure() {
        let rec = EndpointRecord {
            requests: 100,
            failures: 10,
            ..Default::default()
        };
        assert_eq!(rec.backoff_multiplier(), 1);
    }

    #[test]
    fn test_backoff_multiplier_medium_failure() {
        let rec = EndpointRecord {
            requests: 4,
            failures: 2,
            ..Default::default()
        };
        // exactly 50 % → NOT > 0.5, so multiplier = 2
        assert_eq!(rec.backoff_multiplier(), 2);
    }

    #[test]
    fn test_backoff_multiplier_high_failure() {
        let rec = EndpointRecord {
            requests: 10,
            failures: 6,
            ..Default::default()
        };
        assert_eq!(rec.backoff_multiplier(), 4);
    }

    #[test]
    fn test_endpoint_key_strips_query_and_truncates_path() {
        let key = EndpointStats::endpoint_key(
            "https://developer.api.autodesk.com/oss/v2/buckets/my-bucket/objects?limit=10",
        );
        assert_eq!(key, "developer.api.autodesk.com/oss/v2/buckets");
    }

    #[test]
    fn test_record_request_increments_counts() {
        let mut stats = EndpointStats::default();
        stats.record_request("https://api.example.com/a/b/c", 100, false);
        stats.record_request("https://api.example.com/a/b/c", 200, true);

        let key = EndpointStats::endpoint_key("https://api.example.com/a/b/c");
        let rec = &stats.records[&key];
        assert_eq!(rec.requests, 2);
        assert_eq!(rec.failures, 1);
        assert_eq!(rec.total_ms, 300);
    }

    #[test]
    fn test_backoff_multiplier_unknown_endpoint() {
        let stats = EndpointStats::default();
        assert_eq!(stats.backoff_multiplier("https://unknown.example.com/x"), 1);
    }
}

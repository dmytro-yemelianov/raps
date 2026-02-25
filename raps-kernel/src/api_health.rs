// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! API Health Tracking
//!
//! Lock-free latency tracker fed by `send_with_retry`. Computes running average,
//! jitter (standard deviation), min/max, and health status from actual request data
//! with zero extra HTTP calls.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

/// Health status classification based on latency metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// avg < 500ms and jitter < 200ms
    Healthy,
    /// avg < 2s and jitter < 500ms
    Degraded,
    /// avg >= 2s or jitter >= 500ms
    Unhealthy,
    /// No samples recorded yet
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Snapshot of API health metrics at a point in time.
#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    pub avg_latency: Duration,
    pub jitter: Duration,
    pub min_latency: Duration,
    pub max_latency: Duration,
    pub last_latency: Duration,
    pub sample_count: usize,
    pub failure_count: usize,
    pub health_status: HealthStatus,
}

/// Global lock-free API health tracker.
///
/// Uses atomics (same pattern as `profiler.rs`) to avoid mutex contention
/// during concurrent HTTP requests.
pub struct ApiHealth {
    /// Sum of latencies in microseconds.
    total_latency_us: AtomicU64,
    /// Sum of squared latencies in microseconds (for jitter/stddev computation).
    total_latency_sq_us: AtomicU64,
    /// Number of successful request samples.
    sample_count: AtomicUsize,
    /// Minimum observed latency in microseconds.
    min_latency_us: AtomicU64,
    /// Maximum observed latency in microseconds.
    max_latency_us: AtomicU64,
    /// Most recent latency in microseconds.
    last_latency_us: AtomicU64,
    /// Number of terminal failures.
    failure_count: AtomicUsize,
}

impl ApiHealth {
    const fn new() -> Self {
        Self {
            total_latency_us: AtomicU64::new(0),
            total_latency_sq_us: AtomicU64::new(0),
            sample_count: AtomicUsize::new(0),
            min_latency_us: AtomicU64::new(u64::MAX),
            max_latency_us: AtomicU64::new(0),
            last_latency_us: AtomicU64::new(0),
            failure_count: AtomicUsize::new(0),
        }
    }
}

static GLOBAL_HEALTH: ApiHealth = ApiHealth::new();

/// Record latency from a completed HTTP request.
/// Called from `send_with_retry` on every successful completion.
pub fn record_latency(duration: Duration) {
    let us = duration.as_micros() as u64;

    GLOBAL_HEALTH
        .total_latency_us
        .fetch_add(us, Ordering::Relaxed);

    // For jitter: accumulate squared latency.
    // Cap individual squared value to avoid overflow on extremely slow requests.
    let us_sq = us.saturating_mul(us);
    GLOBAL_HEALTH
        .total_latency_sq_us
        .fetch_add(us_sq, Ordering::Relaxed);

    GLOBAL_HEALTH.sample_count.fetch_add(1, Ordering::Relaxed);

    // Update min (atomic fetch_min)
    GLOBAL_HEALTH
        .min_latency_us
        .fetch_min(us, Ordering::Relaxed);

    // Update max (atomic fetch_max)
    GLOBAL_HEALTH
        .max_latency_us
        .fetch_max(us, Ordering::Relaxed);

    // Store last latency
    GLOBAL_HEALTH.last_latency_us.store(us, Ordering::Relaxed);
}

/// Record a terminal failure (request that exhausted all retries).
pub fn record_failure() {
    GLOBAL_HEALTH.failure_count.fetch_add(1, Ordering::Relaxed);
}

/// Take a snapshot of current API health metrics.
pub fn snapshot() -> HealthSnapshot {
    let count = GLOBAL_HEALTH.sample_count.load(Ordering::Relaxed);
    let failures = GLOBAL_HEALTH.failure_count.load(Ordering::Relaxed);

    if count == 0 {
        return HealthSnapshot {
            avg_latency: Duration::ZERO,
            jitter: Duration::ZERO,
            min_latency: Duration::ZERO,
            max_latency: Duration::ZERO,
            last_latency: Duration::ZERO,
            sample_count: 0,
            failure_count: failures,
            health_status: HealthStatus::Unknown,
        };
    }

    let total_us = GLOBAL_HEALTH.total_latency_us.load(Ordering::Relaxed);
    let total_sq_us = GLOBAL_HEALTH.total_latency_sq_us.load(Ordering::Relaxed);
    let min_us = GLOBAL_HEALTH.min_latency_us.load(Ordering::Relaxed);
    let max_us = GLOBAL_HEALTH.max_latency_us.load(Ordering::Relaxed);
    let last_us = GLOBAL_HEALTH.last_latency_us.load(Ordering::Relaxed);

    let avg_us = total_us / count as u64;

    // Compute standard deviation (jitter) using: stddev = sqrt(E[X^2] - E[X]^2)
    let mean_sq = total_sq_us / count as u64;
    let sq_mean = avg_us.saturating_mul(avg_us);
    let variance_us = mean_sq.saturating_sub(sq_mean);
    let jitter_us = isqrt(variance_us);

    let avg_ms = avg_us / 1000;
    let jitter_ms = jitter_us / 1000;

    let health_status = classify_health(avg_ms, jitter_ms);

    HealthSnapshot {
        avg_latency: Duration::from_micros(avg_us),
        jitter: Duration::from_micros(jitter_us),
        min_latency: Duration::from_micros(min_us),
        max_latency: Duration::from_micros(max_us),
        last_latency: Duration::from_micros(last_us),
        sample_count: count,
        failure_count: failures,
        health_status,
    }
}

/// Format a one-liner status string.
pub fn status_line() -> String {
    let snap = snapshot();
    match snap.health_status {
        HealthStatus::Unknown => "API: unknown (no samples)".to_string(),
        _ => {
            format!(
                "API: {} (avg: {}, jitter: {})",
                snap.health_status,
                format_duration_ms(snap.avg_latency),
                format_duration_ms(snap.jitter),
            )
        }
    }
}

/// Classify health status from average latency and jitter (in milliseconds).
fn classify_health(avg_ms: u64, jitter_ms: u64) -> HealthStatus {
    if avg_ms < 500 && jitter_ms < 200 {
        HealthStatus::Healthy
    } else if avg_ms < 2000 && jitter_ms < 500 {
        HealthStatus::Degraded
    } else {
        HealthStatus::Unhealthy
    }
}

/// Integer square root (Heron's method).
fn isqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Format a Duration as a human-readable millisecond or second string.
pub fn format_duration_ms(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests use isolated ApiHealth instances to avoid global state interference.

    fn make_tracker() -> ApiHealth {
        ApiHealth::new()
    }

    fn record_on(tracker: &ApiHealth, duration: Duration) {
        let us = duration.as_micros() as u64;
        tracker.total_latency_us.fetch_add(us, Ordering::Relaxed);
        let us_sq = us.saturating_mul(us);
        tracker
            .total_latency_sq_us
            .fetch_add(us_sq, Ordering::Relaxed);
        tracker.sample_count.fetch_add(1, Ordering::Relaxed);
        tracker.min_latency_us.fetch_min(us, Ordering::Relaxed);
        tracker.max_latency_us.fetch_max(us, Ordering::Relaxed);
        tracker.last_latency_us.store(us, Ordering::Relaxed);
    }

    fn snapshot_of(tracker: &ApiHealth) -> HealthSnapshot {
        let count = tracker.sample_count.load(Ordering::Relaxed);
        let failures = tracker.failure_count.load(Ordering::Relaxed);

        if count == 0 {
            return HealthSnapshot {
                avg_latency: Duration::ZERO,
                jitter: Duration::ZERO,
                min_latency: Duration::ZERO,
                max_latency: Duration::ZERO,
                last_latency: Duration::ZERO,
                sample_count: 0,
                failure_count: failures,
                health_status: HealthStatus::Unknown,
            };
        }

        let total_us = tracker.total_latency_us.load(Ordering::Relaxed);
        let total_sq_us = tracker.total_latency_sq_us.load(Ordering::Relaxed);
        let min_us = tracker.min_latency_us.load(Ordering::Relaxed);
        let max_us = tracker.max_latency_us.load(Ordering::Relaxed);
        let last_us = tracker.last_latency_us.load(Ordering::Relaxed);

        let avg_us = total_us / count as u64;
        let mean_sq = total_sq_us / count as u64;
        let sq_mean = avg_us.saturating_mul(avg_us);
        let variance_us = mean_sq.saturating_sub(sq_mean);
        let jitter_us = isqrt(variance_us);

        let avg_ms = avg_us / 1000;
        let jitter_ms = jitter_us / 1000;
        let health_status = classify_health(avg_ms, jitter_ms);

        HealthSnapshot {
            avg_latency: Duration::from_micros(avg_us),
            jitter: Duration::from_micros(jitter_us),
            min_latency: Duration::from_micros(min_us),
            max_latency: Duration::from_micros(max_us),
            last_latency: Duration::from_micros(last_us),
            sample_count: count,
            failure_count: failures,
            health_status,
        }
    }

    #[test]
    fn test_no_samples_unknown() {
        let tracker = make_tracker();
        let snap = snapshot_of(&tracker);
        assert_eq!(snap.health_status, HealthStatus::Unknown);
        assert_eq!(snap.sample_count, 0);
        assert_eq!(snap.avg_latency, Duration::ZERO);
    }

    #[test]
    fn test_single_sample() {
        let tracker = make_tracker();
        record_on(&tracker, Duration::from_millis(100));
        let snap = snapshot_of(&tracker);
        assert_eq!(snap.sample_count, 1);
        assert_eq!(snap.avg_latency.as_millis(), 100);
        assert_eq!(snap.min_latency.as_millis(), 100);
        assert_eq!(snap.max_latency.as_millis(), 100);
        assert_eq!(snap.jitter.as_millis(), 0);
        assert_eq!(snap.health_status, HealthStatus::Healthy);
    }

    #[test]
    fn test_average_latency() {
        let tracker = make_tracker();
        record_on(&tracker, Duration::from_millis(100));
        record_on(&tracker, Duration::from_millis(200));
        record_on(&tracker, Duration::from_millis(300));
        let snap = snapshot_of(&tracker);
        assert_eq!(snap.sample_count, 3);
        assert_eq!(snap.avg_latency.as_millis(), 200);
        assert_eq!(snap.min_latency.as_millis(), 100);
        assert_eq!(snap.max_latency.as_millis(), 300);
    }

    #[test]
    fn test_jitter_calculation() {
        let tracker = make_tracker();
        // Two samples: 100ms and 300ms. Mean = 200ms.
        // Variance = ((100-200)^2 + (300-200)^2) / 2 = 10000 ms^2
        // Stddev = sqrt(10000) = 100ms
        record_on(&tracker, Duration::from_millis(100));
        record_on(&tracker, Duration::from_millis(300));
        let snap = snapshot_of(&tracker);
        let jitter_ms = snap.jitter.as_millis();
        // Allow small rounding error from integer arithmetic
        assert!(jitter_ms >= 99 && jitter_ms <= 101, "jitter was {}ms", jitter_ms);
    }

    #[test]
    fn test_healthy_classification() {
        assert_eq!(classify_health(200, 50), HealthStatus::Healthy);
        assert_eq!(classify_health(499, 199), HealthStatus::Healthy);
    }

    #[test]
    fn test_degraded_classification() {
        assert_eq!(classify_health(500, 50), HealthStatus::Degraded);
        assert_eq!(classify_health(1999, 499), HealthStatus::Degraded);
        assert_eq!(classify_health(200, 200), HealthStatus::Degraded);
    }

    #[test]
    fn test_unhealthy_classification() {
        assert_eq!(classify_health(2000, 50), HealthStatus::Unhealthy);
        assert_eq!(classify_health(200, 500), HealthStatus::Unhealthy);
        assert_eq!(classify_health(5000, 1000), HealthStatus::Unhealthy);
    }

    #[test]
    fn test_failure_count() {
        let tracker = make_tracker();
        tracker.failure_count.fetch_add(1, Ordering::Relaxed);
        tracker.failure_count.fetch_add(1, Ordering::Relaxed);
        let snap = snapshot_of(&tracker);
        assert_eq!(snap.failure_count, 2);
    }

    #[test]
    fn test_last_latency_tracks_most_recent() {
        let tracker = make_tracker();
        record_on(&tracker, Duration::from_millis(100));
        record_on(&tracker, Duration::from_millis(500));
        record_on(&tracker, Duration::from_millis(200));
        let snap = snapshot_of(&tracker);
        assert_eq!(snap.last_latency.as_millis(), 200);
    }

    #[test]
    fn test_min_max_tracking() {
        let tracker = make_tracker();
        record_on(&tracker, Duration::from_millis(500));
        record_on(&tracker, Duration::from_millis(100));
        record_on(&tracker, Duration::from_millis(1000));
        record_on(&tracker, Duration::from_millis(200));
        let snap = snapshot_of(&tracker);
        assert_eq!(snap.min_latency.as_millis(), 100);
        assert_eq!(snap.max_latency.as_millis(), 1000);
    }

    #[test]
    fn test_isqrt() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(4), 2);
        assert_eq!(isqrt(9), 3);
        assert_eq!(isqrt(10), 3); // floor
        assert_eq!(isqrt(100), 10);
        assert_eq!(isqrt(10000), 100);
    }

    #[test]
    fn test_format_duration_ms() {
        assert_eq!(format_duration_ms(Duration::from_millis(0)), "0ms");
        assert_eq!(format_duration_ms(Duration::from_millis(340)), "340ms");
        assert_eq!(format_duration_ms(Duration::from_millis(999)), "999ms");
        assert_eq!(format_duration_ms(Duration::from_millis(1000)), "1.0s");
        assert_eq!(format_duration_ms(Duration::from_millis(2100)), "2.1s");
    }

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(HealthStatus::Unhealthy.to_string(), "unhealthy");
        assert_eq!(HealthStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_overflow_safety_large_latency() {
        let tracker = make_tracker();
        // Very large latency (100 seconds) — should not panic
        record_on(&tracker, Duration::from_secs(100));
        let snap = snapshot_of(&tracker);
        assert_eq!(snap.sample_count, 1);
        assert_eq!(snap.avg_latency.as_secs(), 100);
    }
}

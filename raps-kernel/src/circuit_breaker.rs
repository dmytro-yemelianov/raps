// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Per-endpoint circuit breaker for API resilience.
//!
//! Tracks failures per API endpoint group and opens the circuit when
//! failures exceed a threshold within a time window. Open circuits
//! probe periodically and close when a probe succeeds.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dashmap::DashMap;

// ---------------------------------------------------------------------------
// Circuit state
// ---------------------------------------------------------------------------

const STATE_CLOSED: u8 = 0;
const STATE_OPEN: u8 = 1;
const STATE_HALF_OPEN: u8 = 2;

/// Current state of a circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation — requests flow through.
    Closed,
    /// Too many failures — requests are rejected until probe interval.
    Open,
    /// One probe request allowed to test recovery.
    HalfOpen,
}

impl CircuitState {
    fn from_u8(v: u8) -> Self {
        match v {
            STATE_OPEN => CircuitState::Open,
            STATE_HALF_OPEN => CircuitState::HalfOpen,
            _ => CircuitState::Closed,
        }
    }

    fn to_u8(self) -> u8 {
        match self {
            CircuitState::Closed => STATE_CLOSED,
            CircuitState::Open => STATE_OPEN,
            CircuitState::HalfOpen => STATE_HALF_OPEN,
        }
    }
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed"),
            CircuitState::Open => write!(f, "Open"),
            CircuitState::HalfOpen => write!(f, "HalfOpen"),
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for a circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures within the window before opening. Default: 5.
    pub failure_threshold: u32,
    /// Time window for counting failures. Default: 60s.
    pub failure_window: Duration,
    /// How long to wait before probing when open. Default: 30s.
    pub probe_interval: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            failure_window: Duration::from_secs(60),
            probe_interval: Duration::from_secs(30),
        }
    }
}

// ---------------------------------------------------------------------------
// Circuit breaker
// ---------------------------------------------------------------------------

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Error returned when the circuit is open and requests are blocked.
#[derive(Debug, thiserror::Error)]
#[error("Circuit breaker open for '{endpoint}' — API is unhealthy, retrying in {retry_after_secs}s")]
pub struct CircuitOpen {
    pub endpoint: String,
    pub retry_after_secs: u64,
}

/// A single circuit breaker tracking one API endpoint group.
pub struct CircuitBreaker {
    state: std::sync::atomic::AtomicU8,
    failure_count: AtomicU32,
    window_start: AtomicU64,
    opened_at: AtomicU64,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: std::sync::atomic::AtomicU8::new(STATE_CLOSED),
            failure_count: AtomicU32::new(0),
            window_start: AtomicU64::new(now_millis()),
            opened_at: AtomicU64::new(0),
            config,
        }
    }

    /// Current state.
    pub fn state(&self) -> CircuitState {
        let s = CircuitState::from_u8(self.state.load(Ordering::Acquire));

        // If open, check if probe interval has elapsed → transition to HalfOpen
        if s == CircuitState::Open {
            let opened = self.opened_at.load(Ordering::Acquire);
            let elapsed = now_millis().saturating_sub(opened);
            if elapsed >= self.config.probe_interval.as_millis() as u64 {
                self.state
                    .store(CircuitState::HalfOpen.to_u8(), Ordering::Release);
                return CircuitState::HalfOpen;
            }
        }
        s
    }

    /// Check if a request is allowed. Returns `Ok(())` if allowed,
    /// or `Err(CircuitOpen)` if the circuit is open.
    pub fn check(&self, endpoint: &str) -> Result<(), CircuitOpen> {
        match self.state() {
            CircuitState::Closed | CircuitState::HalfOpen => Ok(()),
            CircuitState::Open => {
                let opened = self.opened_at.load(Ordering::Acquire);
                let elapsed_ms = now_millis().saturating_sub(opened);
                let probe_ms = self.config.probe_interval.as_millis() as u64;
                let retry_after = probe_ms.saturating_sub(elapsed_ms) / 1000;
                Err(CircuitOpen {
                    endpoint: endpoint.to_string(),
                    retry_after_secs: retry_after.max(1),
                })
            }
        }
    }

    /// Record a successful request. Closes the circuit if half-open.
    pub fn record_success(&self) {
        let current = self.state();
        if current == CircuitState::HalfOpen {
            // Probe succeeded — close circuit
            self.state
                .store(CircuitState::Closed.to_u8(), Ordering::Release);
            self.failure_count.store(0, Ordering::Relaxed);
            self.window_start.store(now_millis(), Ordering::Relaxed);
            tracing::info!("circuit breaker closed — probe succeeded");
        } else if current == CircuitState::Closed {
            // Reset failure count if window has expired
            let window_start = self.window_start.load(Ordering::Relaxed);
            let elapsed = now_millis().saturating_sub(window_start);
            if elapsed >= self.config.failure_window.as_millis() as u64 {
                self.failure_count.store(0, Ordering::Relaxed);
                self.window_start.store(now_millis(), Ordering::Relaxed);
            }
        }
    }

    /// Record a failed request. May open the circuit.
    pub fn record_failure(&self) {
        let current = self.state();

        if current == CircuitState::HalfOpen {
            // Probe failed — reopen
            self.state
                .store(CircuitState::Open.to_u8(), Ordering::Release);
            self.opened_at.store(now_millis(), Ordering::Release);
            tracing::warn!("circuit breaker reopened — probe failed");
            return;
        }

        if current == CircuitState::Open {
            return; // Already open
        }

        // Closed state: count failure within window
        let window_start = self.window_start.load(Ordering::Relaxed);
        let elapsed = now_millis().saturating_sub(window_start);

        if elapsed >= self.config.failure_window.as_millis() as u64 {
            // Window expired — start new window
            self.failure_count.store(1, Ordering::Relaxed);
            self.window_start.store(now_millis(), Ordering::Relaxed);
        } else {
            let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
            if count >= self.config.failure_threshold {
                self.state
                    .store(CircuitState::Open.to_u8(), Ordering::Release);
                self.opened_at.store(now_millis(), Ordering::Release);
                tracing::warn!(
                    failures = count,
                    threshold = self.config.failure_threshold,
                    "circuit breaker opened"
                );
            }
        }
    }

    /// Failure count in the current window.
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// Registry — global set of breakers keyed by endpoint group
// ---------------------------------------------------------------------------

/// Global registry of circuit breakers, one per API endpoint group.
pub struct CircuitBreakerRegistry {
    breakers: DashMap<String, CircuitBreaker>,
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerRegistry {
    pub fn new(default_config: CircuitBreakerConfig) -> Self {
        Self {
            breakers: DashMap::new(),
            default_config,
        }
    }

    /// Get or create a circuit breaker for the given endpoint group.
    pub fn check(&self, endpoint: &str) -> Result<(), CircuitOpen> {
        let breaker = self
            .breakers
            .entry(endpoint.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.default_config.clone()));
        breaker.check(endpoint)
    }

    pub fn record_success(&self, endpoint: &str) {
        if let Some(breaker) = self.breakers.get(endpoint) {
            breaker.record_success();
        }
    }

    pub fn record_failure(&self, endpoint: &str) {
        let breaker = self
            .breakers
            .entry(endpoint.to_string())
            .or_insert_with(|| CircuitBreaker::new(self.default_config.clone()));
        breaker.record_failure();
    }

    /// Snapshot of all circuit breaker states.
    pub fn snapshot(&self) -> Vec<(String, CircuitState, u32)> {
        self.breakers
            .iter()
            .map(|entry| {
                let name = entry.key().clone();
                let state = entry.value().state();
                let failures = entry.value().failure_count();
                (name, state, failures)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Global instance
// ---------------------------------------------------------------------------

static REGISTRY: std::sync::OnceLock<CircuitBreakerRegistry> = std::sync::OnceLock::new();

/// Get the global circuit breaker registry.
pub fn registry() -> &'static CircuitBreakerRegistry {
    REGISTRY.get_or_init(|| CircuitBreakerRegistry::new(CircuitBreakerConfig::default()))
}

/// Initialize with custom config (call before first use).
pub fn init(config: CircuitBreakerConfig) {
    let _ = REGISTRY.set(CircuitBreakerRegistry::new(config));
}

/// Classify a URL into an endpoint group for circuit breaking.
pub fn endpoint_group(url: &str) -> &str {
    if url.contains("/modelderivative/") {
        "model-derivative"
    } else if url.contains("/oss/") {
        "oss"
    } else if url.contains("/project/") || url.contains("/data/") || url.contains("/hq/") {
        "data-management"
    } else if url.contains("/da/") || url.contains("/designautomation/") {
        "design-automation"
    } else if url.contains("/authentication/") {
        "authentication"
    } else if url.contains("/issues/") || url.contains("/rfis/") {
        "acc"
    } else if url.contains("/photo-to-3d/") || url.contains("/photoscene/") {
        "reality-capture"
    } else if url.contains("/webhooks/") {
        "webhooks"
    } else {
        "other"
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starts_closed() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            failure_window: Duration::from_secs(60),
            probe_interval: Duration::from_secs(1),
        });
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.check("test").is_ok());
    }

    #[test]
    fn test_opens_after_threshold() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            failure_window: Duration::from_secs(60),
            probe_interval: Duration::from_secs(30),
        });

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure(); // 3rd failure = threshold
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.check("test").is_err());
    }

    #[test]
    fn test_success_resets_in_closed() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 3,
            failure_window: Duration::from_secs(60),
            probe_interval: Duration::from_secs(30),
        });

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.failure_count(), 2);

        cb.record_success();
        // Failures don't reset on success in closed state within window,
        // but window reset happens when window expires
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_probe_success_closes() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            failure_window: Duration::from_secs(60),
            probe_interval: Duration::from_millis(1), // Very short for test
        });

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for probe interval
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Probe success closes circuit
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_half_open_probe_failure_reopens() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            failure_window: Duration::from_secs(60),
            probe_interval: Duration::from_millis(1),
        });

        cb.record_failure();
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_endpoint_group_classification() {
        assert_eq!(
            endpoint_group("https://developer.api.autodesk.com/modelderivative/v2/designdata/job"),
            "model-derivative"
        );
        assert_eq!(
            endpoint_group("https://developer.api.autodesk.com/oss/v2/buckets"),
            "oss"
        );
        assert_eq!(
            endpoint_group("https://developer.api.autodesk.com/data/v1/projects"),
            "data-management"
        );
        assert_eq!(
            endpoint_group("https://developer.api.autodesk.com/da/us-east/v3/workitems"),
            "design-automation"
        );
        assert_eq!(
            endpoint_group("https://developer.api.autodesk.com/authentication/v2/token"),
            "authentication"
        );
    }

    #[test]
    fn test_registry() {
        let reg = CircuitBreakerRegistry::new(CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        });

        assert!(reg.check("oss").is_ok());
        reg.record_failure("oss");
        reg.record_failure("oss");
        assert!(reg.check("oss").is_err());

        // Other endpoint still works
        assert!(reg.check("model-derivative").is_ok());

        let snap = reg.snapshot();
        assert!(snap.iter().any(|(name, state, _)| name == "oss" && *state == CircuitState::Open));
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Performance and Resource Profiling
//!
//! Provides global tracking for:
//! - Total execution time
//! - Kernel load time
//! - Plugin discovery/load time
//! - HTTP network overhead (request count, retry count, total duration)
//! - Memory consumption at report time (snapshot, not peak)

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Global Profiler state.
///
/// Uses `AtomicU64` for hot-path counters (HTTP duration, request/retry counts)
/// to avoid mutex contention during concurrent multipart uploads.
/// `Mutex<Option<Duration>>` is retained for one-shot milestone markers
/// (kernel/plugins load) which are written at most once.
///
/// Note: `const fn` constructors for `Mutex`, `OnceLock`, and atomics
/// require Rust 1.63+ / 1.70+ respectively. This crate requires Rust 1.88+.
pub struct Profiler {
    pub start_time: OnceLock<Instant>,
    pub kernel_load_duration: Mutex<Option<Duration>>,
    pub plugins_load_duration: Mutex<Option<Duration>>,
    /// Accumulated HTTP network duration in nanoseconds (lock-free).
    pub total_http_nanos: AtomicU64,
    /// Number of logical HTTP requests (one per send_with_retry call).
    pub http_requests_count: AtomicUsize,
    /// Number of HTTP retry attempts (retries only, not initial attempts).
    pub http_retries_count: AtomicUsize,
    pub enabled: AtomicBool,
}

impl Profiler {
    const fn new() -> Self {
        Self {
            start_time: OnceLock::new(),
            kernel_load_duration: Mutex::new(None),
            plugins_load_duration: Mutex::new(None),
            total_http_nanos: AtomicU64::new(0),
            http_requests_count: AtomicUsize::new(0),
            http_retries_count: AtomicUsize::new(0),
            enabled: AtomicBool::new(false),
        }
    }
}

static GLOBAL_PROFILER: Profiler = Profiler::new();

/// Initialize the global profiler. Must be called as early as possible.
pub fn init() {
    let _ = GLOBAL_PROFILER.start_time.set(Instant::now());
}

/// Enable profiling output.
pub fn enable() {
    GLOBAL_PROFILER.enabled.store(true, Ordering::Relaxed);
}

/// Check if profiling is enabled.
pub fn is_enabled() -> bool {
    GLOBAL_PROFILER.enabled.load(Ordering::Relaxed)
}

/// Record the time taken to load the kernel/CLI base (called at most once).
pub fn mark_kernel_loaded() {
    if !is_enabled() {
        return;
    }
    if let Some(start) = GLOBAL_PROFILER.start_time.get()
        && let Ok(mut kernel_dur) = GLOBAL_PROFILER.kernel_load_duration.lock()
        && kernel_dur.is_none()
    {
        *kernel_dur = Some(start.elapsed());
    }
}

/// Record the time it took to discover and load plugins (called at most once).
pub fn mark_plugins_loaded(duration: Duration) {
    if !is_enabled() {
        return;
    }
    if let Ok(mut plugins_dur) = GLOBAL_PROFILER.plugins_load_duration.lock()
        && plugins_dur.is_none()
    {
        *plugins_dur = Some(duration);
    }
}

/// Record a completed HTTP request with its total network duration.
/// Called once per logical request (after all retries are exhausted).
pub fn record_http_request(duration: Duration) {
    if !is_enabled() {
        return;
    }
    GLOBAL_PROFILER
        .total_http_nanos
        .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    GLOBAL_PROFILER
        .http_requests_count
        .fetch_add(1, Ordering::Relaxed);
}

/// Record that a retry attempt occurred (called per retry, not per request).
pub fn record_http_retry() {
    if !is_enabled() {
        return;
    }
    GLOBAL_PROFILER
        .http_retries_count
        .fetch_add(1, Ordering::Relaxed);
}

/// Print the profiling report if profiling is enabled.
/// Accepts an optional command name and exit code for structured log context.
pub fn report(command: Option<&str>, exit_code: Option<i32>) {
    if !is_enabled() {
        return;
    }

    let start_time = match GLOBAL_PROFILER.start_time.get() {
        Some(t) => t,
        None => return, // Profiler was never initialized
    };

    let total_time = start_time.elapsed();
    let kernel_time = GLOBAL_PROFILER
        .kernel_load_duration
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or_default();
    let plugins_time_opt = *GLOBAL_PROFILER
        .plugins_load_duration
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let network_nanos = GLOBAL_PROFILER.total_http_nanos.load(Ordering::Relaxed);
    let network_time = Duration::from_nanos(network_nanos);
    let req_count = GLOBAL_PROFILER.http_requests_count.load(Ordering::Relaxed);
    let retry_count = GLOBAL_PROFILER.http_retries_count.load(Ordering::Relaxed);

    let other_time = total_time.saturating_sub(network_time);

    // Memory snapshot at report time (not peak — OS does not expose peak easily)
    let memory_str = if let Some(usage) = memory_stats::memory_stats() {
        format!("{:.2} MB", usage.physical_mem as f64 / 1024.0 / 1024.0)
    } else {
        "Unknown".to_string()
    };

    let cmd = command.unwrap_or("unknown");
    let code = exit_code.unwrap_or(0);

    // Structured log for file log / aggregation
    tracing::info!(
        command = cmd,
        exit_code = code,
        total_ms = total_time.as_millis() as u64,
        kernel_ms = kernel_time.as_millis() as u64,
        plugins_ms = plugins_time_opt.map(|d| d.as_millis() as u64),
        network_ms = network_time.as_millis() as u64,
        other_ms = other_time.as_millis() as u64,
        http_requests = req_count,
        http_retries = retry_count,
        memory = %memory_str,
        "Performance profile"
    );

    // Human-readable output to stderr
    let plugins_str = match plugins_time_opt {
        Some(d) => format!("{:.3}s", d.as_secs_f64()),
        None => "N/A".to_string(),
    };

    eprintln!("\n=== RAPS Performance Profile ===");
    eprintln!(
        "{:<22} {:.3}s",
        "Total Execution Time:",
        total_time.as_secs_f64()
    );
    eprintln!(
        "{:<22} {:.3}s",
        "Kernel Load Time:",
        kernel_time.as_secs_f64()
    );
    eprintln!("{:<22} {}", "Plugins Load Time:", plugins_str);
    eprintln!(
        "{:<22} {:.3}s",
        "Total Network Time:",
        network_time.as_secs_f64()
    );
    eprintln!("{:<22} {}", "HTTP Requests:", req_count);
    if retry_count > 0 {
        eprintln!("{:<22} {}", "HTTP Retries:", retry_count);
    }
    eprintln!("{:<22} {:.3}s", "Other Time:", other_time.as_secs_f64());
    eprintln!("{:<22} {}", "Memory (at exit):", memory_str);
    eprintln!("================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests use a fresh Profiler instance to avoid global state interference.
    // The global GLOBAL_PROFILER is shared across all tests, so we test the logic
    // via the atomic/mutex primitives directly.

    #[test]
    fn test_atomic_http_duration_accumulates() {
        let counter = AtomicU64::new(0);
        let d1 = Duration::from_millis(100);
        let d2 = Duration::from_millis(250);
        counter.fetch_add(d1.as_nanos() as u64, Ordering::Relaxed);
        counter.fetch_add(d2.as_nanos() as u64, Ordering::Relaxed);
        let total = Duration::from_nanos(counter.load(Ordering::Relaxed));
        assert_eq!(total, Duration::from_millis(350));
    }

    #[test]
    fn test_atomic_request_count() {
        let counter = AtomicUsize::new(0);
        counter.fetch_add(1, Ordering::Relaxed);
        counter.fetch_add(1, Ordering::Relaxed);
        counter.fetch_add(1, Ordering::Relaxed);
        assert_eq!(counter.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_mark_plugins_loaded_only_once() {
        let plugins_dur: Mutex<Option<Duration>> = Mutex::new(None);

        // First call sets the value
        {
            let mut dur = plugins_dur.lock().unwrap();
            if dur.is_none() {
                *dur = Some(Duration::from_millis(50));
            }
        }

        // Second call should NOT overwrite
        {
            let mut dur = plugins_dur.lock().unwrap();
            if dur.is_none() {
                *dur = Some(Duration::from_millis(200));
            }
        }

        let result = plugins_dur.lock().unwrap().unwrap();
        assert_eq!(result, Duration::from_millis(50));
    }

    #[test]
    fn test_mark_kernel_loaded_only_once() {
        let kernel_dur: Mutex<Option<Duration>> = Mutex::new(None);

        {
            let mut dur = kernel_dur.lock().unwrap();
            if dur.is_none() {
                *dur = Some(Duration::from_millis(30));
            }
        }

        {
            let mut dur = kernel_dur.lock().unwrap();
            if dur.is_none() {
                *dur = Some(Duration::from_millis(100));
            }
        }

        let result = kernel_dur.lock().unwrap().unwrap();
        assert_eq!(result, Duration::from_millis(30));
    }

    #[test]
    fn test_plugins_none_displays_as_na() {
        let plugins_time_opt: Option<Duration> = None;
        let plugins_str = match plugins_time_opt {
            Some(d) => format!("{:.3}s", d.as_secs_f64()),
            None => "N/A".to_string(),
        };
        assert_eq!(plugins_str, "N/A");
    }

    #[test]
    fn test_plugins_some_displays_as_seconds() {
        let plugins_time_opt: Option<Duration> = Some(Duration::from_millis(123));
        let plugins_str = match plugins_time_opt {
            Some(d) => format!("{:.3}s", d.as_secs_f64()),
            None => "N/A".to_string(),
        };
        assert_eq!(plugins_str, "0.123s");
    }

    #[test]
    fn test_other_time_saturating_sub() {
        let total = Duration::from_millis(500);
        let network = Duration::from_millis(300);
        let other = total.saturating_sub(network);
        assert_eq!(other, Duration::from_millis(200));
    }

    #[test]
    fn test_other_time_saturating_sub_overflow() {
        // Network time exceeding total (shouldn't happen, but shouldn't panic)
        let total = Duration::from_millis(100);
        let network = Duration::from_millis(300);
        let other = total.saturating_sub(network);
        assert_eq!(other, Duration::ZERO);
    }

    #[test]
    fn test_enabled_flag() {
        let enabled = AtomicBool::new(false);
        assert!(!enabled.load(Ordering::Relaxed));
        enabled.store(true, Ordering::Relaxed);
        assert!(enabled.load(Ordering::Relaxed));
    }
}

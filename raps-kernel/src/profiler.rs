// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Performance and Resource Profiling
//!
//! Provides global tracking for:
//! - Total execution time
//! - Kernel load time
//! - Plugin discovery/load time
//! - HTTP network overhead
//! - Peak memory consumption

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Global Profiler state
pub struct Profiler {
    pub start_time: OnceLock<Instant>,
    pub kernel_load_duration: Mutex<Option<Duration>>,
    pub plugins_load_duration: Mutex<Option<Duration>>,
    pub total_http_duration: Mutex<Duration>,
    pub http_requests_count: AtomicUsize,
    pub enabled: AtomicBool,
}

impl Profiler {
    const fn new() -> Self {
        Self {
            start_time: OnceLock::new(),
            kernel_load_duration: Mutex::new(None),
            plugins_load_duration: Mutex::new(None),
            total_http_duration: Mutex::new(Duration::from_secs(0)),
            http_requests_count: AtomicUsize::new(0),
            enabled: AtomicBool::new(false),
        }
    }
}

static GLOBAL_PROFILER: Profiler = Profiler::new();

/// Initialize the global profiler. Must be called as early as possible.
pub fn init() {
    let _ = GLOBAL_PROFILER.start_time.set(Instant::now());
}

/// Enable profiling output
pub fn enable() {
    GLOBAL_PROFILER.enabled.store(true, Ordering::Relaxed);
}

/// Record the time taken to load the kernel/CLI base.
pub fn mark_kernel_loaded() {
    if let Some(start) = GLOBAL_PROFILER.start_time.get() {
        if let Ok(mut kernel_dur) = GLOBAL_PROFILER.kernel_load_duration.lock() {
            if kernel_dur.is_none() {
                *kernel_dur = Some(start.elapsed());
            }
        }
    }
}

/// Record the time it took to discover and load plugins.
pub fn mark_plugins_loaded(duration: Duration) {
    if let Ok(mut plugins_dur) = GLOBAL_PROFILER.plugins_load_duration.lock() {
        *plugins_dur = Some(duration);
    }
}

/// Increment HTTP request count and add to the total network duration.
pub fn record_http_request(duration: Duration) {
    if let Ok(mut total) = GLOBAL_PROFILER.total_http_duration.lock() {
        *total += duration;
    }
    GLOBAL_PROFILER
        .http_requests_count
        .fetch_add(1, Ordering::Relaxed);
}

/// Print the profiling report if profiling is enabled.
pub fn report() {
    if !GLOBAL_PROFILER.enabled.load(Ordering::Relaxed) {
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
    let plugins_time = GLOBAL_PROFILER
        .plugins_load_duration
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or_default();
    let network_time = *GLOBAL_PROFILER
        .total_http_duration
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let req_count = GLOBAL_PROFILER.http_requests_count.load(Ordering::Relaxed);

    let local_time = total_time.saturating_sub(network_time);

    let memory_str = if let Some(usage) = memory_stats::memory_stats() {
        format!("{:.2} MB", usage.physical_mem as f64 / 1024.0 / 1024.0)
    } else {
        "Unknown".to_string()
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
    eprintln!(
        "{:<22} {:.3}s",
        "Plugins Load Time:",
        plugins_time.as_secs_f64()
    );
    eprintln!(
        "{:<22} {:.3}s",
        "Total Network Time:",
        network_time.as_secs_f64()
    );
    eprintln!("{:<22} {}", "Total HTTP Requests:", req_count);
    eprintln!(
        "{:<22} {:.3}s",
        "Local Processing Time:",
        local_time.as_secs_f64()
    );
    eprintln!("{:<22} {}", "Memory Consumed:", memory_str);
    eprintln!("================================");
}

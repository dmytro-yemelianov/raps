// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Tracked operation helper
//!
//! Wraps an async API call with a spinner (in interactive/Table mode) and appends
//! timing + API health info on completion. Non-interactive and structured output
//! modes (JSON, YAML, CSV) run the operation directly with no spinner.

use anyhow::Result;
use std::future::Future;
use std::time::Instant;

use crate::output::OutputFormat;
use raps_kernel::api_health;

/// Execute an async operation with progress tracking.
///
/// - **Table mode (interactive)**: Shows a spinner during the operation, then
///   replaces it with a completion line including elapsed time and API health.
/// - **Non-interactive / JSON / YAML / CSV**: Runs the operation directly with
///   no visual feedback.
/// - **On error**: Shows a failure indicator with elapsed time, then propagates
///   the error.
pub async fn tracked_op<F, Fut, T>(message: &str, output_format: OutputFormat, op: F) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    if !output_format.supports_colors() {
        return op().await;
    }

    let spinner = raps_kernel::progress::spinner(message);
    let start = Instant::now();

    match op().await {
        Ok(result) => {
            let elapsed = start.elapsed();
            let snap = api_health::snapshot();

            let status_suffix = if snap.sample_count > 0 {
                format!(
                    " ({}, avg: {}, API: {})",
                    api_health::format_duration_ms(elapsed),
                    api_health::format_duration_ms(snap.avg_latency),
                    snap.health_status,
                )
            } else {
                format!(" ({})", api_health::format_duration_ms(elapsed))
            };

            spinner.finish_with_message(format!("\u{2713} {}{}", message, status_suffix));
            Ok(result)
        }
        Err(err) => {
            let elapsed = start.elapsed();
            spinner.finish_with_message(format!(
                "\u{2717} {} (after {})",
                message,
                api_health::format_duration_ms(elapsed),
            ));
            Err(err)
        }
    }
}

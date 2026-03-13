// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Watch mode for translation jobs: polls until complete.

use std::time::{Duration, Instant};

use anyhow::Result;
use colored::Colorize;

use crate::output::OutputFormat;
use raps_derivative::DerivativeClient;
use raps_kernel::progress;

/// Poll a translation job until it reaches a terminal state.
///
/// Displays a spinner while waiting. On success prints a confirmation line;
/// on failure returns an error so the caller can propagate it.
///
/// * `poll_interval` – seconds between status checks
/// * `timeout_secs`  – maximum seconds to wait; 0 means no timeout
pub async fn watch_translation(
    client: &DerivativeClient,
    urn: &str,
    poll_interval: u64,
    timeout_secs: u64,
    output_format: &OutputFormat,
) -> Result<()> {
    let deadline = if timeout_secs > 0 {
        Some(Instant::now() + Duration::from_secs(timeout_secs))
    } else {
        None
    };

    let spinner = progress::spinner("Waiting for translation...");

    loop {
        if let Some(dl) = deadline {
            if Instant::now() > dl {
                spinner.finish_with_message(format!(
                    "{} Timed out after {}s",
                    "\u{23F1}".yellow().bold(),
                    timeout_secs
                ));
                anyhow::bail!("Translation watch timed out after {}s", timeout_secs);
            }
        }

        let (status, progress_msg) = client.get_status(urn).await?;

        spinner.set_message(format!(
            "Translating... status={} progress={}",
            status, progress_msg
        ));

        match status.as_str() {
            "success" => {
                spinner.finish_with_message(format!(
                    "{} Translation complete! ({})",
                    "\u{2713}".green().bold(),
                    progress_msg
                ));
                if output_format.supports_colors() {
                    println!("{} Translation succeeded", "\u{2713}".green().bold());
                }
                return Ok(());
            }
            "failed" | "timeout" => {
                spinner.finish_with_message(format!("{} Translation {}", "X".red().bold(), status));
                anyhow::bail!("Translation failed with status: {}", status);
            }
            _ => {
                tokio::time::sleep(Duration::from_secs(poll_interval)).await;
            }
        }
    }
}

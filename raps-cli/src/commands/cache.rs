// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Cache management commands.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum CacheCommands {
    /// Show cache statistics (entries, total size)
    Stats,

    /// Remove all cached artifacts
    Clear,

    /// Show the cache directory path
    Dir,
}

#[derive(Serialize, schemars::JsonSchema)]
struct CacheStatsOutput {
    enabled: bool,
    directory: String,
    entries: usize,
    total_size: u64,
    total_size_human: String,
}

#[derive(Serialize, schemars::JsonSchema)]
struct CacheClearOutput {
    removed: usize,
}

impl CacheCommands {
    pub fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            CacheCommands::Stats => cache_stats(output_format),
            CacheCommands::Clear => cache_clear(output_format),
            CacheCommands::Dir => cache_dir(output_format),
        }
    }
}

fn cache_stats(output_format: OutputFormat) -> Result<()> {
    let dir = raps_kernel::cache::cache_dir()?;
    let (entries, total_size) = raps_kernel::cache::stats()?;

    let output = CacheStatsOutput {
        enabled: raps_kernel::cache::is_enabled(),
        directory: dir.display().to_string(),
        entries,
        total_size,
        total_size_human: format_size(total_size),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Download Cache".bold());
            println!("{}", "-".repeat(60));
            println!(
                "  {:<16} {}",
                "Status:".bold(),
                if output.enabled {
                    "enabled".green().to_string()
                } else {
                    "disabled".red().to_string()
                }
            );
            println!("  {:<16} {}", "Directory:".bold(), output.directory);
            println!("  {:<16} {}", "Entries:".bold(), output.entries);
            println!(
                "  {:<16} {} ({})",
                "Total size:".bold(),
                output.total_size_human,
                output.total_size
            );
            println!("{}", "-".repeat(60));
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

fn cache_clear(output_format: OutputFormat) -> Result<()> {
    let removed = raps_kernel::cache::clear()?;

    let output = CacheClearOutput { removed };

    match output_format {
        OutputFormat::Table => {
            if removed > 0 {
                println!(
                    "{} Removed {} cached artifact(s)",
                    "✓".green().bold(),
                    removed
                );
            } else {
                println!("{}", "Cache is already empty.".yellow());
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

fn cache_dir(output_format: OutputFormat) -> Result<()> {
    let dir = raps_kernel::cache::cache_dir()?;

    match output_format {
        OutputFormat::Table => {
            println!("{}", dir.display());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "directory": dir.display().to_string()
            }))?;
        }
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

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

    /// Remove old or excess cached artifacts
    Prune {
        /// Remove entries older than duration (e.g. 30d, 2w, 12h)
        #[arg(long)]
        older_than: Option<String>,

        /// Keep total cache size under limit (e.g. 500M, 1G)
        #[arg(long)]
        max_size: Option<String>,

        /// Show what would be removed without actually removing
        #[arg(long)]
        dry_run: bool,
    },
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

#[derive(Serialize, schemars::JsonSchema)]
struct CachePruneOutput {
    removed: usize,
    strategy: String,
}

impl CacheCommands {
    pub fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            CacheCommands::Stats => cache_stats(output_format),
            CacheCommands::Clear => cache_clear(output_format),
            CacheCommands::Dir => cache_dir(output_format),
            CacheCommands::Prune {
                older_than,
                max_size,
                dry_run,
            } => cache_prune(older_than, max_size, dry_run, output_format),
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

fn cache_prune(
    older_than: Option<String>,
    max_size: Option<String>,
    dry_run: bool,
    output_format: OutputFormat,
) -> Result<()> {
    if older_than.is_none() && max_size.is_none() {
        anyhow::bail!("Specify at least one of --older-than or --max-size");
    }

    let mut total_removed = 0usize;
    let mut strategies = Vec::new();

    if let Some(ref age_str) = older_than {
        let max_age = raps_kernel::cache::parse_age(age_str)?;
        if dry_run {
            let (before_count, _) = raps_kernel::cache::stats()?;
            strategies.push(format!(
                "older-than={age_str} (dry run, {before_count} entries total)"
            ));
        } else {
            let removed = raps_kernel::cache::prune_older_than(max_age)?;
            total_removed += removed;
            strategies.push(format!("older-than={age_str}"));
        }
    }

    if let Some(ref size_str) = max_size {
        let max_bytes = raps_kernel::cache::parse_size(size_str)?;
        if dry_run {
            let (_, current_size) = raps_kernel::cache::stats()?;
            let over = current_size.saturating_sub(max_bytes);
            strategies.push(format!(
                "max-size={size_str} (dry run, {} over limit)",
                format_size(over)
            ));
        } else {
            let removed = raps_kernel::cache::prune_to_size(max_bytes)?;
            total_removed += removed;
            strategies.push(format!("max-size={size_str}"));
        }
    }

    let strategy = strategies.join(", ");
    let output = CachePruneOutput {
        removed: total_removed,
        strategy: strategy.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            if dry_run {
                println!("{} Dry run — strategy: {}", "ℹ".cyan().bold(), strategy);
            } else if total_removed > 0 {
                println!(
                    "{} Pruned {} cached artifact(s) ({})",
                    "✓".green().bold(),
                    total_removed,
                    strategy
                );
            } else {
                println!("{}", "Nothing to prune.".yellow());
            }
        }
        _ => {
            output_format.write(&output)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(512), "512 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(1024), "1.00 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(1_048_576), "1.00 MB");
    }

    #[test]
    fn test_format_size_gb() {
        assert_eq!(format_size(1_073_741_824), "1.00 GB");
    }
}

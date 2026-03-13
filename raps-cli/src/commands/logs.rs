// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Log management commands — show, path, clear, follow.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;

use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum LogsCommands {
    /// Print the last N lines of today's log file (default: 50)
    Show {
        /// Number of lines to print
        #[arg(long, short = 'n', value_name = "N", default_value = "50")]
        lines: usize,
    },

    /// Print the path to the log directory
    Path,

    /// Delete all log files in the log directory
    Clear {
        /// Skip the confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Tail the current log file — print new lines as they appear (Ctrl+C to stop)
    Follow,
}

impl LogsCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            LogsCommands::Show { lines } => logs_show(lines, output_format),
            LogsCommands::Path => logs_path(output_format),
            LogsCommands::Clear { yes } => logs_clear(yes, output_format),
            LogsCommands::Follow => logs_follow(),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Return the path to the most recent log file in `dir`, or `None` if no log
/// files exist there.
fn latest_log_file(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut files: Vec<_> = entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("raps.log"))
        .collect();
    // Most recently modified first
    files.sort_by_key(|e| std::cmp::Reverse(e.metadata().and_then(|m| m.modified()).ok()));
    files.into_iter().next().map(|e| e.path())
}

// ---------------------------------------------------------------------------
// show
// ---------------------------------------------------------------------------

fn logs_show(lines: usize, output_format: OutputFormat) -> Result<()> {
    let dir = raps_kernel::logging::log_dir();
    let path = latest_log_file(&dir)
        .with_context(|| format!("No log files found in {}", dir.display()))?;

    // Read the file and collect all lines, then take the last N.
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let all_lines: Vec<&str> = content.lines().collect();
    let tail: Vec<&str> = if all_lines.len() > lines {
        all_lines[all_lines.len() - lines..].to_vec()
    } else {
        all_lines
    };

    if matches!(
        output_format,
        OutputFormat::Table | OutputFormat::Plain | OutputFormat::Ndjson
    ) {
        eprintln!("{}", format!("==> {} <==", path.display()).dimmed());
        for line in &tail {
            println!("{}", line);
        }
    } else {
        output_format.write(&serde_json::json!({
            "file": path.display().to_string(),
            "lines": tail,
        }))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// path
// ---------------------------------------------------------------------------

fn logs_path(output_format: OutputFormat) -> Result<()> {
    let dir = raps_kernel::logging::log_dir();

    if matches!(
        output_format,
        OutputFormat::Table | OutputFormat::Plain | OutputFormat::Ndjson
    ) {
        println!("{}", dir.display());
    } else {
        output_format.write(&serde_json::json!({
            "log_directory": dir.display().to_string(),
        }))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// clear
// ---------------------------------------------------------------------------

fn logs_clear(yes: bool, output_format: OutputFormat) -> Result<()> {
    let dir = raps_kernel::logging::log_dir();

    if !dir.exists() {
        if matches!(
            output_format,
            OutputFormat::Table | OutputFormat::Plain | OutputFormat::Ndjson
        ) {
            println!(
                "{}",
                "Log directory does not exist — nothing to clear.".yellow()
            );
        } else {
            output_format.write(&serde_json::json!({"removed": 0}))?;
        }
        return Ok(());
    }

    // Count log files first.
    let entries: Vec<_> = std::fs::read_dir(&dir)
        .with_context(|| format!("Cannot read log directory {}", dir.display()))?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("raps.log"))
        .collect();

    if entries.is_empty() {
        if matches!(
            output_format,
            OutputFormat::Table | OutputFormat::Plain | OutputFormat::Ndjson
        ) {
            println!("{}", "No log files to clear.".yellow());
        } else {
            output_format.write(&serde_json::json!({"removed": 0}))?;
        }
        return Ok(());
    }

    // Confirmation prompt (unless --yes or machine-readable output).
    let confirmed =
        yes || !matches!(
            output_format,
            OutputFormat::Table | OutputFormat::Plain | OutputFormat::Ndjson
        ) || {
            eprint!(
                "Delete {} log file(s) in {}? [y/N] ",
                entries.len(),
                dir.display()
            );
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
        };

    if !confirmed {
        println!("{}", "Aborted.".yellow());
        return Ok(());
    }

    let mut removed = 0usize;
    for entry in &entries {
        if std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }

    if matches!(
        output_format,
        OutputFormat::Table | OutputFormat::Plain | OutputFormat::Ndjson
    ) {
        println!(
            "{} Removed {} log file(s) from {}",
            "✓".green().bold(),
            removed,
            dir.display()
        );
    } else {
        output_format.write(&serde_json::json!({"removed": removed}))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// follow
// ---------------------------------------------------------------------------

fn logs_follow() -> Result<()> {
    let dir = raps_kernel::logging::log_dir();
    let path = latest_log_file(&dir)
        .with_context(|| format!("No log files found in {}", dir.display()))?;

    eprintln!(
        "{}",
        format!("Following {} (Ctrl+C to stop)", path.display()).dimmed()
    );

    let mut file =
        std::fs::File::open(&path).with_context(|| format!("Failed to open {}", path.display()))?;

    // Seek to end so we only show new lines.
    use std::io::{Read, Seek, SeekFrom};
    file.seek(SeekFrom::End(0))?;

    let mut buf = String::new();
    loop {
        buf.clear();
        let bytes_read = file.read_to_string(&mut buf)?;
        if bytes_read > 0 {
            for line in buf.lines() {
                println!("{}", line);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! `raps watch <dir>` — watch a directory and auto-upload new/modified files.

use anyhow::{Context, Result};
use chrono::Local;
use clap::Args;
use colored::Colorize;
use glob::Pattern;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use raps_kernel::config::Config as RapsConfig;
use raps_oss::OssClient;

#[derive(Debug, Args)]
pub struct WatchArgs {
    /// Local directory to watch
    pub dir: PathBuf,

    /// Target OSS bucket key
    #[arg(long, short)]
    pub bucket: String,

    /// Only watch files matching this glob pattern (e.g. "*.rvt")
    #[arg(long)]
    pub filter: Option<String>,

    /// Exclude files matching this glob pattern (can be repeated)
    #[arg(long = "exclude")]
    pub excludes: Vec<String>,

    /// Debounce delay in milliseconds (wait after last change before uploading)
    #[arg(long, default_value = "500")]
    pub debounce_ms: u64,

    /// Remote key prefix (prepended to the relative file path)
    #[arg(long)]
    pub prefix: Option<String>,
}

/// Check whether a file path matches the given filter/exclude rules.
fn matches_rules(
    path: &std::path::Path,
    filter: &Option<Pattern>,
    excludes: &[Pattern],
) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };

    // Must match the include filter (if set)
    if let Some(pattern) = filter {
        if !pattern.matches(file_name) {
            return false;
        }
    }

    // Must not match any exclude pattern
    for exc in excludes {
        if exc.matches(file_name) {
            return false;
        }
    }

    true
}

pub async fn execute(
    args: WatchArgs,
    config: &RapsConfig,
    _output_format: crate::output::OutputFormat,
) -> Result<()> {
    // Validate directory exists
    if !args.dir.exists() {
        anyhow::bail!("Directory does not exist: {}", args.dir.display());
    }
    if !args.dir.is_dir() {
        anyhow::bail!("Path is not a directory: {}", args.dir.display());
    }

    // Compile glob patterns up front
    let filter_pattern = args
        .filter
        .as_deref()
        .map(|p| Pattern::new(p).context("Invalid --filter glob pattern"))
        .transpose()?;

    let exclude_patterns: Vec<Pattern> = args
        .excludes
        .iter()
        .map(|p| Pattern::new(p).context("Invalid --exclude glob pattern"))
        .collect::<Result<Vec<_>>>()?;

    let debounce_duration = Duration::from_millis(args.debounce_ms);

    println!(
        "{} {} {} {}",
        "Watching".green().bold(),
        args.dir.display().to_string().cyan(),
        "for changes...".green().bold(),
        "(Ctrl+C to stop)".dimmed()
    );

    // Build the OSS client
    let auth_client = raps_kernel::auth::AuthClient::new(config.clone());
    let oss_client = OssClient::new(config.clone(), auth_client);

    // Set up a synchronous mpsc channel for notify events
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .context("Failed to create filesystem watcher")?;

    watcher
        .watch(&args.dir, RecursiveMode::Recursive)
        .with_context(|| format!("Failed to watch directory: {}", args.dir.display()))?;

    // Debounce map: file path -> last-event instant
    let mut pending: HashMap<PathBuf, Instant> = HashMap::new();

    loop {
        // Drain all immediately available events
        loop {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(event)) => {
                    let is_relevant = matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_)
                    );
                    if is_relevant {
                        for path in event.paths {
                            if path.is_file() {
                                pending.insert(path, Instant::now());
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("{} {}", "Watch error:".red(), e);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    anyhow::bail!("Filesystem watcher channel closed unexpectedly");
                }
            }
        }

        // Process entries whose debounce window has elapsed
        let now = Instant::now();
        let ready: Vec<PathBuf> = pending
            .iter()
            .filter(|(_, t)| now.duration_since(**t) >= debounce_duration)
            .map(|(p, _)| p.clone())
            .collect();

        for path in ready {
            pending.remove(&path);

            // Apply filter/exclude rules
            if !matches_rules(&path, &filter_pattern, &exclude_patterns) {
                continue;
            }

            // Compute the object key
            let rel = path
                .strip_prefix(&args.dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");

            let object_key = match &args.prefix {
                Some(pfx) => format!("{}/{}", pfx.trim_end_matches('/'), rel),
                None => rel.to_string(),
            };

            let timestamp = Local::now().format("%H:%M:%S").to_string();
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| object_key.clone());

            println!(
                "[{}] {} {} {} {}",
                timestamp.dimmed(),
                "↑".yellow().bold(),
                file_name.cyan(),
                "→".dimmed(),
                "uploading...".yellow()
            );

            match oss_client
                .upload_object(&args.bucket, &object_key, &path)
                .await
            {
                Ok(info) => {
                    println!(
                        "[{}] {} {} ({} bytes)",
                        timestamp.dimmed(),
                        "✓".green().bold(),
                        info.object_key.green(),
                        info.size
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[{}] {} {}: {}",
                        timestamp.dimmed(),
                        "✗".red().bold(),
                        file_name.red(),
                        e
                    );
                }
            }
        }
    }
}

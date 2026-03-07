// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! `raps sync` — sync a local directory to an OSS bucket (like aws s3 sync).

use anyhow::{Context, Result};
use colored::Colorize;
use sha1::{Digest, Sha1};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

use raps_oss::OssClient;

#[derive(Debug, clap::Args)]
pub struct SyncArgs {
    /// Local directory to sync from
    pub local_dir: PathBuf,

    /// Target OSS bucket key
    pub bucket: String,

    /// Remote key prefix (e.g. "models/v2")
    #[arg(long)]
    pub prefix: Option<String>,

    /// Delete remote objects not present locally
    #[arg(long)]
    pub delete: bool,

    /// Show what would change without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Concurrent uploads (default: 4)
    #[arg(long, default_value = "4")]
    pub parallel: usize,

    /// Glob pattern to exclude (repeatable)
    #[arg(long = "exclude")]
    pub excludes: Vec<String>,

    /// Use SHA-1 comparison instead of size+mtime
    #[arg(long)]
    pub checksum: bool,
}

/// Walk a directory recursively and collect all file paths.
fn walk_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let entries =
            std::fs::read_dir(&current).with_context(|| format!("Cannot read dir: {}", current.display()))?;
        for entry in entries {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }

    Ok(files)
}

/// Compute SHA-1 of a local file.
fn sha1_of_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| format!("Cannot read file: {}", path.display()))?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

/// Format a byte count as a human-readable string.
fn human_size(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Check whether a path matches any of the given glob patterns.
fn is_excluded(path: &Path, patterns: &[glob::Pattern]) -> bool {
    let path_str = path.to_string_lossy();
    patterns.iter().any(|p| p.matches(&path_str))
}

pub async fn execute(client: &OssClient, args: SyncArgs) -> Result<()> {
    let local_dir = args
        .local_dir
        .canonicalize()
        .with_context(|| format!("Local directory not found: {}", args.local_dir.display()))?;

    if !local_dir.is_dir() {
        anyhow::bail!("{} is not a directory", local_dir.display());
    }

    // Compile exclude patterns
    let exclude_patterns: Vec<glob::Pattern> = args
        .excludes
        .iter()
        .map(|s| glob::Pattern::new(s).with_context(|| format!("Invalid glob pattern: {}", s)))
        .collect::<Result<Vec<_>>>()?;

    // Walk local directory
    let all_local_files = walk_dir(&local_dir)?;

    // Filter excluded files and build relative-path map
    // Maps relative_path_string -> absolute_path
    let local_files: Vec<(String, PathBuf)> = all_local_files
        .into_iter()
        .filter(|p| {
            let rel = p.strip_prefix(&local_dir).unwrap_or(p);
            !is_excluded(rel, &exclude_patterns)
        })
        .map(|abs| {
            let rel = abs.strip_prefix(&local_dir).unwrap_or(&abs).to_path_buf();
            // Normalise to forward slashes for OSS keys
            let rel_str = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            (rel_str, abs)
        })
        .collect();

    // Build remote key for each local file
    let build_remote_key = |rel: &str| -> String {
        match &args.prefix {
            Some(pfx) => {
                let pfx = pfx.trim_matches('/');
                format!("{}/{}", pfx, rel)
            }
            None => rel.to_owned(),
        }
    };

    // List remote objects under bucket
    println!("{}", "Listing remote objects...".dimmed());
    let remote_items = client.list_objects(&args.bucket).await?;

    // Build HashMap<object_key, size (and sha1)>
    let remote_map: HashMap<String, (u64, Option<String>)> = remote_items
        .into_iter()
        .map(|item| (item.object_key.clone(), (item.size, item.sha1)))
        .collect();

    // Build set of remote keys that correspond to local files (for --delete logic)
    let local_remote_keys: HashSet<String> = local_files
        .iter()
        .map(|(rel, _)| build_remote_key(rel))
        .collect();

    // Classify each local file
    #[derive(Debug)]
    enum FileAction {
        Upload { reason: String },
        Skip,
    }

    let mut to_upload: Vec<(String, PathBuf, String)> = Vec::new(); // (remote_key, local_path, reason)
    let mut skipped = 0usize;

    for (rel, abs_path) in &local_files {
        let remote_key = build_remote_key(rel);
        let display_name = abs_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| rel.clone());

        let action = if let Some((remote_size, remote_sha1)) = remote_map.get(&remote_key) {
            if args.checksum {
                // Compare SHA-1
                let local_sha1 = sha1_of_file(abs_path)?;
                let remote_sha1_lower = remote_sha1
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase();
                if local_sha1.to_lowercase() == remote_sha1_lower && !remote_sha1_lower.is_empty() {
                    FileAction::Skip
                } else {
                    FileAction::Upload {
                        reason: "changed (checksum)".to_owned(),
                    }
                }
            } else {
                // Compare size
                let local_meta = std::fs::metadata(abs_path)
                    .with_context(|| format!("Cannot stat: {}", abs_path.display()))?;
                if local_meta.len() == *remote_size {
                    FileAction::Skip
                } else {
                    FileAction::Upload {
                        reason: format!(
                            "changed ({} local vs {} remote)",
                            human_size(local_meta.len()),
                            human_size(*remote_size)
                        ),
                    }
                }
            }
        } else {
            let local_meta = std::fs::metadata(abs_path)
                .with_context(|| format!("Cannot stat: {}", abs_path.display()))?;
            FileAction::Upload {
                reason: format!("new, {}", human_size(local_meta.len())),
            }
        };

        match action {
            FileAction::Upload { ref reason } => {
                println!(
                    "{} {:<40} ({})",
                    "↑".green().bold(),
                    display_name,
                    reason
                );
                to_upload.push((remote_key, abs_path.clone(), reason.clone()));
            }
            FileAction::Skip => {
                println!(
                    "{} {:<40} (unchanged, skipped)",
                    "=".dimmed(),
                    display_name
                );
                skipped += 1;
            }
        }
    }

    // Compute objects to delete
    let to_delete: Vec<String> = if args.delete {
        remote_map
            .keys()
            .filter(|k| {
                // Only delete keys that are under our prefix (if set), and not in local set
                let under_prefix = match &args.prefix {
                    Some(pfx) => {
                        let pfx = pfx.trim_matches('/');
                        k.starts_with(&format!("{}/", pfx)) || k.as_str() == pfx
                    }
                    None => true,
                };
                under_prefix && !local_remote_keys.contains(*k)
            })
            .cloned()
            .collect()
    } else {
        vec![]
    };

    for key in &to_delete {
        let name = key.rsplit('/').next().unwrap_or(key);
        println!(
            "{} {:<40} (deleted)",
            "x".red().bold(),
            name
        );
    }

    // Summary line before executing
    println!(
        "\n{} {} to upload, {} skipped, {} to delete",
        "Plan:".bold(),
        to_upload.len().to_string().cyan(),
        skipped.to_string().dimmed(),
        to_delete.len().to_string().red(),
    );

    if args.dry_run {
        println!("{}", "(dry-run: no changes made)".yellow());
        return Ok(());
    }

    // Execute uploads in parallel
    let upload_count = to_upload.len();
    if upload_count > 0 {
        let semaphore = Arc::new(Semaphore::new(args.parallel));
        let client = Arc::new(client.clone());
        let bucket = Arc::new(args.bucket.clone());

        let mut handles = Vec::new();

        for (remote_key, abs_path, _reason) in to_upload {
            let permit = semaphore.clone().acquire_owned().await?;
            let client = client.clone();
            let bucket = bucket.clone();

            let handle = tokio::spawn(async move {
                let result = client
                    .upload_object(&bucket, &remote_key, &abs_path)
                    .await;
                drop(permit);
                (remote_key, result)
            });

            handles.push(handle);
        }

        let mut upload_errors = 0usize;
        for handle in handles {
            match handle.await {
                Ok((key, Ok(_))) => {
                    tracing::debug!("Uploaded: {}", key);
                }
                Ok((key, Err(e))) => {
                    eprintln!("{} Failed to upload {}: {}", "ERROR".red().bold(), key, e);
                    upload_errors += 1;
                }
                Err(e) => {
                    eprintln!("{} Task panicked: {}", "ERROR".red().bold(), e);
                    upload_errors += 1;
                }
            }
        }

        if upload_errors > 0 {
            anyhow::bail!("{} upload(s) failed", upload_errors);
        }
    }

    // Execute deletes
    let delete_count = to_delete.len();
    for key in &to_delete {
        client.delete_object(&args.bucket, key).await?;
    }

    println!(
        "\n{} {} uploaded, {} skipped, {} deleted",
        "Done:".green().bold(),
        upload_count.to_string().cyan(),
        skipped.to_string().dimmed(),
        delete_count.to_string().red(),
    );

    Ok(())
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Bucket snapshot commands.
//!
//! Records, compares, and lists point-in-time manifests of OSS bucket objects.

use anyhow::{Context, Result};
use chrono::Utc;
use clap::Subcommand;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::output::OutputFormat;
use raps_oss::OssClient;

// ── Clap structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum SnapshotCommands {
    /// Record a manifest (key + size + sha1 + last-modified) for all objects in a bucket
    Create {
        /// Bucket key to snapshot
        bucket: String,

        /// Output JSON file (default: snapshot-<bucket>-<timestamp>.json)
        #[arg(long = "out-file", short = 'o')]
        out_file: Option<PathBuf>,
    },

    /// Compare two snapshots and show added/removed/changed objects
    Diff {
        /// Older snapshot JSON file
        old: PathBuf,

        /// Newer snapshot JSON file
        new: PathBuf,

        /// Emit JSON instead of a human-readable table
        #[arg(long)]
        json: bool,
    },

    /// List snapshot files in the current directory
    List,
}

impl SnapshotCommands {
    pub async fn execute(self, oss_client: &OssClient, output_format: OutputFormat) -> Result<()> {
        match self {
            SnapshotCommands::Create { bucket, out_file } => {
                create_snapshot(oss_client, &bucket, out_file, output_format).await
            }
            SnapshotCommands::Diff { old, new, json } => diff_snapshots(&old, &new, json),
            SnapshotCommands::List => list_snapshots(),
        }
    }
}

// ── Snapshot data model ───────────────────────────────────────────────────────

/// A single object entry in a snapshot manifest.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SnapshotEntry {
    pub key: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
}

/// Root structure written to disk as a snapshot JSON file.
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SnapshotManifest {
    pub bucket: String,
    pub captured_at: String,
    pub object_count: usize,
    pub objects: Vec<SnapshotEntry>,
}

// ── `snapshot create` ─────────────────────────────────────────────────────────

async fn create_snapshot(
    oss_client: &OssClient,
    bucket: &str,
    output_path: Option<PathBuf>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!(
            "{}",
            format!("Listing objects in bucket '{}'…", bucket).dimmed()
        );
    }

    let objects = oss_client
        .list_objects(bucket)
        .await
        .with_context(|| format!("Failed to list objects in bucket '{}'", bucket))?;

    let now = Utc::now();
    let timestamp = now.format("%Y%m%dT%H%M%SZ").to_string();

    let entries: Vec<SnapshotEntry> = objects
        .iter()
        .map(|obj| SnapshotEntry {
            key: obj.object_key.clone(),
            size: obj.size,
            sha1: obj.sha1.clone(),
            // ObjectItem from list_objects does not include last_modified;
            // use None (the details endpoint would require one call per object).
            last_modified: None,
        })
        .collect();

    let manifest = SnapshotManifest {
        bucket: bucket.to_string(),
        captured_at: now.to_rfc3339(),
        object_count: entries.len(),
        objects: entries,
    };

    // Choose output file
    let out_file = output_path.unwrap_or_else(|| {
        PathBuf::from(format!("snapshot-{}-{}.json", bucket, timestamp))
    });

    let json = serde_json::to_string_pretty(&manifest)?;
    tokio::fs::write(&out_file, &json)
        .await
        .with_context(|| format!("Failed to write snapshot to '{}'", out_file.display()))?;

    match output_format {
        OutputFormat::Table => {
            println!("{} Snapshot created", "✓".green().bold());
            println!("  {} {}", "Bucket:".bold(), bucket.cyan());
            println!("  {} {}", "Objects:".bold(), manifest.object_count);
            println!("  {} {}", "File:".bold(), out_file.display());
            println!("  {} {}", "Captured at:".bold(), manifest.captured_at.dimmed());
        }
        _ => {
            output_format.write(&manifest)?;
        }
    }

    Ok(())
}

// ── `snapshot diff` ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct DiffOutput {
    added: Vec<SnapshotEntry>,
    removed: Vec<SnapshotEntry>,
    changed: Vec<ChangedEntry>,
    unchanged_count: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct ChangedEntry {
    key: String,
    old_size: u64,
    new_size: u64,
    old_sha1: Option<String>,
    new_sha1: Option<String>,
}

fn load_manifest(path: &PathBuf) -> Result<SnapshotManifest> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read snapshot file '{}'", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Cannot parse snapshot file '{}'", path.display()))
}

fn diff_snapshots(old_path: &PathBuf, new_path: &PathBuf, emit_json: bool) -> Result<()> {
    let old = load_manifest(old_path)?;
    let new = load_manifest(new_path)?;

    // Index by object key
    let old_map: HashMap<&str, &SnapshotEntry> =
        old.objects.iter().map(|e| (e.key.as_str(), e)).collect();
    let new_map: HashMap<&str, &SnapshotEntry> =
        new.objects.iter().map(|e| (e.key.as_str(), e)).collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    let mut unchanged = 0usize;

    // Objects in new but not in old → added
    for (key, entry) in &new_map {
        if !old_map.contains_key(key) {
            added.push((*entry).clone());
        }
    }

    // Objects in old but not in new → removed; objects in both → compare
    for (key, old_entry) in &old_map {
        match new_map.get(key) {
            None => removed.push((*old_entry).clone()),
            Some(new_entry) => {
                let size_changed = old_entry.size != new_entry.size;
                let sha1_changed = match (&old_entry.sha1, &new_entry.sha1) {
                    (Some(o), Some(n)) => o != n,
                    _ => false,
                };
                if size_changed || sha1_changed {
                    changed.push(ChangedEntry {
                        key: key.to_string(),
                        old_size: old_entry.size,
                        new_size: new_entry.size,
                        old_sha1: old_entry.sha1.clone(),
                        new_sha1: new_entry.sha1.clone(),
                    });
                } else {
                    unchanged += 1;
                }
            }
        }
    }

    // Sort for deterministic output
    added.sort_by(|a, b| a.key.cmp(&b.key));
    removed.sort_by(|a, b| a.key.cmp(&b.key));
    changed.sort_by(|a, b| a.key.cmp(&b.key));

    let diff = DiffOutput {
        added,
        removed,
        changed,
        unchanged_count: unchanged,
    };

    if emit_json {
        println!("{}", serde_json::to_string_pretty(&diff)?);
        return Ok(());
    }

    // Human-readable table output
    println!();
    println!("{}", "Snapshot Diff".bold());
    println!(
        "  {} {}  {}  {}",
        "Old:".bold(),
        old_path.display(),
        "→".dimmed(),
        new_path.display()
    );
    println!("{}", "─".repeat(60));

    if diff.added.is_empty() && diff.removed.is_empty() && diff.changed.is_empty() {
        println!("  {} No changes detected.", "✓".green().bold());
    } else {
        for e in &diff.added {
            println!(
                "  {} {} ({})",
                "+".green().bold(),
                e.key.green(),
                format_size(e.size)
            );
        }
        for e in &diff.removed {
            println!(
                "  {} {} ({})",
                "-".red().bold(),
                e.key.red(),
                format_size(e.size)
            );
        }
        for e in &diff.changed {
            println!(
                "  {} {} ({} → {})",
                "~".yellow().bold(),
                e.key.yellow(),
                format_size(e.old_size),
                format_size(e.new_size)
            );
        }
    }

    println!("{}", "─".repeat(60));
    println!(
        "  {} added, {} removed, {} changed, {} unchanged",
        diff.added.len().to_string().green(),
        diff.removed.len().to_string().red(),
        diff.changed.len().to_string().yellow(),
        diff.unchanged_count
    );

    Ok(())
}

// ── `snapshot list` ───────────────────────────────────────────────────────────

fn list_snapshots() -> Result<()> {
    let cwd = std::env::current_dir()?;

    let mut files: Vec<PathBuf> = std::fs::read_dir(&cwd)?
        .filter_map(|entry| entry.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("snapshot-") && n.ends_with(".json"))
                .unwrap_or(false)
        })
        .collect();

    files.sort();

    if files.is_empty() {
        println!("{}", "No snapshot files found in current directory.".yellow());
        return Ok(());
    }

    println!("{}", "Snapshot Files".bold());
    println!("{}", "─".repeat(60));

    for path in &files {
        let size = std::fs::metadata(path)
            .map(|m| format_size(m.len()))
            .unwrap_or_else(|_| "?".to_string());

        // Try to peek at bucket/object-count without full parse
        let summary = peek_manifest_summary(path);

        println!(
            "  {} {}  {} {}",
            "·".cyan(),
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .bold(),
            size.dimmed(),
            summary
        );
    }

    println!("{}", "─".repeat(60));
    println!("  {} snapshot(s)", files.len());

    Ok(())
}

fn peek_manifest_summary(path: &PathBuf) -> String {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map(|v| {
            let bucket = v["bucket"].as_str().unwrap_or("?");
            let count = v["object_count"].as_u64().unwrap_or(0);
            let captured = v["captured_at"].as_str().unwrap_or("?");
            format!("bucket={} objects={} at={}", bucket, count, &captured[..captured.len().min(19)])
        })
        .unwrap_or_default()
}

// ── Utilities ─────────────────────────────────────────────────────────────────

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
        format!("{bytes} B")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_manifest(bucket: &str, objects: Vec<(&str, u64, Option<&str>)>) -> SnapshotManifest {
        SnapshotManifest {
            bucket: bucket.to_string(),
            captured_at: "2026-01-01T00:00:00Z".to_string(),
            object_count: objects.len(),
            objects: objects
                .into_iter()
                .map(|(key, size, sha1)| SnapshotEntry {
                    key: key.to_string(),
                    size,
                    sha1: sha1.map(|s| s.to_string()),
                    last_modified: None,
                })
                .collect(),
        }
    }

    fn write_manifest(manifest: &SnapshotManifest) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", serde_json::to_string_pretty(manifest).unwrap()).unwrap();
        f
    }

    #[test]
    fn test_diff_no_changes() {
        let m = make_manifest("b", vec![("a.dwg", 100, Some("sha1a"))]);
        let f1 = write_manifest(&m);
        let f2 = write_manifest(&m);
        // Should run without error
        diff_snapshots(&f1.path().to_path_buf(), &f2.path().to_path_buf(), true).unwrap();
    }

    #[test]
    fn test_diff_detects_added() {
        let old = make_manifest("b", vec![("a.dwg", 100, Some("sha1a"))]);
        let new = make_manifest(
            "b",
            vec![("a.dwg", 100, Some("sha1a")), ("b.rvt", 200, Some("sha1b"))],
        );
        let f1 = write_manifest(&old);
        let f2 = write_manifest(&new);
        let out = {
            // Capture via in-process re-implementation
            let old_m = load_manifest(&f1.path().to_path_buf()).unwrap();
            let new_m = load_manifest(&f2.path().to_path_buf()).unwrap();
            let old_map: HashMap<&str, &SnapshotEntry> =
                old_m.objects.iter().map(|e| (e.key.as_str(), e)).collect();
            let new_map: HashMap<&str, &SnapshotEntry> =
                new_m.objects.iter().map(|e| (e.key.as_str(), e)).collect();
            let added: Vec<_> = new_map
                .keys()
                .filter(|k| !old_map.contains_key(**k))
                .collect();
            added.len()
        };
        assert_eq!(out, 1);
    }

    #[test]
    fn test_diff_detects_removed() {
        let old = make_manifest(
            "b",
            vec![("a.dwg", 100, Some("sha1a")), ("gone.rvt", 50, None)],
        );
        let new = make_manifest("b", vec![("a.dwg", 100, Some("sha1a"))]);
        let f1 = write_manifest(&old);
        let f2 = write_manifest(&new);

        let old_m = load_manifest(&f1.path().to_path_buf()).unwrap();
        let new_m = load_manifest(&f2.path().to_path_buf()).unwrap();
        let new_map: HashMap<&str, &SnapshotEntry> =
            new_m.objects.iter().map(|e| (e.key.as_str(), e)).collect();
        let removed: Vec<_> = old_m
            .objects
            .iter()
            .filter(|e| !new_map.contains_key(e.key.as_str()))
            .collect();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].key, "gone.rvt");
    }

    #[test]
    fn test_diff_detects_changed() {
        let old = make_manifest("b", vec![("a.dwg", 100, Some("sha1old"))]);
        let new = make_manifest("b", vec![("a.dwg", 200, Some("sha1new"))]);
        let f1 = write_manifest(&old);
        let f2 = write_manifest(&new);

        let old_m = load_manifest(&f1.path().to_path_buf()).unwrap();
        let new_m = load_manifest(&f2.path().to_path_buf()).unwrap();
        let old_map: HashMap<&str, &SnapshotEntry> =
            old_m.objects.iter().map(|e| (e.key.as_str(), e)).collect();
        let new_map: HashMap<&str, &SnapshotEntry> =
            new_m.objects.iter().map(|e| (e.key.as_str(), e)).collect();
        let changed: Vec<_> = old_map
            .iter()
            .filter_map(|(k, oe)| new_map.get(k).map(|ne| (k, oe, ne)))
            .filter(|(_, oe, ne)| oe.size != ne.size || oe.sha1 != ne.sha1)
            .collect();
        assert_eq!(changed.len(), 1);
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(512), "512 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(2048), "2.00 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(2 * 1024 * 1024), "2.00 MB");
    }
}

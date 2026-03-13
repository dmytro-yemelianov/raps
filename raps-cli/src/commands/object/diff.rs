// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Diff command — compare two OSS objects or an OSS object against a local file.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;
use sha1::{Digest, Sha1};
use sha2::Sha256;
use similar::{ChangeTag, TextDiff};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

use crate::output::OutputFormat;
use raps_oss::OssClient;

use super::format_size;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Compute SHA-1 and SHA-256 of a byte slice.
fn compute_hashes(data: &[u8]) -> (String, String) {
    let sha1 = hex::encode(Sha1::digest(data));
    let sha256 = hex::encode(Sha256::digest(data));
    (sha1, sha256)
}

/// Return true when the byte slice looks like UTF-8 text (no lone NUL bytes).
fn is_text(data: &[u8]) -> bool {
    // Quick heuristic: if we can decode the first 8 KiB as valid UTF-8 and
    // there are no embedded NUL bytes, treat it as text.
    let sample = &data[..data.len().min(8192)];
    !sample.contains(&0u8) && std::str::from_utf8(sample).is_ok()
}

// ── per-side descriptor ───────────────────────────────────────────────────────

struct Side {
    label: String,
    data: Vec<u8>,
    sha1: String,
    sha256: String,
}

impl Side {
    fn new(label: impl Into<String>, data: Vec<u8>) -> Self {
        let (sha1, sha256) = compute_hashes(&data);
        Self {
            label: label.into(),
            data,
            sha1,
            sha256,
        }
    }

    fn size(&self) -> u64 {
        self.data.len() as u64
    }
}

// ── output types ─────────────────────────────────────────────────────────────

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct DiffOutput {
    pub left_label: String,
    pub right_label: String,
    pub left_size: u64,
    pub right_size: u64,
    pub left_sha1: String,
    pub right_sha1: String,
    pub left_sha256: String,
    pub right_sha256: String,
    pub identical: bool,
    pub is_text: bool,
    /// Present only when `--stat` or `--checksum-only` is not set and files
    /// differ and are text files.
    pub diff: Option<String>,
    /// Number of changed lines (text files only, stat mode)
    pub changed_lines: Option<usize>,
    /// Number of unchanged lines (text files only, stat mode)
    pub unchanged_lines: Option<usize>,
}

// ── main entry point ──────────────────────────────────────────────────────────

/// Download an OSS object into a temp file and return (label, bytes).
async fn fetch_oss(client: &OssClient, bucket_key: &str, object_key: &str) -> Result<Side> {
    let tmp = NamedTempFile::new().context("Failed to create temp file")?;
    client
        .download_object(bucket_key, object_key, tmp.path())
        .await
        .with_context(|| {
            format!(
                "Failed to download OSS object '{}/{}'",
                bucket_key, object_key
            )
        })?;
    let data = tokio::fs::read(tmp.path())
        .await
        .context("Failed to read downloaded object")?;
    Ok(Side::new(
        format!("oss:{}/{}", bucket_key, object_key),
        data,
    ))
}

/// Read a local file.
async fn fetch_local(path: &Path) -> Result<Side> {
    let data = tokio::fs::read(path)
        .await
        .with_context(|| format!("Failed to read local file '{}'", path.display()))?;
    Ok(Side::new(format!("local:{}", path.display()), data))
}

/// Return true when the string looks like an OSS object key, i.e. it contains
/// a `/` and does NOT start with `.` or `/` relative filesystem indicators,
/// and does not refer to an existing local path.
fn looks_like_oss_key(s: &str) -> bool {
    // If the path actually exists on disk, treat it as local.
    if Path::new(s).exists() {
        return false;
    }
    // OSS bucket keys are never absolute filesystem paths.
    if s.starts_with('/') || s.starts_with('.') {
        return false;
    }
    // OSS object keys always have at least one `/` separating bucket from key.
    s.contains('/')
}

/// Split `"bucket/object/key"` → `("bucket", "object/key")`.
fn split_bucket_object(s: &str) -> Result<(&str, &str)> {
    s.split_once('/')
        .ok_or_else(|| anyhow::anyhow!("Expected '<bucket>/<object-key>', got '{}'", s))
}

// ── unified diff helpers ──────────────────────────────────────────────────────

fn build_unified_diff(left: &Side, right: &Side) -> (String, usize, usize) {
    let left_text = String::from_utf8_lossy(&left.data);
    let right_text = String::from_utf8_lossy(&right.data);

    let diff = TextDiff::from_lines(left_text.as_ref(), right_text.as_ref());

    let mut out = String::new();
    let mut changed = 0usize;
    let mut unchanged = 0usize;

    // Header lines
    out.push_str(&format!("--- {}\n", left.label));
    out.push_str(&format!("+++ {}\n", right.label));

    for group in diff.grouped_ops(3) {
        for op in &group {
            for change in diff.iter_changes(op) {
                match change.tag() {
                    ChangeTag::Equal => {
                        unchanged += 1;
                        out.push(' ');
                    }
                    ChangeTag::Delete => {
                        changed += 1;
                        out.push('-');
                    }
                    ChangeTag::Insert => {
                        changed += 1;
                        out.push('+');
                    }
                }
                out.push_str(change.value());
                if change.missing_newline() {
                    out.push('\n');
                }
            }
        }
    }

    (out, changed, unchanged)
}

// ── command handler ───────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(super) async fn diff_objects(
    client: &OssClient,
    left_arg: String,
    right_arg: String,
    stat: bool,
    checksum_only: bool,
    output_format: OutputFormat,
) -> Result<()> {
    // Resolve left side (always OSS key)
    let left_side = if looks_like_oss_key(&left_arg) {
        let (bucket, object) = split_bucket_object(&left_arg)?;
        if output_format.supports_colors() {
            println!(
                "{}",
                format!("Fetching OSS object '{}'...", left_arg).dimmed()
            );
        }
        fetch_oss(client, bucket, object).await?
    } else {
        // Treat as local path
        fetch_local(Path::new(&left_arg)).await?
    };

    // Resolve right side (OSS key or local path)
    let right_side = if looks_like_oss_key(&right_arg) {
        let (bucket, object) = split_bucket_object(&right_arg)?;
        if output_format.supports_colors() {
            println!(
                "{}",
                format!("Fetching OSS object '{}'...", right_arg).dimmed()
            );
        }
        fetch_oss(client, bucket, object).await?
    } else {
        let path = PathBuf::from(&right_arg);
        fetch_local(&path).await?
    };

    // Checksums & identity
    let identical = left_side.sha1 == right_side.sha1;
    let text = is_text(&left_side.data) && is_text(&right_side.data);

    // Build diff output struct
    let (diff_text, changed_lines, unchanged_lines) =
        if !checksum_only && !stat && !identical && text {
            let (d, c, u) = build_unified_diff(&left_side, &right_side);
            (Some(d), Some(c), Some(u))
        } else if (stat) && !identical && text {
            // Stat mode: compute counts but don't emit full diff
            let (_, c, u) = build_unified_diff(&left_side, &right_side);
            (None, Some(c), Some(u))
        } else {
            (None, None, None)
        };

    let output = DiffOutput {
        left_label: left_side.label.clone(),
        right_label: right_side.label.clone(),
        left_size: left_side.size(),
        right_size: right_side.size(),
        left_sha1: left_side.sha1.clone(),
        right_sha1: right_side.sha1.clone(),
        left_sha256: left_side.sha256.clone(),
        right_sha256: right_side.sha256.clone(),
        identical,
        is_text: text,
        diff: diff_text,
        changed_lines,
        unchanged_lines,
    };

    match output_format {
        OutputFormat::Table => render_table(&output, stat, checksum_only),
        _ => output_format.write(&output)?,
    }

    // Exit with non-zero status when files differ (similar to `diff(1)`)
    if !output.identical {
        std::process::exit(1);
    }

    Ok(())
}

// ── table renderer ────────────────────────────────────────────────────────────

fn render_table(output: &DiffOutput, stat: bool, checksum_only: bool) {
    println!("\n{}", "Object Diff".bold().underline());
    println!("{}", "-".repeat(70));
    println!("  {} {}", "Left: ".bold(), output.left_label.cyan());
    println!("  {} {}", "Right:".bold(), output.right_label.cyan());

    println!("\n{}", "Sizes".bold());
    let left_size_str = format_size(output.left_size);
    let right_size_str = format_size(output.right_size);
    if output.left_size == output.right_size {
        println!(
            "  {} {} = {}",
            "Size:".bold(),
            left_size_str,
            right_size_str
        );
    } else {
        println!(
            "  {} {} {} {}",
            "Left: ".bold(),
            left_size_str,
            "->".dimmed(),
            right_size_str
        );
        let diff_bytes = output.right_size as i64 - output.left_size as i64;
        let sign = if diff_bytes >= 0 { "+" } else { "" };
        println!("  {} {}{} bytes", "Delta:".bold(), sign, diff_bytes);
    }

    println!("\n{}", "Checksums".bold());
    let sha1_match = output.left_sha1 == output.right_sha1;
    let sha256_match = output.left_sha256 == output.right_sha256;

    let sha1_indicator = if sha1_match {
        "match".green().bold()
    } else {
        "MISMATCH".red().bold()
    };
    let sha256_indicator = if sha256_match {
        "match".green().bold()
    } else {
        "MISMATCH".red().bold()
    };

    println!(
        "  {} {} [{}]",
        "SHA-1:  ".bold(),
        output.left_sha1.dimmed(),
        sha1_indicator
    );
    if !sha1_match {
        println!("          {}", output.right_sha1.dimmed());
    }
    println!(
        "  {} {} [{}]",
        "SHA-256:".bold(),
        output.left_sha256.dimmed(),
        sha256_indicator
    );
    if !sha256_match {
        println!("          {}", output.right_sha256.dimmed());
    }

    // Summary
    println!("\n{}", "Summary".bold());
    if output.identical {
        println!(
            "  {} Files are {}",
            "=".green().bold(),
            "identical".green().bold()
        );
    } else {
        println!(
            "  {} Files are {}",
            "!".red().bold(),
            "different".red().bold()
        );

        if !checksum_only {
            if output.is_text {
                if stat || output.diff.is_none() {
                    // stat mode: show line counts
                    if let (Some(changed), Some(unchanged)) =
                        (output.changed_lines, output.unchanged_lines)
                    {
                        println!(
                            "  {} changed lines: {}, unchanged lines: {}",
                            "~".yellow().bold(),
                            changed.to_string().yellow(),
                            unchanged
                        );
                    }
                }
            } else {
                println!(
                    "  {} Binary files — content diff not shown",
                    "~".yellow().bold()
                );
            }
        }
    }

    // Full diff output (non-stat, non-checksum-only, text files)
    if let Some(ref diff) = output.diff {
        println!("\n{}", "Diff".bold().underline());
        for line in diff.lines() {
            if let Some(rest) = line.strip_prefix('+') {
                println!("{}", format!("+{}", rest).green());
            } else if let Some(rest) = line.strip_prefix('-') {
                println!("{}", format!("-{}", rest).red());
            } else {
                println!("{}", line.dimmed());
            }
        }
    }

    println!("{}", "-".repeat(70));
}

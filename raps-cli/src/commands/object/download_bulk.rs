// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Parallel bulk download command for OSS objects.

use anyhow::Result;
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use raps_kernel::interactive;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::output::OutputFormat;
use raps_oss::OssClient;

use super::{format_size, select_bucket};

// ──────────────────────────── output types ───────────────────────────────────

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct BulkDownloadFileResult {
    object_key: String,
    output_path: String,
    size: Option<u64>,
    skipped: bool,
    success: bool,
    error: Option<String>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct BulkDownloadSummary {
    success: bool,
    downloaded: usize,
    skipped: usize,
    failed: usize,
    total_bytes: u64,
    total_bytes_human: String,
    elapsed_secs: f64,
    files: Vec<BulkDownloadFileResult>,
}

// ──────────────────────────── main function ──────────────────────────────────

pub(super) async fn download_bulk(
    client: &OssClient,
    bucket: Option<String>,
    prefix: String,
    output_dir: PathBuf,
    concurrency: usize,
    skip_existing: bool,
    flat: bool,
    output_format: OutputFormat,
) -> Result<()> {
    let bucket_key = select_bucket(client, bucket).await?;

    // Ensure the output directory exists
    tokio::fs::create_dir_all(&output_dir).await.map_err(|e| {
        anyhow::anyhow!(
            "Failed to create output directory '{}': {}",
            output_dir.display(),
            e
        )
    })?;

    if output_format.supports_colors() {
        println!(
            "{} objects matching '{}' from bucket '{}' …",
            "Listing".dimmed(),
            prefix.cyan(),
            bucket_key.cyan()
        );
    }

    // List all objects and filter by prefix
    let all_objects = client.list_objects(&bucket_key).await?;
    let objects: Vec<_> = all_objects
        .into_iter()
        .filter(|o| o.object_key.starts_with(&prefix))
        .collect();

    if objects.is_empty() {
        if output_format.supports_colors() {
            println!(
                "{}",
                format!("No objects found matching prefix '{prefix}'").yellow()
            );
        }
        return Ok(());
    }

    if output_format.supports_colors() {
        println!(
            "{} {} objects to download with concurrency {}",
            "Found".dimmed(),
            objects.len().to_string().cyan(),
            concurrency.to_string().cyan()
        );
    }

    // Build per-object destination paths
    struct DownloadTask {
        object_key: String,
        size: u64,
        dest: PathBuf,
    }

    let tasks: Vec<DownloadTask> = objects
        .iter()
        .map(|obj| {
            let dest = if flat {
                // Place all files directly in output_dir, using only the filename portion
                let filename = Path::new(&obj.object_key)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| obj.object_key.replace('/', "_"));
                output_dir.join(filename)
            } else {
                // Preserve the key structure relative to the prefix
                let relative = obj
                    .object_key
                    .strip_prefix(&prefix)
                    .unwrap_or(&obj.object_key)
                    .trim_start_matches('/');
                output_dir.join(relative)
            };
            DownloadTask {
                object_key: obj.object_key.clone(),
                size: obj.size,
                dest,
            }
        })
        .collect();

    // ── multi-progress bars ───────────────────────────────────────────────────
    let use_progress = !interactive::is_non_interactive();
    let mp = Arc::new(MultiProgress::new());

    // Overall progress bar: counts completed objects
    let overall_pb = if use_progress {
        let pb = mp.add(ProgressBar::new(tasks.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} Overall [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}",
                )
                .expect("hardcoded template is valid")
                .progress_chars("█▓░"),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb.set_message("downloading…");
        pb
    } else {
        ProgressBar::hidden()
    };

    // ── spawn parallel tasks ─────────────────────────────────────────────────
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let client_arc = Arc::new(client.clone());
    let bucket_arc = Arc::new(bucket_key.clone());
    let overall_pb_arc = Arc::new(overall_pb);
    let mp_arc = mp.clone();

    let start = Instant::now();
    let mut join_set: JoinSet<BulkDownloadFileResult> = JoinSet::new();

    for task in tasks {
        let permit = semaphore.clone().acquire_owned().await?;
        let client = client_arc.clone();
        let bucket = bucket_arc.clone();
        let overall_pb = overall_pb_arc.clone();
        let mp = mp_arc.clone();
        let use_pb = use_progress;

        join_set.spawn(async move {
            let object_key = task.object_key.clone();
            let dest = task.dest.clone();
            let size = task.size;

            // Skip check: file already present with matching size
            if skip_existing {
                if let Ok(meta) = tokio::fs::metadata(&dest).await {
                    if meta.len() == size {
                        drop(permit);
                        overall_pb.inc(1);
                        return BulkDownloadFileResult {
                            object_key,
                            output_path: dest.display().to_string(),
                            size: Some(size),
                            skipped: true,
                            success: true,
                            error: None,
                        };
                    }
                }
            }

            // Ensure parent directory exists (for non-flat mode)
            if let Some(parent) = dest.parent() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    drop(permit);
                    overall_pb.inc(1);
                    return BulkDownloadFileResult {
                        object_key,
                        output_path: dest.display().to_string(),
                        size: None,
                        skipped: false,
                        success: false,
                        error: Some(format!("Failed to create directory: {e}")),
                    };
                }
            }

            // Per-file progress bar
            let file_pb: Option<ProgressBar> = if use_pb {
                let pb = mp.add(ProgressBar::new(size));
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template(
                            "  {msg:.cyan} [{bar:35.cyan/blue}] {bytes}/{total_bytes} ({percent}%)",
                        )
                        .expect("hardcoded template is valid")
                        .progress_chars("█▓░"),
                );
                let short = if object_key.len() > 30 {
                    format!("…{}", &object_key[object_key.len() - 29..])
                } else {
                    object_key.clone()
                };
                pb.set_message(short);
                Some(pb)
            } else {
                None
            };

            // Perform the download using the OSS client
            let result = download_with_progress(&client, &bucket, &object_key, &dest, file_pb.as_ref()).await;

            if let Some(pb) = file_pb {
                pb.finish_and_clear();
            }
            drop(permit);
            overall_pb.inc(1);

            match result {
                Ok(()) => BulkDownloadFileResult {
                    object_key,
                    output_path: dest.display().to_string(),
                    size: Some(size),
                    skipped: false,
                    success: true,
                    error: None,
                },
                Err(e) => BulkDownloadFileResult {
                    object_key,
                    output_path: dest.display().to_string(),
                    size: None,
                    skipped: false,
                    success: false,
                    error: Some(e.to_string()),
                },
            }
        });
    }

    // Collect results
    let mut file_results: Vec<BulkDownloadFileResult> = Vec::new();
    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(r) => file_results.push(r),
            Err(e) => file_results.push(BulkDownloadFileResult {
                object_key: "unknown".to_string(),
                output_path: String::new(),
                size: None,
                skipped: false,
                success: false,
                error: Some(format!("Task panicked: {e}")),
            }),
        }
    }

    // Finish the overall bar
    overall_pb_arc.finish_with_message("done");

    let elapsed = start.elapsed();

    // Compute summary
    let downloaded = file_results.iter().filter(|r| r.success && !r.skipped).count();
    let skipped = file_results.iter().filter(|r| r.skipped).count();
    let failed = file_results.iter().filter(|r| !r.success).count();
    let total_bytes: u64 = file_results
        .iter()
        .filter(|r| r.success && !r.skipped)
        .filter_map(|r| r.size)
        .sum();

    let summary = BulkDownloadSummary {
        success: failed == 0,
        downloaded,
        skipped,
        failed,
        total_bytes,
        total_bytes_human: format_size(total_bytes),
        elapsed_secs: elapsed.as_secs_f64(),
        files: file_results,
    };

    // Output
    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Bulk Download Summary:".bold());
            println!("{}", "-".repeat(70));

            for f in &summary.files {
                if f.skipped {
                    println!(
                        "  {} {} {}",
                        "~".yellow().bold(),
                        f.object_key.dimmed(),
                        "(skipped — already exists)".dimmed()
                    );
                } else if f.success {
                    let sz = f.size.map(format_size).unwrap_or_default();
                    println!(
                        "  {} {} {}",
                        "✓".green().bold(),
                        f.object_key.cyan(),
                        sz.dimmed()
                    );
                } else {
                    println!(
                        "  {} {} {}",
                        "X".red().bold(),
                        f.object_key,
                        f.error.as_deref().unwrap_or("unknown error").red()
                    );
                }
            }

            println!("{}", "-".repeat(70));
            println!(
                "  {} {} downloaded, {} skipped, {} failed",
                "Total:".bold(),
                downloaded.to_string().green(),
                skipped.to_string().yellow(),
                failed.to_string().red()
            );
            println!(
                "  {} {} in {:.1}s",
                "Size:".bold(),
                summary.total_bytes_human,
                elapsed.as_secs_f64()
            );

            if failed > 0 {
                println!(
                    "\n  {} {} file(s) failed. Re-run with {} to skip already-downloaded files.",
                    "Hint:".yellow().bold(),
                    failed,
                    "--skip-existing".cyan()
                );
            }
        }
        _ => {
            output_format.write(&summary)?;
        }
    }

    if summary.failed > 0 {
        anyhow::bail!("{} file(s) failed to download", summary.failed);
    }

    Ok(())
}

// ──────────────────────────── helpers ────────────────────────────────────────

/// Download a single object, updating an optional per-file progress bar.
///
/// We use the public `download_object` API which handles all the retry/redirect
/// logic internally. The per-file progress bar is updated by wrapping the dest
/// path in a temporary file and tracking bytes written.
async fn download_with_progress(
    client: &OssClient,
    bucket_key: &str,
    object_key: &str,
    dest: &Path,
    file_pb: Option<&ProgressBar>,
) -> Result<()> {
    // When we have a progress bar we do the streaming manually so we can
    // call set_position. When no progress bar is needed (non-interactive)
    // we delegate to the existing client method which handles retries etc.
    if let Some(pb) = file_pb {
        download_streaming(client, bucket_key, object_key, dest, pb).await
    } else {
        client.download_object(bucket_key, object_key, dest).await
    }
}

/// Stream-download with manual progress reporting.
async fn download_streaming(
    client: &OssClient,
    bucket_key: &str,
    object_key: &str,
    dest: &Path,
    pb: &ProgressBar,
) -> Result<()> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let signed = client
        .get_signed_download_url(bucket_key, object_key, None)
        .await?;

    let download_url = signed
        .url
        .ok_or_else(|| anyhow::anyhow!("No download URL returned for '{object_key}'"))?;

    // Build a plain reqwest client for the S3 direct download (no auth needed)
    let response = reqwest::Client::new()
        .get(&download_url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start download for '{object_key}': {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Download failed for '{object_key}': HTTP {status} — {body}"
        ));
    }

    if let Some(len) = response.content_length() {
        pb.set_length(len);
    }

    let mut file = tokio::fs::File::create(dest).await.map_err(|e| {
        anyhow::anyhow!("Failed to create '{}': {}", dest.display(), e)
    })?;

    let mut stream = response.bytes_stream();
    let mut written: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|e| anyhow::anyhow!("Download error for '{object_key}': {e}"))?;
        file.write_all(&chunk).await.map_err(|e| {
            anyhow::anyhow!("Write error for '{}': {}", dest.display(), e)
        })?;
        written += chunk.len() as u64;
        pb.set_position(written);
    }

    file.flush().await?;
    Ok(())
}

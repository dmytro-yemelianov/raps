// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Upload commands for single and batch file uploads to OSS buckets.

use anyhow::{Context, Result};
use colored::Colorize;
use futures_util::future;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::output::OutputFormat;
use raps_oss::OssClient;

use super::{format_size, select_bucket};

#[derive(Serialize)]
struct UploadOutput {
    success: bool,
    object_id: String,
    bucket_key: String,
    object_key: String,
    size: u64,
    size_human: String,
    sha1: Option<String>,
    urn: String,
}

pub(super) async fn upload_object(
    client: &OssClient,
    bucket: Option<String>,
    file: PathBuf,
    key: Option<String>,
    resume: bool,
    output_format: OutputFormat,
) -> Result<()> {
    let from_stdin = file.as_os_str() == "-";

    // Spool stdin to a temp file when `-` is given, otherwise validate the path
    let (_tmp_guard, actual_file) = if from_stdin {
        if resume {
            anyhow::bail!("--resume cannot be used when reading from stdin");
        }
        if key.is_none() {
            anyhow::bail!("--key is required when reading from stdin (no filename to derive)");
        }
        let mut tmp =
            tempfile::NamedTempFile::new().context("Failed to create temp file for stdin spool")?;
        std::io::copy(&mut std::io::stdin().lock(), &mut tmp).context("Failed to read stdin")?;
        let path = tmp.path().to_path_buf();
        (Some(tmp), path)
    } else {
        if !file.exists() {
            anyhow::bail!("File not found: {}", file.display());
        }
        (None, file.clone())
    };

    // Select bucket
    let bucket_key = select_bucket(client, bucket).await?;

    // Determine object key
    let object_key = key.unwrap_or_else(|| {
        file.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string()
    });

    if output_format.supports_colors() {
        let source = if from_stdin {
            "stdin"
        } else {
            &file.display().to_string()
        };
        let resume_msg = if resume { " (with resume)" } else { "" };
        println!(
            "{} {} {} {}{}",
            "Uploading".dimmed(),
            source.cyan(),
            "to".dimmed(),
            format!("{}/{}", bucket_key, object_key).cyan(),
            resume_msg.dimmed()
        );
    }

    let object_info = client
        .upload_object_with_options(&bucket_key, &object_key, &actual_file, resume)
        .await?;

    let urn = client.get_urn(&bucket_key, &object_key);
    let output = UploadOutput {
        success: true,
        object_id: object_info.object_id.clone(),
        bucket_key: bucket_key.clone(),
        object_key: object_key.clone(),
        size: object_info.size,
        size_human: format_size(object_info.size),
        sha1: object_info.sha1.clone(),
        urn: urn.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Upload complete!", "✓".green().bold());
            println!("  {} {}", "Object ID:".bold(), output.object_id);
            println!("  {} {}", "Size:".bold(), output.size_human);
            if let Some(ref sha1) = output.sha1 {
                println!("  {} {}", "SHA1:".bold(), sha1.dimmed());
            }
            println!(
                "\n  {} {}",
                "URN (for translation):".bold().yellow(),
                output.urn
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

// ============== BATCH UPLOAD STATE ==============

/// Persistent state for a batch upload operation
#[derive(Serialize, Deserialize)]
struct BatchUploadState {
    bucket_key: String,
    files: Vec<BatchFileState>,
    started_at: String,
}

/// State of a single file in a batch upload
#[derive(Serialize, Deserialize)]
struct BatchFileState {
    path: String,
    status: BatchFileStatus,
    error: Option<String>,
}

/// Status of a file in a batch upload
#[derive(Serialize, Deserialize, PartialEq)]
enum BatchFileStatus {
    Pending,
    Completed,
    Failed,
}

/// Get the path to the batch upload state file
fn batch_state_path() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
        .ok_or_else(|| anyhow::anyhow!("Failed to determine project directories"))?;
    Ok(proj_dirs.data_dir().join("batch_upload_state.json"))
}

/// Save batch upload state to disk
fn save_batch_state(state: &BatchUploadState) -> Result<()> {
    let path = batch_state_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Load batch upload state from disk
fn load_batch_state() -> Result<Option<BatchUploadState>> {
    let path = batch_state_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let state: BatchUploadState = serde_json::from_str(&content)?;
    Ok(Some(state))
}

/// Remove batch upload state file
fn clear_batch_state() -> Result<()> {
    let path = batch_state_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

// ============== PARALLEL UPLOADS ==============

#[derive(Serialize)]
struct BatchUploadResult {
    success: bool,
    uploaded: usize,
    failed: usize,
    total_size: u64,
    files: Vec<BatchFileResult>,
}

#[derive(Serialize)]
struct BatchFileResult {
    name: String,
    success: bool,
    size: Option<u64>,
    error: Option<String>,
}

pub(super) async fn upload_batch(
    client: &OssClient,
    bucket: Option<String>,
    files: Vec<PathBuf>,
    parallel: usize,
    resume: bool,
    output_format: OutputFormat,
) -> Result<()> {
    // If resume flag, try to load previous state
    if resume {
        if let Some(saved_state) = load_batch_state()? {
            let pending_count = saved_state
                .files
                .iter()
                .filter(|f| f.status != BatchFileStatus::Completed)
                .count();

            if pending_count == 0 {
                if output_format.supports_colors() {
                    println!(
                        "{} Previous batch upload already completed!",
                        "✓".green().bold()
                    );
                }
                clear_batch_state()?;
                return Ok(());
            }

            if output_format.supports_colors() {
                println!(
                    "{} Resuming batch upload: {}/{} files remaining",
                    "→".cyan(),
                    pending_count,
                    saved_state.files.len()
                );
            }

            return resume_batch_upload(client, saved_state, parallel, output_format).await;
        } else {
            anyhow::bail!(
                "No previous batch upload state found. Start a new upload without --resume."
            );
        }
    }

    if files.is_empty() {
        anyhow::bail!("No files specified for upload");
    }

    // Validate all files exist
    for file in &files {
        if !file.exists() {
            anyhow::bail!("File not found: {}", file.display());
        }
    }

    // Select bucket
    let bucket_key = select_bucket(client, bucket).await?;

    // Create initial state
    let mut state = BatchUploadState {
        bucket_key: bucket_key.clone(),
        files: files
            .iter()
            .map(|f| BatchFileState {
                path: f.display().to_string(),
                status: BatchFileStatus::Pending,
                error: None,
            })
            .collect(),
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    save_batch_state(&state)?;

    if output_format.supports_colors() {
        println!(
            "{} {} files to bucket '{}' with {} parallel uploads",
            "Uploading".dimmed(),
            files.len().to_string().cyan(),
            bucket_key.cyan(),
            parallel.to_string().cyan()
        );
    }

    let semaphore = Arc::new(Semaphore::new(parallel));
    let client = Arc::new(client.clone());
    let bucket_key = Arc::new(bucket_key);

    let mut handles = Vec::new();

    for (idx, file) in files.into_iter().enumerate() {
        let permit = semaphore.clone().acquire_owned().await?;
        let client = client.clone();
        let bucket = bucket_key.clone();
        let file_path = file.clone();

        let handle = tokio::spawn(async move {
            let object_key = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unnamed")
                .to_string();

            let result = client.upload_object(&bucket, &object_key, &file_path).await;

            drop(permit); // Release permit

            (idx, file_path, object_key, result)
        });

        handles.push(handle);
    }

    // Wait for all uploads
    let results = future::join_all(handles).await;

    let mut batch_result = BatchUploadResult {
        success: true,
        uploaded: 0,
        failed: 0,
        total_size: 0,
        files: Vec::new(),
    };

    for result in results {
        match result {
            Ok((idx, file_path, _object_key, upload_result)) => {
                let file_name = file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                match upload_result {
                    Ok(info) => {
                        batch_result.uploaded += 1;
                        batch_result.total_size += info.size;
                        batch_result.files.push(BatchFileResult {
                            name: file_name,
                            success: true,
                            size: Some(info.size),
                            error: None,
                        });
                        state.files[idx].status = BatchFileStatus::Completed;
                        state.files[idx].error = None;
                    }
                    Err(e) => {
                        batch_result.failed += 1;
                        batch_result.success = false;
                        batch_result.files.push(BatchFileResult {
                            name: file_name,
                            success: false,
                            size: None,
                            error: Some(e.to_string()),
                        });
                        state.files[idx].status = BatchFileStatus::Failed;
                        state.files[idx].error = Some(e.to_string());
                    }
                }
                save_batch_state(&state)?;
            }
            Err(e) => {
                batch_result.failed += 1;
                batch_result.success = false;
                batch_result.files.push(BatchFileResult {
                    name: "unknown".to_string(),
                    success: false,
                    size: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Clear state on full success
    if batch_result.failed == 0 {
        clear_batch_state()?;
    } else {
        // Save final state so it can be resumed
        save_batch_state(&state)?;
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Batch Upload Summary:".bold());
            println!("{}", "-".repeat(60));

            for file in &batch_result.files {
                if file.success {
                    let size = file.size.map(format_size).unwrap_or_default();
                    println!(
                        "  {} {} {}",
                        "✓".green().bold(),
                        file.name.cyan(),
                        size.dimmed()
                    );
                } else {
                    println!(
                        "  {} {} {}",
                        "X".red().bold(),
                        file.name,
                        file.error.as_deref().unwrap_or("Unknown error").red()
                    );
                }
            }

            println!("{}", "-".repeat(60));
            println!(
                "  {} {} uploaded, {} failed",
                "Total:".bold(),
                batch_result.uploaded.to_string().green(),
                batch_result.failed.to_string().red()
            );
            println!(
                "  {} {}",
                "Size:".bold(),
                format_size(batch_result.total_size)
            );

            if batch_result.failed > 0 {
                println!(
                    "\n  {} Use {} to retry failed files.",
                    "Hint:".yellow().bold(),
                    "--resume".cyan()
                );
            }
        }
        _ => {
            output_format.write(&batch_result)?;
        }
    }

    if batch_result.failed > 0 {
        anyhow::bail!("{} file(s) failed to upload", batch_result.failed);
    }

    Ok(())
}

/// Resume a previously interrupted batch upload
async fn resume_batch_upload(
    client: &OssClient,
    mut state: BatchUploadState,
    parallel: usize,
    output_format: OutputFormat,
) -> Result<()> {
    // Collect indices of files that need uploading (Pending or Failed)
    let pending_indices: Vec<usize> = state
        .files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.status != BatchFileStatus::Completed)
        .map(|(i, _)| i)
        .collect();

    if output_format.supports_colors() {
        println!(
            "{} {} files to bucket '{}' with {} parallel uploads",
            "Uploading".dimmed(),
            pending_indices.len().to_string().cyan(),
            state.bucket_key.cyan(),
            parallel.to_string().cyan()
        );
    }

    let semaphore = Arc::new(Semaphore::new(parallel));
    let client = Arc::new(client.clone());
    let bucket_key = Arc::new(state.bucket_key.clone());

    let mut handles = Vec::new();

    for idx in &pending_indices {
        let file_path = PathBuf::from(&state.files[*idx].path);

        if !file_path.exists() {
            state.files[*idx].status = BatchFileStatus::Failed;
            state.files[*idx].error = Some(format!("File not found: {}", file_path.display()));
            save_batch_state(&state)?;
            continue;
        }

        let permit = semaphore.clone().acquire_owned().await?;
        let client = client.clone();
        let bucket = bucket_key.clone();
        let file_idx = *idx;

        let handle = tokio::spawn(async move {
            let object_key = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unnamed")
                .to_string();

            let result = client.upload_object(&bucket, &object_key, &file_path).await;

            drop(permit); // Release permit

            (file_idx, file_path, object_key, result)
        });

        handles.push(handle);
    }

    // Wait for all uploads
    let results = future::join_all(handles).await;

    let mut batch_result = BatchUploadResult {
        success: true,
        uploaded: 0,
        failed: 0,
        total_size: 0,
        files: Vec::new(),
    };

    // Count previously completed files
    let previously_completed = state
        .files
        .iter()
        .filter(|f| f.status == BatchFileStatus::Completed)
        .count();

    for result in results {
        match result {
            Ok((idx, file_path, _object_key, upload_result)) => {
                let file_name = file_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                match upload_result {
                    Ok(info) => {
                        batch_result.uploaded += 1;
                        batch_result.total_size += info.size;
                        batch_result.files.push(BatchFileResult {
                            name: file_name,
                            success: true,
                            size: Some(info.size),
                            error: None,
                        });
                        state.files[idx].status = BatchFileStatus::Completed;
                        state.files[idx].error = None;
                    }
                    Err(e) => {
                        batch_result.failed += 1;
                        batch_result.success = false;
                        batch_result.files.push(BatchFileResult {
                            name: file_name,
                            success: false,
                            size: None,
                            error: Some(e.to_string()),
                        });
                        state.files[idx].status = BatchFileStatus::Failed;
                        state.files[idx].error = Some(e.to_string());
                    }
                }
                save_batch_state(&state)?;
            }
            Err(e) => {
                batch_result.failed += 1;
                batch_result.success = false;
                batch_result.files.push(BatchFileResult {
                    name: "unknown".to_string(),
                    success: false,
                    size: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    let total_failed = state
        .files
        .iter()
        .filter(|f| f.status == BatchFileStatus::Failed)
        .count();

    // Clear state on full success
    let all_done = state
        .files
        .iter()
        .all(|f| f.status == BatchFileStatus::Completed);
    if all_done {
        clear_batch_state()?;
    } else {
        save_batch_state(&state)?;
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Batch Upload Summary (resumed):".bold());
            println!("{}", "-".repeat(60));

            for file in &batch_result.files {
                if file.success {
                    let size = file.size.map(format_size).unwrap_or_default();
                    println!(
                        "  {} {} {}",
                        "✓".green().bold(),
                        file.name.cyan(),
                        size.dimmed()
                    );
                } else {
                    println!(
                        "  {} {} {}",
                        "X".red().bold(),
                        file.name,
                        file.error.as_deref().unwrap_or("Unknown error").red()
                    );
                }
            }

            println!("{}", "-".repeat(60));
            println!(
                "  {} {} uploaded (this run), {} previously completed",
                "Total:".bold(),
                batch_result.uploaded.to_string().green(),
                previously_completed.to_string().green()
            );
            println!(
                "  {} {}",
                "Size:".bold(),
                format_size(batch_result.total_size)
            );

            if total_failed > 0 {
                println!(
                    "  {} {} failed",
                    "Errors:".bold(),
                    total_failed.to_string().red()
                );
                println!(
                    "\n  {} Use {} to retry failed files.",
                    "Hint:".yellow().bold(),
                    "--resume".cyan()
                );
            }
        }
        _ => {
            output_format.write(&batch_result)?;
        }
    }

    if total_failed > 0 {
        anyhow::bail!("{} file(s) failed to upload", total_failed);
    }

    Ok(())
}

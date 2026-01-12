// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Object management commands
//!
//! Commands for uploading, downloading, listing, and deleting objects in OSS buckets.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use futures_util::future;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;

use raps_kernel::output::OutputFormat;
use raps_kernel::prompts;
use raps_oss::OssClient;

#[derive(Debug, Subcommand)]
pub enum ObjectCommands {
    /// Upload a file to a bucket
    Upload {
        /// Bucket key
        bucket: Option<String>,

        /// Path to the file to upload
        file: PathBuf,

        /// Object key (defaults to filename)
        #[arg(short, long)]
        key: Option<String>,

        /// Resume interrupted upload (for large files)
        #[arg(short, long)]
        resume: bool,
    },

    /// Upload multiple files in parallel
    #[command(name = "upload-batch")]
    UploadBatch {
        /// Bucket key
        bucket: Option<String>,

        /// Files to upload
        files: Vec<PathBuf>,

        /// Number of parallel uploads (default: 4)
        #[arg(short, long, default_value = "4")]
        parallel: usize,
    },

    /// Download an object from a bucket
    Download {
        /// Bucket key
        bucket: Option<String>,

        /// Object key to download
        object: Option<String>,

        /// Output file path (defaults to object key)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// List objects in a bucket
    List {
        /// Bucket key
        bucket: Option<String>,
    },

    /// Delete an object from a bucket
    Delete {
        /// Bucket key
        bucket: Option<String>,

        /// Object key to delete
        object: Option<String>,

        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Get a signed S3 URL for direct download (bypasses OSS servers)
    SignedUrl {
        /// Bucket key
        bucket: String,

        /// Object key
        object: String,

        /// Expiration time in minutes (1-60, default 2)
        #[arg(short, long)]
        minutes: Option<u32>,
    },
}

impl ObjectCommands {
    pub async fn execute(self, client: &OssClient, output_format: OutputFormat) -> Result<()> {
        match self {
            ObjectCommands::Upload {
                bucket,
                file,
                key,
                resume,
            } => upload_object(client, bucket, file, key, resume, output_format).await,
            ObjectCommands::UploadBatch {
                bucket,
                files,
                parallel,
            } => upload_batch(client, bucket, files, parallel, output_format).await,
            ObjectCommands::Download {
                bucket,
                object,
                output,
            } => download_object(client, bucket, object, output, output_format).await,
            ObjectCommands::List { bucket } => list_objects(client, bucket, output_format).await,
            ObjectCommands::Delete {
                bucket,
                object,
                yes,
            } => delete_object(client, bucket, object, yes, output_format).await,
            ObjectCommands::SignedUrl {
                bucket,
                object,
                minutes,
            } => get_signed_url(client, &bucket, &object, minutes, output_format).await,
        }
    }
}

async fn select_bucket(client: &OssClient, provided: Option<String>) -> Result<String> {
    match provided {
        Some(b) => Ok(b),
        None => {
            let buckets = client.list_buckets().await?;
            if buckets.is_empty() {
                anyhow::bail!("No buckets found. Create a bucket first using 'raps bucket create'");
            }

            let bucket_keys: Vec<String> = buckets.iter().map(|b| b.bucket_key.clone()).collect();

            let selection = prompts::select("Select bucket", &bucket_keys)?;
            Ok(bucket_keys[selection].clone())
        }
    }
}

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

async fn upload_object(
    client: &OssClient,
    bucket: Option<String>,
    file: PathBuf,
    key: Option<String>,
    resume: bool,
    output_format: OutputFormat,
) -> Result<()> {
    // Validate file exists
    if !file.exists() {
        anyhow::bail!("File not found: {}", file.display());
    }

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
        let resume_msg = if resume { " (with resume)" } else { "" };
        println!(
            "{} {} {} {}{}",
            "Uploading".dimmed(),
            file.display().to_string().cyan(),
            "to".dimmed(),
            format!("{}/{}", bucket_key, object_key).cyan(),
            resume_msg.dimmed()
        );
    }

    let object_info = client
        .upload_object_with_options(&bucket_key, &object_key, &file, resume)
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

#[derive(Serialize)]
struct DownloadOutput {
    success: bool,
    bucket_key: String,
    object_key: String,
    output_path: String,
}

async fn download_object(
    client: &OssClient,
    bucket: Option<String>,
    object: Option<String>,
    output: Option<PathBuf>,
    output_format: OutputFormat,
) -> Result<()> {
    // Select bucket
    let bucket_key = select_bucket(client, bucket).await?;

    // Select or get object key
    let object_key = match object {
        Some(o) => o,
        None => {
            let objects = client.list_objects(&bucket_key).await?;
            if objects.is_empty() {
                anyhow::bail!("No objects found in bucket '{bucket_key}'");
            }

            let object_keys: Vec<String> = objects
                .iter()
                .map(|o| format!("{} ({})", o.object_key, format_size(o.size)))
                .collect();

            let selection = prompts::select("Select object to download", &object_keys)?;
            objects[selection].object_key.clone()
        }
    };

    // Determine output path
    let output_path = output.unwrap_or_else(|| PathBuf::from(&object_key));

    // Check if output file exists (respects --yes flag)
    if output_path.exists() {
        let overwrite = prompts::confirm(
            format!(
                "File '{}' already exists. Overwrite?",
                output_path.display()
            ),
            false,
        )?;

        if !overwrite {
            println!("{}", "Download cancelled.".yellow());
            return Ok(());
        }
    }

    if output_format.supports_colors() {
        println!(
            "{} {} {} {}",
            "Downloading".dimmed(),
            format!("{}/{}", bucket_key, object_key).cyan(),
            "to".dimmed(),
            output_path.display().to_string().cyan()
        );
    }

    client
        .download_object(&bucket_key, &object_key, &output_path)
        .await?;

    let output = DownloadOutput {
        success: true,
        bucket_key: bucket_key.clone(),
        object_key: object_key.clone(),
        output_path: output_path.display().to_string(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Download complete!", "✓".green().bold());
            println!("  {} {}", "Saved to:".bold(), output.output_path);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct ObjectListOutput {
    bucket_key: String,
    object_key: String,
    size: u64,
    size_human: String,
    sha1: Option<String>,
}

async fn list_objects(
    client: &OssClient,
    bucket: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    // Select bucket
    let bucket_key = select_bucket(client, bucket).await?;

    if output_format.supports_colors() {
        println!(
            "{}",
            format!("Fetching objects from '{}'...", bucket_key).dimmed()
        );
    }

    let objects = client.list_objects(&bucket_key).await?;

    let object_outputs: Vec<ObjectListOutput> = objects
        .iter()
        .map(|obj| ObjectListOutput {
            bucket_key: bucket_key.clone(),
            object_key: obj.object_key.clone(),
            size: obj.size,
            size_human: format_size(obj.size),
            sha1: obj.sha1.clone(),
        })
        .collect();

    if object_outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No objects found in this bucket.".yellow()),
            _ => {
                output_format.write(&Vec::<ObjectListOutput>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{} {}", "Objects in".bold(), bucket_key.cyan().bold());
            println!("{}", "-".repeat(80));
            println!(
                "{:<50} {:>15} {}",
                "Object Key".bold(),
                "Size".bold(),
                "SHA1".bold()
            );
            println!("{}", "-".repeat(80));

            for obj in &object_outputs {
                println!(
                    "{:<50} {:>15} {}",
                    truncate_str(&obj.object_key, 50).cyan(),
                    obj.size_human,
                    obj.sha1
                        .as_ref()
                        .map(|s| &s[..8.min(s.len())])
                        .unwrap_or("N/A")
                        .dimmed()
                );
            }

            println!("{}", "-".repeat(80));
        }
        _ => {
            output_format.write(&object_outputs)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct DeleteObjectOutput {
    success: bool,
    bucket_key: String,
    object_key: String,
    message: String,
}

async fn delete_object(
    client: &OssClient,
    bucket: Option<String>,
    object: Option<String>,
    skip_confirm: bool,
    output_format: OutputFormat,
) -> Result<()> {
    // Select bucket
    let bucket_key = select_bucket(client, bucket).await?;

    // Select or get object key
    let object_key = match object {
        Some(o) => o,
        None => {
            let objects = client.list_objects(&bucket_key).await?;
            if objects.is_empty() {
                println!("{}", "No objects found in this bucket.".yellow());
                return Ok(());
            }

            let object_keys: Vec<String> = objects.iter().map(|o| o.object_key.clone()).collect();

            let selection = prompts::select("Select object to delete", &object_keys)?;
            object_keys[selection].clone()
        }
    };

    // Confirm deletion (respects --yes flag)
    if !skip_confirm {
        let confirmed = prompts::confirm_destructive(format!(
            "Are you sure you want to delete '{}/{}'?",
            bucket_key,
            object_key.red()
        ))?;

        if !confirmed {
            println!("{}", "Deletion cancelled.".yellow());
            return Ok(());
        }
    }

    if output_format.supports_colors() {
        println!("{}", "Deleting object...".dimmed());
    }

    client.delete_object(&bucket_key, &object_key).await?;

    let output = DeleteObjectOutput {
        success: true,
        bucket_key: bucket_key.clone(),
        object_key: object_key.clone(),
        message: format!(
            "Object '{}/{}' deleted successfully!",
            bucket_key, object_key
        ),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} {}", "✓".green().bold(), output.message);
        }
        _ => {
            output_format.write(&output)?;
        }
    }
    Ok(())
}

/// Format file size in human-readable format
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

#[derive(Serialize)]
struct SignedUrlOutput {
    success: bool,
    bucket_key: String,
    object_key: String,
    url: Option<String>,
    urls: Option<Vec<String>>,
    size: Option<u64>,
    size_human: Option<String>,
    sha1: Option<String>,
    status: Option<String>,
    expiry_minutes: u32,
}

async fn get_signed_url(
    client: &OssClient,
    bucket: &str,
    object: &str,
    minutes: Option<u32>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Generating signed S3 download URL...".dimmed());
    }

    let signed = client
        .get_signed_download_url(bucket, object, minutes)
        .await?;

    let expiry = minutes.unwrap_or(2);
    let output = SignedUrlOutput {
        success: true,
        bucket_key: bucket.to_string(),
        object_key: object.to_string(),
        url: signed.url.clone(),
        urls: signed.urls.clone(),
        size: signed.size,
        size_human: signed.size.map(format_size),
        sha1: signed.sha1.clone(),
        status: signed.status.clone(),
        expiry_minutes: expiry,
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Signed URL generated!", "✓".green().bold());

            if let Some(ref url) = output.url {
                println!("\n{}", "Download URL (single part):".bold());
                println!("{}", url.cyan());
            }

            if let Some(ref urls) = output.urls
                && !urls.is_empty()
            {
                println!("\n{} ({} parts):", "Download URLs".bold(), urls.len());
                for (i, url) in urls.iter().enumerate() {
                    println!("  {} Part {}: {}", "-".cyan(), i + 1, url);
                }
            }

            if let Some(ref size_human) = output.size_human {
                println!("\n  {} {}", "Size:".bold(), size_human);
            }

            if let Some(ref sha1) = output.sha1 {
                println!("  {} {}", "SHA1:".bold(), sha1.dimmed());
            }

            if let Some(ref status) = output.status {
                println!("  {} {}", "Status:".bold(), status);
            }

            println!(
                "\n{}",
                format!("Note: URL expires in {} minutes", expiry).yellow()
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

/// Truncate string with ellipsis
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
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

async fn upload_batch(
    client: &OssClient,
    bucket: Option<String>,
    files: Vec<PathBuf>,
    parallel: usize,
    output_format: OutputFormat,
) -> Result<()> {
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

    for file in files {
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

            (file_path, object_key, result)
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
            Ok((file_path, _object_key, upload_result)) => {
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
                    }
                }
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

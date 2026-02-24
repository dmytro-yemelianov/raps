// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Object management commands
//!
//! Commands for uploading, downloading, listing, and deleting objects in OSS buckets.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use futures_util::future;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::output::OutputFormat;
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

        /// Resume a previously interrupted batch upload
        #[arg(long)]
        resume: bool,
    },

    /// Download an object from a bucket
    Download {
        /// Bucket key
        bucket: Option<String>,

        /// Object key to download
        object: Option<String>,

        /// Output file path (defaults to object key)
        #[arg(long = "out-file")]
        out_file: Option<PathBuf>,
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

    /// Get detailed information about an object
    Info {
        /// Bucket key
        bucket: String,
        /// Object key
        object: String,
    },

    /// Copy an object to another bucket or key
    Copy {
        /// Source bucket key
        #[arg(long)]
        source_bucket: String,
        /// Source object key
        #[arg(long)]
        source_object: String,
        /// Destination bucket key
        #[arg(long)]
        dest_bucket: String,
        /// Destination object key (defaults to source object key)
        #[arg(long)]
        dest_object: Option<String>,
    },

    /// Rename an object within a bucket
    Rename {
        /// Bucket key
        bucket: String,
        /// Current object key
        object: String,
        /// New object key
        #[arg(long)]
        new_key: String,
    },

    /// Batch copy objects from one bucket to another
    #[command(name = "batch-copy")]
    BatchCopy {
        /// Source bucket key
        source_bucket: String,
        /// Destination bucket key
        dest_bucket: String,
        /// Filter objects by key prefix
        #[arg(long)]
        prefix: Option<String>,
        /// Comma-separated specific object keys to copy
        #[arg(long)]
        keys: Option<String>,
    },

    /// Batch rename objects within a bucket
    #[command(name = "batch-rename")]
    BatchRename {
        /// Bucket key
        bucket: String,
        /// Pattern to match in object keys
        #[arg(long)]
        from: String,
        /// Replacement pattern for matched keys
        #[arg(long)]
        to: String,
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
                resume,
            } => upload_batch(client, bucket, files, parallel, resume, output_format).await,
            ObjectCommands::Download {
                bucket,
                object,
                out_file,
            } => download_object(client, bucket, object, out_file, output_format).await,
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
            ObjectCommands::Info { bucket, object } => {
                object_info(client, &bucket, &object, output_format).await
            }
            ObjectCommands::Copy {
                source_bucket,
                source_object,
                dest_bucket,
                dest_object,
            } => {
                copy_object(
                    client,
                    &source_bucket,
                    &source_object,
                    &dest_bucket,
                    dest_object.as_deref(),
                    output_format,
                )
                .await
            }
            ObjectCommands::Rename {
                bucket,
                object,
                new_key,
            } => rename_object(client, &bucket, &object, &new_key, output_format).await,
            ObjectCommands::BatchCopy {
                source_bucket,
                dest_bucket,
                prefix,
                keys,
            } => {
                batch_copy_objects(
                    client,
                    &source_bucket,
                    &dest_bucket,
                    prefix,
                    keys,
                    output_format,
                )
                .await
            }
            ObjectCommands::BatchRename { bucket, from, to } => {
                batch_rename_objects(client, &bucket, &from, &to, output_format).await
            }
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

// ============== OBJECT INFO ==============

#[derive(Serialize)]
struct ObjectInfoOutput {
    bucket_key: String,
    object_key: String,
    object_id: String,
    size: u64,
    size_human: String,
    content_type: String,
    sha1: String,
    created_date: Option<String>,
    last_modified_date: Option<String>,
    urn: String,
}

async fn object_info(
    client: &OssClient,
    bucket: &str,
    object: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!(
            "{}",
            format!("Fetching details for '{}/{}'...", bucket, object).dimmed()
        );
    }

    let details = client.get_object_details(bucket, object).await?;
    let urn = client.get_urn(bucket, object);

    let output = ObjectInfoOutput {
        bucket_key: details.bucket_key.clone(),
        object_key: details.object_key.clone(),
        object_id: details.object_id.clone(),
        size: details.size,
        size_human: format_size(details.size),
        content_type: details.content_type.clone(),
        sha1: details.sha1.clone(),
        created_date: details.created_date.clone(),
        last_modified_date: details.last_modified_date.clone(),
        urn,
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{} {}", "Object:".bold(), output.object_key.cyan().bold());
            println!("{}", "-".repeat(60));
            println!("  {} {}", "Bucket:".bold(), output.bucket_key);
            println!(
                "  {} {} ({})",
                "Size:".bold(),
                output.size_human,
                output.size
            );
            println!("  {} {}", "Content-Type:".bold(), output.content_type);
            println!("  {} {}", "SHA1:".bold(), output.sha1.dimmed());
            println!(
                "  {} {}",
                "Created:".bold(),
                output.created_date.as_deref().unwrap_or("N/A")
            );
            println!(
                "  {} {}",
                "Modified:".bold(),
                output.last_modified_date.as_deref().unwrap_or("N/A")
            );
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

// ============== OBJECT COPY ==============

#[derive(Serialize)]
struct CopyObjectOutput {
    success: bool,
    source_bucket: String,
    source_object: String,
    dest_bucket: String,
    dest_object: String,
    size: u64,
    size_human: String,
    message: String,
}

async fn copy_object(
    client: &OssClient,
    source_bucket: &str,
    source_object: &str,
    dest_bucket: &str,
    dest_object: Option<&str>,
    output_format: OutputFormat,
) -> Result<()> {
    let destination_key = dest_object.unwrap_or(source_object);

    if output_format.supports_colors() {
        println!(
            "{} {} {} {}",
            "Copying".dimmed(),
            format!("{}/{}", source_bucket, source_object).cyan(),
            "to".dimmed(),
            format!("{}/{}", dest_bucket, destination_key).cyan()
        );
    }

    // OSS does not have a native copy API, so we download to a temp file and re-upload
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!(
        "raps_copy_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));

    // Download from source
    client
        .download_object(source_bucket, source_object, &temp_path)
        .await
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temp_path);
        })?;

    // Upload to destination
    let upload_result = client
        .upload_object(dest_bucket, destination_key, &temp_path)
        .await;

    // Clean up temp file regardless of outcome
    let _ = std::fs::remove_file(&temp_path);

    let info = upload_result?;

    let output = CopyObjectOutput {
        success: true,
        source_bucket: source_bucket.to_string(),
        source_object: source_object.to_string(),
        dest_bucket: dest_bucket.to_string(),
        dest_object: destination_key.to_string(),
        size: info.size,
        size_human: format_size(info.size),
        message: format!(
            "Copied '{}/{}' to '{}/{}'",
            source_bucket, source_object, dest_bucket, destination_key
        ),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} {}", "✓".green().bold(), output.message);
            println!("  {} {}", "Size:".bold(), output.size_human);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

// ============== OBJECT RENAME ==============

#[derive(Serialize)]
struct RenameObjectOutput {
    success: bool,
    bucket_key: String,
    old_key: String,
    new_key: String,
    size: u64,
    size_human: String,
    message: String,
}

async fn rename_object(
    client: &OssClient,
    bucket: &str,
    object: &str,
    new_key: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!(
            "{} {} {} {} {} {}",
            "Renaming".dimmed(),
            format!("{}/{}", bucket, object).cyan(),
            "to".dimmed(),
            format!("{}/{}", bucket, new_key).cyan(),
            "in".dimmed(),
            bucket.cyan()
        );
    }

    // OSS does not have a native rename API; implement as copy + delete
    // Step 1: Download to temp file
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!(
        "raps_rename_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));

    client
        .download_object(bucket, object, &temp_path)
        .await
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temp_path);
        })?;

    // Step 2: Upload with new key
    let upload_result = client.upload_object(bucket, new_key, &temp_path).await;

    // Clean up temp file regardless of outcome
    let _ = std::fs::remove_file(&temp_path);

    let info = upload_result?;

    // Step 3: Delete the original object
    client.delete_object(bucket, object).await?;

    let output = RenameObjectOutput {
        success: true,
        bucket_key: bucket.to_string(),
        old_key: object.to_string(),
        new_key: new_key.to_string(),
        size: info.size,
        size_human: format_size(info.size),
        message: format!(
            "Renamed '{}/{}' to '{}/{}'",
            bucket, object, bucket, new_key
        ),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} {}", "✓".green().bold(), output.message);
            println!("  {} {}", "Old key:".bold(), output.old_key);
            println!("  {} {}", "New key:".bold(), output.new_key);
            println!("  {} {}", "Size:".bold(), output.size_human);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

// ============== BATCH OPERATIONS ==============

async fn batch_copy_objects(
    client: &OssClient,
    source_bucket: &str,
    dest_bucket: &str,
    prefix: Option<String>,
    keys: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    // Determine which keys to copy
    let object_keys = if let Some(keys_str) = keys {
        keys_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        // List objects from source bucket, optionally filtered by prefix
        let objects = client.list_objects(source_bucket).await?;
        let filtered: Vec<String> = objects
            .into_iter()
            .filter(|o| {
                prefix
                    .as_ref()
                    .is_none_or(|p| o.object_key.starts_with(p.as_str()))
            })
            .map(|o| o.object_key)
            .collect();
        filtered
    };

    if object_keys.is_empty() {
        println!("{}", "No objects to copy.".dimmed());
        return Ok(());
    }

    println!(
        "{} {} objects from {} to {}...",
        "Copying".dimmed(),
        object_keys.len(),
        source_bucket.cyan(),
        dest_bucket.cyan()
    );

    let result = client
        .batch_copy_objects(source_bucket, dest_bucket, &object_keys)
        .await?;

    match output_format {
        OutputFormat::Table => {
            for item in &result.results {
                match &item.result {
                    Ok(_) => println!("  {} {}", "✓".green().bold(), item.key),
                    Err(e) => println!("  {} {} ({})", "✗".red().bold(), item.key, e),
                }
            }
            println!(
                "\n{} copied, {} failed (of {} total)",
                result.succeeded.to_string().green(),
                result.failed.to_string().red(),
                result.total
            );
        }
        _ => {
            #[derive(Serialize)]
            struct BatchSummary {
                total: usize,
                succeeded: usize,
                failed: usize,
            }
            output_format.write(&BatchSummary {
                total: result.total,
                succeeded: result.succeeded,
                failed: result.failed,
            })?;
        }
    }

    Ok(())
}

async fn batch_rename_objects(
    client: &OssClient,
    bucket: &str,
    from_pattern: &str,
    to_pattern: &str,
    output_format: OutputFormat,
) -> Result<()> {
    // List objects and find those matching the pattern
    let objects = client.list_objects(bucket).await?;
    let renames: Vec<(String, String)> = objects
        .into_iter()
        .filter(|o| o.object_key.contains(from_pattern))
        .map(|o| {
            let new_key = o.object_key.replace(from_pattern, to_pattern);
            (o.object_key, new_key)
        })
        .collect();

    if renames.is_empty() {
        println!(
            "{} No objects match pattern '{}'.",
            "⚠".yellow(),
            from_pattern
        );
        return Ok(());
    }

    println!(
        "{} {} objects in {}...",
        "Renaming".dimmed(),
        renames.len(),
        bucket.cyan()
    );

    let result = client.batch_rename_objects(bucket, &renames).await?;

    match output_format {
        OutputFormat::Table => {
            for item in &result.results {
                let new_key = renames
                    .iter()
                    .find(|(old, _)| old == &item.key)
                    .map(|(_, new)| new.as_str())
                    .unwrap_or("?");
                match &item.result {
                    Ok(_) => println!("  {} {} → {}", "✓".green().bold(), item.key, new_key),
                    Err(e) => println!("  {} {} ({})", "✗".red().bold(), item.key, e),
                }
            }
            println!(
                "\n{} renamed, {} failed (of {} total)",
                result.succeeded.to_string().green(),
                result.failed.to_string().red(),
                result.total
            );
        }
        _ => {
            #[derive(Serialize)]
            struct BatchSummary {
                total: usize,
                succeeded: usize,
                failed: usize,
            }
            output_format.write(&BatchSummary {
                total: result.total,
                succeeded: result.succeeded,
                failed: result.failed,
            })?;
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

async fn upload_batch(
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

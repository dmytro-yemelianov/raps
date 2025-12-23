//! Object management commands
//!
//! Commands for uploading, downloading, listing, and deleting objects in OSS buckets.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Confirm, Select};
use serde::Serialize;
use std::path::PathBuf;

use crate::api::OssClient;
use crate::output::OutputFormat;

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
            ObjectCommands::Upload { bucket, file, key } => {
                upload_object(client, bucket, file, key, output_format).await
            }
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

            let selection = Select::new()
                .with_prompt("Select bucket")
                .items(&bucket_keys)
                .interact()?;

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
        println!(
            "{} {} {} {}",
            "Uploading".dimmed(),
            file.display().to_string().cyan(),
            "to".dimmed(),
            format!("{}/{}", bucket_key, object_key).cyan()
        );
    }

    let object_info = client
        .upload_object(&bucket_key, &object_key, &file)
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
            println!("\n  {} {}", "URN (for translation):".bold().yellow(), output.urn);
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
                anyhow::bail!("No objects found in bucket '{}'", bucket_key);
            }

            let object_keys: Vec<String> = objects
                .iter()
                .map(|o| format!("{} ({})", o.object_key, format_size(o.size)))
                .collect();

            let selection = Select::new()
                .with_prompt("Select object to download")
                .items(&object_keys)
                .interact()?;

            objects[selection].object_key.clone()
        }
    };

    // Determine output path
    let output_path = output.unwrap_or_else(|| PathBuf::from(&object_key));

    // Check if output file exists
    if output_path.exists() {
        let overwrite = Confirm::new()
            .with_prompt(format!(
                "File '{}' already exists. Overwrite?",
                output_path.display()
            ))
            .default(false)
            .interact()?;

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

async fn list_objects(client: &OssClient, bucket: Option<String>, output_format: OutputFormat) -> Result<()> {
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
            println!("{}", "─".repeat(80));
            println!(
                "{:<50} {:>15} {}",
                "Object Key".bold(),
                "Size".bold(),
                "SHA1".bold()
            );
            println!("{}", "─".repeat(80));

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

            println!("{}", "─".repeat(80));
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

            let selection = Select::new()
                .with_prompt("Select object to delete")
                .items(&object_keys)
                .interact()?;

            object_keys[selection].clone()
        }
    };

    // Confirm deletion
    if !skip_confirm {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "Are you sure you want to delete '{}/{}'?",
                bucket_key,
                object_key.red()
            ))
            .default(false)
            .interact()?;

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
        message: format!("Object '{}/{}' deleted successfully!", bucket_key, object_key),
    };

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} {}",
                "✓".green().bold(),
                output.message
            );
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

            if let Some(ref urls) = output.urls {
                if !urls.is_empty() {
                    println!("\n{} ({} parts):", "Download URLs".bold(), urls.len());
                    for (i, url) in urls.iter().enumerate() {
                        println!("  {} Part {}: {}", "•".cyan(), i + 1, url);
                    }
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

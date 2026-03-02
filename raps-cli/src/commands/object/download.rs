// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Download, list, info, signed URL, and delete commands for OSS objects.

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::path::PathBuf;

use crate::output::OutputFormat;
use raps_kernel::prompts;
use raps_oss::OssClient;

use super::{format_size, select_bucket, truncate_str};

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct DownloadOutput {
    success: bool,
    bucket_key: String,
    object_key: String,
    output_path: String,
}

pub(super) async fn download_object(
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

    let to_stdout = output.as_ref().is_some_and(|p| p.as_os_str() == "-");

    if to_stdout {
        // Stream directly to stdout — no progress messages, no formatting
        let mut stdout = tokio::io::stdout();
        client
            .download_object_to_writer(&bucket_key, &object_key, &mut stdout)
            .await?;
        return Ok(());
    }

    // File-based download
    let output_path = match output {
        Some(p) => p,
        None => {
            let safe = raps_kernel::security::sanitize_filename(&object_key)?;
            PathBuf::from(safe)
        }
    };

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

    // Try cache first — use SHA1 from object details
    if raps_kernel::cache::is_enabled()
        && let Ok(details) = client.get_object_details(&bucket_key, &object_key).await
        && let Ok(true) = raps_kernel::cache::materialize(&details.sha1, &output_path)
    {
        if output_format.supports_colors() {
            println!(
                "{} {} (from cache)",
                "✓".green().bold(),
                format!("{}/{}", bucket_key, object_key).cyan()
            );
        }
        let output = DownloadOutput {
            success: true,
            bucket_key,
            object_key,
            output_path: output_path.display().to_string(),
        };
        match output_format {
            OutputFormat::Table => {
                println!("  {} {}", "Saved to:".bold(), output.output_path);
            }
            _ => {
                output_format.write(&output)?;
            }
        }
        return Ok(());
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

    // Store in cache after successful download
    if raps_kernel::cache::is_enabled()
        && let Ok(details) = client.get_object_details(&bucket_key, &object_key).await
    {
        let _ = raps_kernel::cache::store(&details.sha1, &output_path);
    }

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

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct ObjectListOutput {
    bucket_key: String,
    object_key: String,
    size: u64,
    size_human: String,
    sha1: Option<String>,
}

pub(super) async fn list_objects(
    client: &OssClient,
    bucket: Option<String>,
    limit: Option<usize>,
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

    let objects = client.list_objects_with_limit(&bucket_key, limit).await?;

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

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct DeleteObjectOutput {
    success: bool,
    bucket_key: String,
    object_key: String,
    message: String,
}

pub(super) async fn delete_object(
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

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct SignedUrlOutput {
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

pub(super) async fn get_signed_url(
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

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct ObjectInfoOutput {
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

pub(super) async fn object_info(
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

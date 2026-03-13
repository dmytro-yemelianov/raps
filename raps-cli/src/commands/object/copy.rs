// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Copy and rename commands for OSS objects, including batch operations.

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_oss::OssClient;

use super::format_size;

// ============== OBJECT COPY ==============

#[derive(Serialize, schemars::JsonSchema)]
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

pub(super) async fn copy_object(
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

    // OSS does not have a native copy API, so we download to a temp file and re-upload.
    // Use the current working directory for the temp file to stay within the sandbox.
    let temp_dir = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
    let temp_path = temp_dir.join(format!(
        ".raps_copy_{}",
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
    let _ = tokio::fs::remove_file(&temp_path).await;

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

#[derive(Serialize, schemars::JsonSchema)]
struct RenameObjectOutput {
    success: bool,
    bucket_key: String,
    old_key: String,
    new_key: String,
    size: u64,
    size_human: String,
    message: String,
}

pub(super) async fn rename_object(
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
    // Step 1: Download to temp file (use cwd to stay within sandbox)
    let temp_dir = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
    let temp_path = temp_dir.join(format!(
        ".raps_rename_{}",
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
    let _ = tokio::fs::remove_file(&temp_path).await;

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

pub(super) async fn batch_copy_objects(
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
            #[derive(Serialize, schemars::JsonSchema)]
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

pub(super) async fn batch_rename_objects(
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
            #[derive(Serialize, schemars::JsonSchema)]
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

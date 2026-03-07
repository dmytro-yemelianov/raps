// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Object management commands
//!
//! Commands for uploading, downloading, listing, and deleting objects in OSS buckets.

mod copy;
pub(crate) mod download;
mod download_bulk;
pub(crate) mod secret_scan;
pub(crate) mod upload;

use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

use crate::output::OutputFormat;
use raps_kernel::prompts;
use raps_oss::OssClient;

use copy::{batch_copy_objects, batch_rename_objects, copy_object, rename_object};
use download::{delete_object, download_object, get_signed_url, list_objects, object_info};
use download_bulk::download_bulk;
use upload::{upload_batch, upload_object};

#[derive(Debug, Subcommand)]
pub enum ObjectCommands {
    /// Upload a file to a bucket (use `-` to read from stdin)
    Upload {
        /// Bucket key
        bucket: Option<String>,

        /// Path to the file to upload, or `-` for stdin
        file: PathBuf,

        /// Object key (defaults to filename; required when reading from stdin)
        #[arg(short, long)]
        key: Option<String>,

        /// Resume interrupted upload (for large files)
        #[arg(short, long)]
        resume: bool,

        /// Skip upload if an identical object (same SHA-1) already exists
        #[arg(long)]
        skip_if_exists: bool,

        /// Show cost/time estimate before uploading
        #[arg(long)]
        cost_estimate: bool,

        /// Skip secret scanning before upload
        #[arg(long)]
        skip_secret_scan: bool,

        /// Allow upload even if secrets are detected (overrides the default block)
        #[arg(long)]
        allow_secrets: bool,
    },

    /// Upload multiple files in parallel
    #[command(name = "upload-batch")]
    UploadBatch {
        /// Bucket key
        bucket: Option<String>,

        /// Files to upload
        files: Vec<PathBuf>,

        /// Number of parallel uploads (defaults to global --concurrency)
        #[arg(short, long)]
        parallel: Option<usize>,

        /// Resume a previously interrupted batch upload
        #[arg(long)]
        resume: bool,

        /// Skip upload if an identical object (same SHA-1) already exists
        #[arg(long)]
        skip_if_exists: bool,
    },

    /// Download an object from a bucket (use `--out-file -` to write to stdout)
    Download {
        /// Bucket key
        bucket: Option<String>,

        /// Object key to download
        object: Option<String>,

        /// Output file path (defaults to object key; use `-` for stdout)
        #[arg(long = "out-file")]
        out_file: Option<PathBuf>,
    },

    /// Download multiple objects in parallel matching a prefix
    #[command(name = "download-bulk")]
    DownloadBulk {
        /// Bucket key
        bucket: Option<String>,

        /// Object key prefix to match (e.g. `models/` to download all objects under that path)
        prefix: String,

        /// Directory to write downloaded files into
        #[arg(long, default_value = ".")]
        output_dir: PathBuf,

        /// Maximum number of parallel downloads
        #[arg(long, default_value = "4")]
        concurrency: usize,

        /// Skip files that already exist locally with a matching size (on by default)
        #[arg(long, default_value = "true")]
        skip_existing: bool,

        /// Download all files directly into --output-dir without preserving the key prefix structure
        #[arg(long)]
        flat: bool,
    },

    /// List objects in a bucket
    List {
        /// Bucket key
        bucket: Option<String>,

        /// Maximum number of objects to return
        #[arg(long, short = 'n')]
        limit: Option<usize>,
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

    /// List pending/incomplete resumable uploads
    #[command(name = "upload-status")]
    UploadStatus,

    /// Abort a resumable upload and clean up state
    #[command(name = "upload-abort")]
    UploadAbort {
        /// Bucket key
        bucket: String,
        /// Object key
        object: String,
    },

    /// Remove stale upload state files
    #[command(name = "upload-cleanup")]
    UploadCleanup {
        /// Remove all state files without checking file existence
        #[arg(long)]
        all: bool,
    },
}

impl ObjectCommands {
    pub async fn execute(
        self,
        client: &OssClient,
        output_format: OutputFormat,
        concurrency: usize,
    ) -> Result<()> {
        match self {
            ObjectCommands::Upload {
                bucket,
                file,
                key,
                resume,
                skip_if_exists,
                cost_estimate,
                skip_secret_scan,
                allow_secrets,
            } => upload_object(client, bucket, file, key, resume, skip_if_exists, cost_estimate, skip_secret_scan, allow_secrets, output_format).await,
            ObjectCommands::UploadBatch {
                bucket,
                files,
                parallel,
                resume,
                skip_if_exists,
            } => {
                upload_batch(
                    client,
                    bucket,
                    files,
                    parallel.unwrap_or(concurrency),
                    resume,
                    skip_if_exists,
                    output_format,
                )
                .await
            }
            ObjectCommands::Download {
                bucket,
                object,
                out_file,
            } => download_object(client, bucket, object, out_file, output_format).await,
            ObjectCommands::DownloadBulk {
                bucket,
                prefix,
                output_dir,
                concurrency: cmd_concurrency,
                skip_existing,
                flat,
            } => {
                download_bulk(
                    client,
                    bucket,
                    prefix,
                    output_dir,
                    cmd_concurrency,
                    skip_existing,
                    flat,
                    output_format,
                )
                .await
            }
            ObjectCommands::List { bucket, limit } => {
                list_objects(client, bucket, limit, output_format).await
            }
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
            ObjectCommands::UploadStatus => upload::upload_status(output_format),
            ObjectCommands::UploadAbort { bucket, object } => {
                upload::upload_abort(&bucket, &object, output_format)
            }
            ObjectCommands::UploadCleanup { all } => upload::upload_cleanup(all, output_format),
        }
    }
}

pub(super) async fn select_bucket(client: &OssClient, provided: Option<String>) -> Result<String> {
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

/// Format file size in human-readable format
pub(super) fn format_size(bytes: u64) -> String {
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

/// Truncate string with ellipsis
pub(super) fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(512), "512 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.00 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1_048_576), "1.00 MB");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn test_format_size_zero() {
        assert_eq!(format_size(0), "0 B");
    }

    #[test]
    fn test_format_size_boundary() {
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.00 KB");
    }

    #[test]
    fn test_truncate_str_short() {
        let result = truncate_str("hello", 10);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_str_long() {
        let result = truncate_str("this is a very long string", 10);
        assert_eq!(result, "this is...");
        assert_eq!(result.len(), 10);
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Inspect commands for examining archive contents via HTTP Range requests.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_oss::OssClient;

use super::object::{format_size, select_bucket, truncate_str};

#[derive(Debug, Subcommand)]
pub enum InspectCommands {
    /// List contents of a zip archive without downloading the entire file
    Zip {
        /// Bucket key
        bucket: Option<String>,

        /// Object key of the zip file
        object: Option<String>,
    },
}

#[derive(Serialize, schemars::JsonSchema)]
struct ZipEntryOutput {
    name: String,
    compressed_size: u64,
    uncompressed_size: u64,
    uncompressed_size_human: String,
    is_directory: bool,
}

#[derive(Serialize, schemars::JsonSchema)]
struct ZipInspectOutput {
    bucket_key: String,
    object_key: String,
    total_entries: usize,
    total_uncompressed_size: u64,
    total_uncompressed_size_human: String,
    bytes_downloaded: u64,
    bytes_downloaded_human: String,
    entries: Vec<ZipEntryOutput>,
}

impl InspectCommands {
    pub async fn execute(self, client: &OssClient, output_format: OutputFormat) -> Result<()> {
        match self {
            InspectCommands::Zip { bucket, object } => {
                inspect_zip(client, bucket, object, output_format).await
            }
        }
    }
}

async fn inspect_zip(
    client: &OssClient,
    bucket: Option<String>,
    object: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    use raps_kernel::prompts;

    let bucket_key = select_bucket(client, bucket).await?;

    let object_key = match object {
        Some(o) => o,
        None => {
            let objects = client.list_objects(&bucket_key).await?;
            let zip_objects: Vec<_> = objects
                .iter()
                .filter(|o| {
                    o.object_key.ends_with(".zip")
                        || o.object_key.ends_with(".ZIP")
                        || o.object_key.ends_with(".jar")
                })
                .collect();

            if zip_objects.is_empty() {
                anyhow::bail!("No zip files found in bucket '{bucket_key}'");
            }

            let keys: Vec<String> = zip_objects
                .iter()
                .map(|o| format!("{} ({})", o.object_key, format_size(o.size)))
                .collect();

            let selection = prompts::select("Select zip file to inspect", &keys)?;
            zip_objects[selection].object_key.clone()
        }
    };

    if output_format.supports_colors() {
        println!(
            "{} {} {}...",
            "Inspecting".dimmed(),
            format!("{}/{}", bucket_key, object_key).cyan(),
            "(fetching central directory)".dimmed()
        );
    }

    // Get file size from object details
    let details = client
        .get_object_details(&bucket_key, &object_key)
        .await
        .context("Failed to get object details")?;
    let file_size = details.size;

    if file_size < 22 {
        anyhow::bail!("File is too small to be a valid zip archive");
    }

    // Fetch the last 65536 bytes (max comment size) + 22 (EOCD size)
    // This should contain the End of Central Directory record
    let tail_size: u64 = 65558.min(file_size);
    let start = file_size - tail_size;
    let tail = client
        .fetch_range(&bucket_key, &object_key, start, file_size - 1)
        .await
        .context("Failed to fetch zip tail via Range request")?;

    let mut bytes_downloaded = tail.len() as u64;

    // Find End of Central Directory record (signature 0x06054b50)
    let eocd_pos = find_eocd(&tail).context(
        "Could not find zip End of Central Directory — file may not be a zip archive",
    )?;

    // Parse EOCD
    let eocd = &tail[eocd_pos..];
    if eocd.len() < 22 {
        anyhow::bail!("Truncated EOCD record");
    }

    let total_entries = u16::from_le_bytes([eocd[10], eocd[11]]) as usize;
    let cd_size = u32::from_le_bytes([eocd[12], eocd[13], eocd[14], eocd[15]]) as u64;
    let cd_offset = u32::from_le_bytes([eocd[16], eocd[17], eocd[18], eocd[19]]) as u64;

    // Check if we already have the central directory in our tail fetch
    let cd_data = if cd_offset >= start {
        // Central directory is within our tail fetch
        let local_offset = (cd_offset - start) as usize;
        tail[local_offset..local_offset + cd_size as usize].to_vec()
    } else {
        // Need to fetch the central directory separately
        let cd_bytes = client
            .fetch_range(
                &bucket_key,
                &object_key,
                cd_offset,
                cd_offset + cd_size - 1,
            )
            .await
            .context("Failed to fetch zip central directory")?;
        bytes_downloaded += cd_bytes.len() as u64;
        cd_bytes
    };

    // Parse central directory entries
    let entries = parse_central_directory(&cd_data, total_entries)?;

    let total_uncompressed: u64 = entries.iter().map(|e| e.uncompressed_size).sum();

    let entry_outputs: Vec<ZipEntryOutput> = entries
        .into_iter()
        .map(|e| ZipEntryOutput {
            is_directory: e.name.ends_with('/'),
            name: e.name,
            compressed_size: e.compressed_size,
            uncompressed_size: e.uncompressed_size,
            uncompressed_size_human: format_size(e.uncompressed_size),
        })
        .collect();

    let output = ZipInspectOutput {
        bucket_key: bucket_key.clone(),
        object_key: object_key.clone(),
        total_entries: entry_outputs.len(),
        total_uncompressed_size: total_uncompressed,
        total_uncompressed_size_human: format_size(total_uncompressed),
        bytes_downloaded,
        bytes_downloaded_human: format_size(bytes_downloaded),
        entries: entry_outputs,
    };

    match output_format {
        OutputFormat::Table => {
            println!(
                "\n{} {}/{}",
                "Archive:".bold(),
                bucket_key.cyan(),
                object_key.cyan()
            );
            println!("{}", "-".repeat(80));
            println!(
                "{:<50} {:>12} {:>12}",
                "Name".bold(),
                "Compressed".bold(),
                "Size".bold()
            );
            println!("{}", "-".repeat(80));

            for entry in &output.entries {
                let name_display = if entry.is_directory {
                    truncate_str(&entry.name, 50).blue().to_string()
                } else {
                    truncate_str(&entry.name, 50).to_string()
                };
                println!(
                    "{:<50} {:>12} {:>12}",
                    name_display,
                    format_size(entry.compressed_size),
                    entry.uncompressed_size_human,
                );
            }

            println!("{}", "-".repeat(80));
            println!(
                "{} {} entries, {} uncompressed",
                "->".cyan(),
                output.total_entries,
                output.total_uncompressed_size_human
            );
            println!(
                "   {} of {} downloaded ({:.1}%)",
                output.bytes_downloaded_human,
                format_size(file_size),
                (bytes_downloaded as f64 / file_size as f64) * 100.0
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Zip parsing helpers
// ---------------------------------------------------------------------------

/// Find the End of Central Directory record in a byte buffer.
/// Searches backwards from the end for the EOCD signature (0x06054b50).
fn find_eocd(data: &[u8]) -> Option<usize> {
    if data.len() < 22 {
        return None;
    }
    // Search backwards for EOCD signature
    for i in (0..=data.len() - 22).rev() {
        if data[i] == 0x50
            && data[i + 1] == 0x4b
            && data[i + 2] == 0x05
            && data[i + 3] == 0x06
        {
            return Some(i);
        }
    }
    None
}

struct CdEntry {
    name: String,
    compressed_size: u64,
    uncompressed_size: u64,
}

/// Parse central directory entries from raw bytes.
fn parse_central_directory(data: &[u8], expected_count: usize) -> Result<Vec<CdEntry>> {
    let mut entries = Vec::with_capacity(expected_count);
    let mut offset = 0;

    for _ in 0..expected_count {
        if offset + 46 > data.len() {
            break;
        }

        // Verify central directory file header signature (0x02014b50)
        if data[offset] != 0x50
            || data[offset + 1] != 0x4b
            || data[offset + 2] != 0x01
            || data[offset + 3] != 0x02
        {
            anyhow::bail!("Invalid central directory entry at offset {}", offset);
        }

        let compressed_size =
            u32::from_le_bytes([data[offset + 20], data[offset + 21], data[offset + 22], data[offset + 23]]) as u64;
        let uncompressed_size =
            u32::from_le_bytes([data[offset + 24], data[offset + 25], data[offset + 26], data[offset + 27]]) as u64;
        let name_len =
            u16::from_le_bytes([data[offset + 28], data[offset + 29]]) as usize;
        let extra_len =
            u16::from_le_bytes([data[offset + 30], data[offset + 31]]) as usize;
        let comment_len =
            u16::from_le_bytes([data[offset + 32], data[offset + 33]]) as usize;

        let name_start = offset + 46;
        let name_end = name_start + name_len;

        if name_end > data.len() {
            break;
        }

        let name = String::from_utf8_lossy(&data[name_start..name_end]).to_string();

        entries.push(CdEntry {
            name,
            compressed_size,
            uncompressed_size,
        });

        offset = name_end + extra_len + comment_len;
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_eocd() {
        // Minimal valid EOCD record (22 bytes)
        let mut data = vec![0u8; 100];
        // Place EOCD signature at offset 78
        data[78] = 0x50;
        data[79] = 0x4b;
        data[80] = 0x05;
        data[81] = 0x06;
        // Rest is zeros (valid for empty archive)

        assert_eq!(find_eocd(&data), Some(78));
    }

    #[test]
    fn test_find_eocd_not_found() {
        let data = vec![0u8; 100];
        assert_eq!(find_eocd(&data), None);
    }

    #[test]
    fn test_find_eocd_too_small() {
        let data = vec![0u8; 10];
        assert_eq!(find_eocd(&data), None);
    }

    #[test]
    fn test_parse_central_directory_single_entry() {
        // Build a minimal central directory entry
        let name = b"hello.txt";
        let mut cd = vec![0u8; 46 + name.len()];

        // Signature
        cd[0] = 0x50;
        cd[1] = 0x4b;
        cd[2] = 0x01;
        cd[3] = 0x02;

        // Compressed size = 100
        cd[20..24].copy_from_slice(&100u32.to_le_bytes());
        // Uncompressed size = 200
        cd[24..28].copy_from_slice(&200u32.to_le_bytes());
        // Name length
        cd[28..30].copy_from_slice(&(name.len() as u16).to_le_bytes());
        // Extra length = 0
        cd[30..32].copy_from_slice(&0u16.to_le_bytes());
        // Comment length = 0
        cd[32..34].copy_from_slice(&0u16.to_le_bytes());

        // Name
        cd[46..46 + name.len()].copy_from_slice(name);

        let entries = parse_central_directory(&cd, 1).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "hello.txt");
        assert_eq!(entries[0].compressed_size, 100);
        assert_eq!(entries[0].uncompressed_size, 200);
    }
}

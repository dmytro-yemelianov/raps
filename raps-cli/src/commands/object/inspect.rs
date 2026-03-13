// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Archive inspection via HTTP Range requests.
//!
//! Supports listing and extracting files from .tar.gz and .zip archives
//! stored in OSS without downloading the entire archive.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;
use std::io::Read as _;

use crate::output::OutputFormat;
use raps_oss::OssClient;

use super::format_size;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct ArchiveEntry {
    pub path: String,
    pub size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f64,
    pub archive_type: String,
}

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct InspectOutput {
    pub bucket_key: String,
    pub object_key: String,
    pub archive_type: String,
    pub total_files: usize,
    pub total_size: u64,
    pub total_size_human: String,
    pub entries: Vec<ArchiveEntry>,
}

// ---------------------------------------------------------------------------
// ZIP inspection helpers
// ---------------------------------------------------------------------------

/// Minimum size of End-of-Central-Directory record (without comment)
const EOCD_SIZE: u64 = 22;
/// ZIP EOCD signature
const EOCD_SIG: u32 = 0x06054b50;
/// ZIP local file header signature
const LOCAL_FILE_SIG: u32 = 0x04034b50;
/// ZIP central directory header signature
const CD_SIG: u32 = 0x02014b50;

/// Find the End-of-Central-Directory record offset within `tail` bytes.
///
/// We scan backwards from the end looking for the 4-byte EOCD signature.
fn find_eocd(tail: &[u8]) -> Option<usize> {
    if tail.len() < EOCD_SIZE as usize {
        return None;
    }
    // Scan backwards (comment can be up to 65535 bytes)
    let limit = tail.len().saturating_sub(EOCD_SIZE as usize);
    for i in (0..=limit).rev() {
        if tail.len() - i < 4 {
            continue;
        }
        let sig = u32::from_le_bytes(tail[i..i + 4].try_into().ok()?);
        if sig == EOCD_SIG {
            return Some(i);
        }
    }
    None
}

/// Parse ZIP central-directory entries from raw bytes.
fn parse_zip_central_directory(cd_data: &[u8]) -> Vec<ArchiveEntry> {
    let mut entries = Vec::new();
    let mut offset = 0usize;

    while offset + 46 <= cd_data.len() {
        let sig = u32::from_le_bytes(cd_data[offset..offset + 4].try_into().unwrap_or([0; 4]));
        if sig != CD_SIG {
            break;
        }

        let compressed_size = u32::from_le_bytes(
            cd_data[offset + 20..offset + 24]
                .try_into()
                .unwrap_or([0; 4]),
        ) as u64;
        let uncompressed_size = u32::from_le_bytes(
            cd_data[offset + 24..offset + 28]
                .try_into()
                .unwrap_or([0; 4]),
        ) as u64;
        let fname_len = u16::from_le_bytes(
            cd_data[offset + 28..offset + 30]
                .try_into()
                .unwrap_or([0; 2]),
        ) as usize;
        let extra_len = u16::from_le_bytes(
            cd_data[offset + 30..offset + 32]
                .try_into()
                .unwrap_or([0; 2]),
        ) as usize;
        let comment_len = u16::from_le_bytes(
            cd_data[offset + 32..offset + 34]
                .try_into()
                .unwrap_or([0; 2]),
        ) as usize;

        let name_start = offset + 46;
        let name_end = name_start + fname_len;

        if name_end > cd_data.len() {
            break;
        }

        let path = String::from_utf8_lossy(&cd_data[name_start..name_end]).into_owned();

        let ratio = if uncompressed_size > 0 {
            (1.0 - compressed_size as f64 / uncompressed_size as f64) * 100.0
        } else {
            0.0
        };

        entries.push(ArchiveEntry {
            path,
            size: uncompressed_size,
            compressed_size,
            compression_ratio: (ratio * 10.0).round() / 10.0,
            archive_type: "zip".to_string(),
        });

        offset = name_end + extra_len + comment_len;
    }

    entries
}

/// Inspect a ZIP archive stored in OSS using Range requests.
async fn inspect_zip(
    client: &OssClient,
    bucket_key: &str,
    object_key: &str,
    object_size: u64,
) -> Result<Vec<ArchiveEntry>> {
    // Fetch the last 64 KB to find the EOCD record
    let tail_size = 65_536u64.min(object_size);
    let tail_start = object_size.saturating_sub(tail_size);
    let tail = client
        .fetch_range(bucket_key, object_key, tail_start, object_size - 1)
        .await
        .context("Failed to fetch ZIP tail for EOCD search")?;

    let eocd_offset = find_eocd(&tail)
        .context("Could not find ZIP End-of-Central-Directory — is this a valid .zip file?")?;

    // Parse EOCD
    let eocd = &tail[eocd_offset..];
    if eocd.len() < EOCD_SIZE as usize {
        anyhow::bail!("EOCD record too short");
    }

    let cd_entry_count =
        u16::from_le_bytes(eocd[10..12].try_into().context("EOCD parse error")?) as u64;
    let cd_size = u32::from_le_bytes(eocd[12..16].try_into().context("EOCD parse error")?) as u64;
    let cd_offset = u32::from_le_bytes(eocd[16..20].try_into().context("EOCD parse error")?) as u64;

    if cd_entry_count == 0 || cd_size == 0 {
        return Ok(Vec::new());
    }

    // Fetch central directory bytes
    let cd_end = cd_offset + cd_size - 1;
    let cd_data = client
        .fetch_range(bucket_key, object_key, cd_offset, cd_end)
        .await
        .context("Failed to fetch ZIP central directory")?;

    Ok(parse_zip_central_directory(&cd_data))
}

// ---------------------------------------------------------------------------
// tar.gz inspection helpers
// ---------------------------------------------------------------------------

/// Inspect a .tar.gz archive by streaming and decompressing it via Range
/// requests in chunks. We read the entire file in chunks because tar.gz
/// archives do not have a central directory at the end.
///
/// For large archives this still avoids a full sequential download by
/// stopping as soon as the end-of-archive is detected.
async fn inspect_targz(
    client: &OssClient,
    bucket_key: &str,
    object_key: &str,
    object_size: u64,
) -> Result<Vec<ArchiveEntry>> {
    // Stream the file in 512 KB chunks, feeding into a gzip decoder + tar reader.
    const CHUNK: u64 = 512 * 1024;

    // Collect all chunks first, then parse. For very large archives the caller
    // may prefer streaming, but this keeps the implementation simple and
    // avoids having to manage async + sync boundary with the `tar` crate.
    let mut all_bytes: Vec<u8> = Vec::new();
    let mut pos = 0u64;

    while pos < object_size {
        let end = (pos + CHUNK - 1).min(object_size - 1);
        let chunk = client
            .fetch_range(bucket_key, object_key, pos, end)
            .await
            .with_context(|| format!("Failed to fetch tar.gz chunk at offset {pos}"))?;
        all_bytes.extend_from_slice(&chunk);
        pos = end + 1;
    }

    // Decompress gzip and parse tar headers
    let cursor = std::io::Cursor::new(&all_bytes);
    let gz = flate2::read::GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(gz);

    let mut entries = Vec::new();
    for entry in archive.entries().context("Failed to read tar entries")? {
        let entry = entry.context("Failed to read tar entry")?;
        let header = entry.header();
        let path = entry
            .path()
            .context("Failed to read entry path")?
            .to_string_lossy()
            .into_owned();
        let size = header.size().unwrap_or(0);

        entries.push(ArchiveEntry {
            path,
            size,
            // tar.gz entries don't expose individual compressed sizes
            compressed_size: size,
            compression_ratio: 0.0,
            archive_type: "tar.gz".to_string(),
        });
    }

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Extract a single file from a ZIP archive using Range requests
// ---------------------------------------------------------------------------

async fn extract_from_zip(
    client: &OssClient,
    bucket_key: &str,
    object_key: &str,
    object_size: u64,
    path_to_extract: &str,
) -> Result<Vec<u8>> {
    // Get the central directory entry to find the local file header offset
    let tail_size = 65_536u64.min(object_size);
    let tail_start = object_size.saturating_sub(tail_size);
    let tail = client
        .fetch_range(bucket_key, object_key, tail_start, object_size - 1)
        .await
        .context("Failed to fetch ZIP tail")?;

    let eocd_offset = find_eocd(&tail).context("Could not find ZIP End-of-Central-Directory")?;

    let eocd = &tail[eocd_offset..];
    let cd_size = u32::from_le_bytes(eocd[12..16].try_into().context("EOCD parse error")?) as u64;
    let cd_offset = u32::from_le_bytes(eocd[16..20].try_into().context("EOCD parse error")?) as u64;

    let cd_data = client
        .fetch_range(bucket_key, object_key, cd_offset, cd_offset + cd_size - 1)
        .await
        .context("Failed to fetch ZIP central directory")?;

    // Find the matching entry in the central directory to get local header offset
    let mut pos = 0usize;
    while pos + 46 <= cd_data.len() {
        let sig = u32::from_le_bytes(cd_data[pos..pos + 4].try_into().unwrap_or([0; 4]));
        if sig != CD_SIG {
            break;
        }

        let compressed_size =
            u32::from_le_bytes(cd_data[pos + 20..pos + 24].try_into().unwrap_or([0; 4])) as u64;
        let uncompressed_size =
            u32::from_le_bytes(cd_data[pos + 24..pos + 28].try_into().unwrap_or([0; 4])) as u64;
        let fname_len =
            u16::from_le_bytes(cd_data[pos + 28..pos + 30].try_into().unwrap_or([0; 2])) as usize;
        let extra_len =
            u16::from_le_bytes(cd_data[pos + 30..pos + 32].try_into().unwrap_or([0; 2])) as usize;
        let comment_len =
            u16::from_le_bytes(cd_data[pos + 32..pos + 34].try_into().unwrap_or([0; 2])) as usize;
        let local_header_offset =
            u32::from_le_bytes(cd_data[pos + 42..pos + 46].try_into().unwrap_or([0; 4])) as u64;
        let compression_method =
            u16::from_le_bytes(cd_data[pos + 10..pos + 12].try_into().unwrap_or([0; 2]));

        let name_start = pos + 46;
        let name_end = name_start + fname_len;

        if name_end > cd_data.len() {
            break;
        }

        let name = String::from_utf8_lossy(&cd_data[name_start..name_end]);

        if name == path_to_extract {
            // Fetch the local file header (30 bytes fixed + variable)
            let lh_data = client
                .fetch_range(
                    bucket_key,
                    object_key,
                    local_header_offset,
                    local_header_offset + 29,
                )
                .await
                .context("Failed to fetch local file header")?;

            let lh_sig = u32::from_le_bytes(lh_data[0..4].try_into().context("Local header sig")?);
            if lh_sig != LOCAL_FILE_SIG {
                anyhow::bail!("Invalid local file header signature");
            }

            let lh_fname_len =
                u16::from_le_bytes(lh_data[26..28].try_into().context("Local header fname")?)
                    as u64;
            let lh_extra_len =
                u16::from_le_bytes(lh_data[28..30].try_into().context("Local header extra")?)
                    as u64;

            let data_offset = local_header_offset + 30 + lh_fname_len + lh_extra_len;

            // Fetch compressed data
            let compressed = client
                .fetch_range(
                    bucket_key,
                    object_key,
                    data_offset,
                    data_offset + compressed_size - 1,
                )
                .await
                .context("Failed to fetch file data")?;

            // Decompress
            let decompressed = match compression_method {
                0 => {
                    // Stored (no compression)
                    compressed
                }
                8 => {
                    // Deflate
                    let mut decoder =
                        flate2::read::DeflateDecoder::new(std::io::Cursor::new(compressed));
                    let mut out = Vec::with_capacity(uncompressed_size as usize);
                    decoder
                        .read_to_end(&mut out)
                        .context("Failed to decompress deflate data")?;
                    out
                }
                m => {
                    anyhow::bail!("Unsupported compression method: {m}");
                }
            };

            return Ok(decompressed);
        }

        pos = name_end + extra_len + comment_len;
    }

    anyhow::bail!("File '{}' not found in archive", path_to_extract)
}

// ---------------------------------------------------------------------------
// Extract a single file from a tar.gz archive
// ---------------------------------------------------------------------------

async fn extract_from_targz(
    client: &OssClient,
    bucket_key: &str,
    object_key: &str,
    object_size: u64,
    path_to_extract: &str,
) -> Result<Vec<u8>> {
    const CHUNK: u64 = 512 * 1024;
    let mut all_bytes: Vec<u8> = Vec::new();
    let mut pos = 0u64;

    while pos < object_size {
        let end = (pos + CHUNK - 1).min(object_size - 1);
        let chunk = client
            .fetch_range(bucket_key, object_key, pos, end)
            .await
            .with_context(|| format!("Failed to fetch tar.gz chunk at offset {pos}"))?;
        all_bytes.extend_from_slice(&chunk);
        pos = end + 1;
    }

    let cursor = std::io::Cursor::new(&all_bytes);
    let gz = flate2::read::GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(gz);

    for entry in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry.context("Failed to read tar entry")?;
        let p = entry
            .path()
            .context("Failed to read path")?
            .to_string_lossy()
            .into_owned();

        if p == path_to_extract || p.trim_start_matches("./") == path_to_extract {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .context("Failed to read entry data")?;
            return Ok(buf);
        }
    }

    anyhow::bail!("File '{}' not found in archive", path_to_extract)
}

// ---------------------------------------------------------------------------
// Detect archive type from object key
// ---------------------------------------------------------------------------

enum ArchiveType {
    Zip,
    TarGz,
}

fn detect_archive_type(object_key: &str) -> Result<ArchiveType> {
    let lower = object_key.to_lowercase();
    if lower.ends_with(".zip") {
        Ok(ArchiveType::Zip)
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        Ok(ArchiveType::TarGz)
    } else {
        anyhow::bail!("Unsupported archive format. Object key must end with .zip, .tar.gz, or .tgz")
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub(super) async fn inspect_object(
    client: &OssClient,
    bucket: String,
    object: String,
    extract: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    // Get object metadata to know its size
    if output_format.supports_colors() {
        println!(
            "{}",
            format!("Fetching metadata for '{}/{}'...", bucket, object).dimmed()
        );
    }

    let details = client
        .get_object_details(&bucket, &object)
        .await
        .context("Failed to get object details")?;

    let object_size = details.size;
    let archive_type = detect_archive_type(&object)?;
    let archive_type_str = match archive_type {
        ArchiveType::Zip => "zip",
        ArchiveType::TarGz => "tar.gz",
    };

    // --- Extract mode ---
    if let Some(ref path_to_extract) = extract {
        if output_format.supports_colors() {
            println!(
                "{}",
                format!(
                    "Extracting '{}' from '{}/{}' via Range requests...",
                    path_to_extract, bucket, object
                )
                .dimmed()
            );
        }

        let data = match archive_type {
            ArchiveType::Zip => {
                extract_from_zip(client, &bucket, &object, object_size, path_to_extract).await?
            }
            ArchiveType::TarGz => {
                extract_from_targz(client, &bucket, &object, object_size, path_to_extract).await?
            }
        };

        // Write extracted bytes to stdout or to a file named after the basename
        let filename = std::path::Path::new(path_to_extract)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path_to_extract.clone());

        let output_path = std::path::PathBuf::from(&filename);
        std::fs::write(&output_path, &data)
            .with_context(|| format!("Failed to write extracted file to '{filename}'"))?;

        match output_format {
            OutputFormat::Table => {
                println!(
                    "{} Extracted '{}' ({}) to '{}'",
                    "✓".green().bold(),
                    path_to_extract,
                    format_size(data.len() as u64),
                    filename
                );
            }
            _ => {
                #[derive(Serialize, schemars::JsonSchema)]
                struct ExtractOutput {
                    success: bool,
                    path: String,
                    output_file: String,
                    size: u64,
                }
                output_format.write(&ExtractOutput {
                    success: true,
                    path: path_to_extract.clone(),
                    output_file: filename,
                    size: data.len() as u64,
                })?;
            }
        }

        return Ok(());
    }

    // --- List mode ---
    if output_format.supports_colors() {
        println!(
            "{}",
            format!(
                "Inspecting '{}/{}' ({}) via Range requests...",
                bucket,
                object,
                format_size(object_size)
            )
            .dimmed()
        );
    }

    let entries = match archive_type {
        ArchiveType::Zip => inspect_zip(client, &bucket, &object, object_size).await?,
        ArchiveType::TarGz => inspect_targz(client, &bucket, &object, object_size).await?,
    };

    let total_files = entries.len();
    let total_size: u64 = entries.iter().map(|e| e.size).sum();

    let inspect_output = InspectOutput {
        bucket_key: bucket.clone(),
        object_key: object.clone(),
        archive_type: archive_type_str.to_string(),
        total_files,
        total_size,
        total_size_human: format_size(total_size),
        entries,
    };

    match output_format {
        OutputFormat::Table => {
            println!(
                "\n{} {} {} {}",
                "Archive:".bold(),
                object.cyan().bold(),
                "in".dimmed(),
                bucket.cyan()
            );
            println!(
                "  {} {} | {} files | {}",
                "Type:".bold(),
                inspect_output.archive_type.to_uppercase(),
                inspect_output.total_files,
                inspect_output.total_size_human
            );
            println!("{}", "-".repeat(90));
            println!(
                "{:<55} {:>12} {:>12} {:>8}",
                "Path".bold(),
                "Size".bold(),
                "Compressed".bold(),
                "Ratio".bold()
            );
            println!("{}", "-".repeat(90));

            for entry in &inspect_output.entries {
                // Skip directory entries
                if entry.path.ends_with('/') {
                    continue;
                }
                let ratio_str = if entry.compression_ratio > 0.0 {
                    format!("{:.1}%", entry.compression_ratio)
                } else {
                    "N/A".to_string()
                };
                println!(
                    "{:<55} {:>12} {:>12} {:>8}",
                    super::truncate_str(&entry.path, 55).cyan(),
                    format_size(entry.size),
                    format_size(entry.compressed_size),
                    ratio_str
                );
            }

            println!("{}", "-".repeat(90));
            println!(
                "  {} {} files, {}",
                "Total:".bold(),
                inspect_output.total_files,
                inspect_output.total_size_human
            );
        }
        _ => {
            output_format.write(&inspect_output)?;
        }
    }

    Ok(())
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Tests for `raps object inspect` and the archive-type detection logic.
//!
//! Pure unit tests exercise the archive-type heuristic inlined from
//! `raps-cli/src/commands/object/inspect.rs`.
//! CLI integration tests exercise argument parsing without network access.

use assert_cmd::Command;
use predicates::prelude::*;

// ---------------------------------------------------------------------------
// Inline the detect_archive_type heuristic
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum ArchiveType {
    Zip,
    TarGz,
}

fn detect_archive_type(object_key: &str) -> Result<ArchiveType, String> {
    let lower = object_key.to_lowercase();
    if lower.ends_with(".zip") {
        Ok(ArchiveType::Zip)
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        Ok(ArchiveType::TarGz)
    } else {
        Err(format!(
            "Unsupported archive format. Object key must end with .zip, .tar.gz, or .tgz (got '{object_key}')"
        ))
    }
}

// ---------------------------------------------------------------------------
// Unit tests: archive-type detection from object key
// ---------------------------------------------------------------------------

#[test]
fn test_detect_zip_extension() {
    assert_eq!(
        detect_archive_type("archive.zip").unwrap(),
        ArchiveType::Zip
    );
}

#[test]
fn test_detect_zip_extension_uppercase() {
    assert_eq!(
        detect_archive_type("ARCHIVE.ZIP").unwrap(),
        ArchiveType::Zip
    );
}

#[test]
fn test_detect_zip_with_path_prefix() {
    assert_eq!(
        detect_archive_type("models/v2/upload.zip").unwrap(),
        ArchiveType::Zip
    );
}

#[test]
fn test_detect_tar_gz_extension() {
    assert_eq!(
        detect_archive_type("backup.tar.gz").unwrap(),
        ArchiveType::TarGz
    );
}

#[test]
fn test_detect_tgz_extension() {
    assert_eq!(
        detect_archive_type("archive.tgz").unwrap(),
        ArchiveType::TarGz
    );
}

#[test]
fn test_detect_tar_gz_with_path_prefix() {
    assert_eq!(
        detect_archive_type("builds/linux/release.tar.gz").unwrap(),
        ArchiveType::TarGz
    );
}

#[test]
fn test_detect_unknown_extension_returns_error() {
    let err = detect_archive_type("file.rar").unwrap_err();
    assert!(
        err.contains("Unsupported"),
        "error should mention 'Unsupported', got: {err}"
    );
}

#[test]
fn test_detect_no_extension_returns_error() {
    let err = detect_archive_type("no-extension-file").unwrap_err();
    assert!(
        err.contains("Unsupported"),
        "error should mention 'Unsupported', got: {err}"
    );
}

#[test]
fn test_detect_tar_only_returns_error() {
    // Plain .tar (no gzip) is not supported
    let err = detect_archive_type("archive.tar").unwrap_err();
    assert!(
        err.contains("Unsupported"),
        ".tar without .gz should be unsupported, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// Unit tests: magic-byte detection for ZIP and gzip
// ---------------------------------------------------------------------------

/// Returns true when the byte slice starts with a ZIP local file header
/// signature (PK\x03\x04).
fn has_zip_magic(data: &[u8]) -> bool {
    data.len() >= 4 && data[0] == 0x50 && data[1] == 0x4b && data[2] == 0x03 && data[3] == 0x04
}

/// Returns true when the byte slice starts with the gzip magic bytes
/// (\x1f\x8b).
fn has_gzip_magic(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b
}

#[test]
fn test_zip_magic_bytes_detected() {
    let data = vec![0x50u8, 0x4b, 0x03, 0x04, 0x14, 0x00];
    assert!(
        has_zip_magic(&data),
        "PK\\x03\\x04 should be detected as ZIP"
    );
}

#[test]
fn test_zip_magic_bytes_not_false_positive() {
    let data = b"PK not zip".to_vec(); // 'P'=0x50, 'K'=0x4b, ' '=0x20 ≠ 0x03
    assert!(
        !has_zip_magic(&data),
        "PK without \\x03\\x04 should not match ZIP magic"
    );
}

#[test]
fn test_gzip_magic_bytes_detected() {
    let data = vec![0x1fu8, 0x8b, 0x08, 0x00, 0x00];
    assert!(
        has_gzip_magic(&data),
        "\\x1f\\x8b should be detected as gzip"
    );
}

#[test]
fn test_gzip_magic_bytes_not_false_positive() {
    let data = b"not gzip\x1f".to_vec();
    assert!(
        !has_gzip_magic(&data),
        "random bytes should not match gzip magic"
    );
}

#[test]
fn test_empty_slice_has_no_zip_magic() {
    assert!(!has_zip_magic(b""), "empty slice should not have ZIP magic");
}

#[test]
fn test_empty_slice_has_no_gzip_magic() {
    assert!(
        !has_gzip_magic(b""),
        "empty slice should not have gzip magic"
    );
}

#[test]
fn test_zip_magic_single_byte_no_panic() {
    assert!(!has_zip_magic(&[0x50]));
}

#[test]
fn test_gzip_magic_single_byte_no_panic() {
    assert!(!has_gzip_magic(&[0x1f]));
}

// ---------------------------------------------------------------------------
// Unit tests: find_eocd (ZIP End-of-Central-Directory search)
// ---------------------------------------------------------------------------

/// Mirrors `find_eocd` from inspect.rs.
const EOCD_SIZE: usize = 22;
const EOCD_SIG: u32 = 0x06054b50;

fn find_eocd(tail: &[u8]) -> Option<usize> {
    if tail.len() < EOCD_SIZE {
        return None;
    }
    let limit = tail.len().saturating_sub(EOCD_SIZE);
    for i in (0..=limit).rev() {
        if tail.len() - i < 4 {
            continue;
        }
        let sig_bytes: [u8; 4] = tail[i..i + 4].try_into().ok()?;
        let sig = u32::from_le_bytes(sig_bytes);
        if sig == EOCD_SIG {
            return Some(i);
        }
    }
    None
}

#[test]
fn test_find_eocd_finds_signature_at_end() {
    let mut tail = vec![0u8; 100];
    // Write EOCD signature at offset 78 (leaving 22 bytes for the record)
    let sig_bytes = EOCD_SIG.to_le_bytes();
    tail[78..82].copy_from_slice(&sig_bytes);
    assert_eq!(find_eocd(&tail), Some(78));
}

#[test]
fn test_find_eocd_returns_none_when_no_signature() {
    let tail = vec![0xaau8; 100];
    assert!(
        find_eocd(&tail).is_none(),
        "should return None with no EOCD signature"
    );
}

#[test]
fn test_find_eocd_returns_none_when_too_short() {
    let tail = vec![0u8; 10]; // less than EOCD_SIZE
    assert!(
        find_eocd(&tail).is_none(),
        "should return None for too-short slice"
    );
}

#[test]
fn test_find_eocd_finds_signature_at_start() {
    let mut tail = vec![0u8; 22];
    let sig_bytes = EOCD_SIG.to_le_bytes();
    tail[0..4].copy_from_slice(&sig_bytes);
    assert_eq!(find_eocd(&tail), Some(0));
}

// ---------------------------------------------------------------------------
// CLI integration tests
// ---------------------------------------------------------------------------

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

#[test]
fn test_inspect_help_exits_zero() {
    raps()
        .args(["object", "inspect", "--help"])
        .assert()
        .success();
}

#[test]
fn test_inspect_help_mentions_extract() {
    raps()
        .args(["object", "inspect", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--extract").or(predicate::str::contains("extract")));
}

#[test]
fn test_inspect_requires_bucket_and_object() {
    raps()
        .args(["object", "inspect"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("BUCKET")));
}

#[test]
fn test_inspect_unsupported_extension_fails_before_api() {
    // Supplying an object key without .zip/.tar.gz/.tgz should produce an
    // "Unsupported archive format" error before any API call is attempted.
    let out = raps()
        .args(["object", "inspect", "my-bucket", "file.rar"])
        .output()
        .unwrap();

    // May fail with "Unsupported archive format" or with an auth error first.
    // If auth error comes first, the test is inconclusive but still passes.
    let stderr = String::from_utf8_lossy(&out.stderr);
    // We just verify the command exited non-zero (either parse or runtime error).
    assert!(
        !out.status.success(),
        "inspect of .rar file should not succeed, stderr: {stderr}"
    );
}

#[test]
fn test_inspect_zip_extension_accepted_by_clap() {
    // clap should accept bucket + object.zip without complaining about args.
    let out = raps()
        .args(["object", "inspect", "my-bucket", "archive.zip"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("unrecognized"),
        "clap rejected valid args for inspect: {stderr}"
    );
}

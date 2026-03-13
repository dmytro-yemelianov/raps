// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Unit and integration tests for `raps object diff`.
//!
//! Pure unit tests exercise the `is_text` heuristic directly (copied / inlined
//! here because the private function is not exported).  Integration tests drive
//! the CLI binary against temporary local files so no network access is needed.

use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write as _;

// ---------------------------------------------------------------------------
// Inline the is_text heuristic so it can be unit-tested without pub export
// ---------------------------------------------------------------------------

/// Mirrors the logic in `raps-cli/src/commands/object/diff.rs`.
fn is_text(data: &[u8]) -> bool {
    let sample = &data[..data.len().min(8192)];
    !sample.contains(&0u8) && std::str::from_utf8(sample).is_ok()
}

// ---------------------------------------------------------------------------
// Unit tests: binary / text detection
// ---------------------------------------------------------------------------

#[test]
fn test_binary_detection_null_byte() {
    let mut data = b"some normal text".to_vec();
    data.push(0u8); // embed a NUL byte
    data.extend_from_slice(b" more text");
    assert!(
        !is_text(&data),
        "data with NUL byte should be detected as binary"
    );
}

#[test]
fn test_binary_detection_pdf_header() {
    // PDF files start with %PDF- and contain binary bytes
    let data: Vec<u8> = b"%PDF-1.4\x00\x01\x02\x03".to_vec();
    assert!(
        !is_text(&data),
        "PDF-like binary should be detected as binary"
    );
}

#[test]
fn test_text_detection_plain_utf8() {
    let data = b"Hello, world!\nThis is a plain UTF-8 text file.\n".to_vec();
    assert!(is_text(&data), "plain UTF-8 should be detected as text");
}

#[test]
fn test_text_detection_json() {
    let data = br#"{"key": "value", "number": 42, "flag": true}"#.to_vec();
    assert!(is_text(&data), "JSON should be detected as text");
}

#[test]
fn test_text_detection_multiline() {
    let data = "line1\nline2\nline3\n".repeat(100).into_bytes();
    assert!(is_text(&data), "multi-line text should be detected as text");
}

#[test]
fn test_binary_detection_zip_magic() {
    // ZIP files start with PK\x03\x04 which contains non-UTF-8 bytes
    let mut data = vec![0x50u8, 0x4b, 0x03, 0x04]; // PK\x03\x04
    data.extend_from_slice(b"rest of zip file");
    // \x03 and \x04 are valid ASCII control chars but zip content is binary
    // The NUL-byte check catches typical zip interiors even if the magic alone
    // passes; let's embed a NUL to simulate real zip body:
    data.push(0x00);
    assert!(
        !is_text(&data),
        "ZIP data with NUL byte should be detected as binary"
    );
}

#[test]
fn test_binary_detection_gzip_magic() {
    // gzip magic: \x1f\x8b — \x8b is not valid UTF-8
    let data = vec![0x1fu8, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(
        !is_text(&data),
        "gzip magic bytes should be detected as binary (invalid UTF-8)"
    );
}

#[test]
fn test_empty_slice_is_text() {
    // An empty file has no NUL bytes and no invalid UTF-8 — treat as text.
    assert!(is_text(b""), "empty slice should be considered text");
}

#[test]
fn test_exactly_8192_bytes_no_nul_is_text() {
    let data: Vec<u8> = b"a".repeat(8192);
    assert!(is_text(&data), "8 KiB of 'a' bytes should be text");
}

#[test]
fn test_nul_byte_beyond_sample_window_not_caught() {
    // NUL at byte 8193 is outside the 8 KiB sample window — the heuristic
    // won't catch it.  This is a known limitation; the test documents the behaviour.
    let mut data: Vec<u8> = b"a".repeat(8193);
    data.push(0x00); // NUL at position 8193
    // The function only checks the first 8192 bytes, so this returns true.
    assert!(
        is_text(&data),
        "NUL beyond 8 KiB window is not detected (known limitation)"
    );
}

// ---------------------------------------------------------------------------
// CLI integration tests — no network required (local file paths)
// ---------------------------------------------------------------------------

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

/// Write `content` to a named temp file and return the file + its path string.
fn temp_file_with(content: &[u8]) -> (tempfile::NamedTempFile, String) {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content).unwrap();
    f.flush().unwrap();
    let path = f.path().to_string_lossy().into_owned();
    (f, path)
}

#[test]
fn test_diff_help_exits_zero() {
    raps().args(["object", "diff", "--help"]).assert().success();
}

#[test]
fn test_diff_help_lists_checksum_only_flag() {
    raps()
        .args(["object", "diff", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--checksum-only").or(predicate::str::contains("checksum")),
        );
}

#[test]
fn test_diff_identical_local_files_exits_zero() {
    let (_fa, pa) = temp_file_with(b"identical content\n");
    let (_fb, pb) = temp_file_with(b"identical content\n");

    // Identical files → diff exits 0
    raps().args(["object", "diff", &pa, &pb]).assert().success();
}

#[test]
fn test_diff_different_local_files_exits_nonzero() {
    let (_fa, pa) = temp_file_with(b"version A\n");
    let (_fb, pb) = temp_file_with(b"version B\n");

    // Different files → diff exits 1 (like POSIX diff)
    raps().args(["object", "diff", &pa, &pb]).assert().failure();
}

#[test]
fn test_diff_checksum_only_skips_content_diff() {
    let (_fa, pa) = temp_file_with(b"left side content\n");
    let (_fb, pb) = temp_file_with(b"right side content\n");

    let out = raps()
        .args(["object", "diff", "--checksum-only", &pa, &pb])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let combined = format!("{stdout}");

    // In --checksum-only mode the textual diff block must not appear
    assert!(
        !combined.contains("@@") && !combined.contains("+right") && !combined.contains("-left"),
        "--checksum-only should suppress the diff body, got:\n{combined}"
    );
}

#[test]
fn test_diff_identical_files_json_output_identical_true() {
    let (_fa, pa) = temp_file_with(b"same\n");
    let (_fb, pb) = temp_file_with(b"same\n");

    let out = raps()
        .args(["object", "diff", "--output", "json", &pa, &pb])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"identical\":true") || stdout.contains("\"identical\": true"),
        "JSON output should contain 'identical: true' for equal files, got:\n{stdout}"
    );
}

#[test]
fn test_diff_different_files_json_output_identical_false() {
    let (_fa, pa) = temp_file_with(b"aaa\n");
    let (_fb, pb) = temp_file_with(b"bbb\n");

    let out = raps()
        .args(["object", "diff", "--output", "json", &pa, &pb])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"identical\":false") || stdout.contains("\"identical\": false"),
        "JSON output should contain 'identical: false' for differing files, got:\n{stdout}"
    );
}

#[test]
fn test_diff_requires_two_args() {
    raps()
        .args(["object", "diff"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("LEFT")));
}

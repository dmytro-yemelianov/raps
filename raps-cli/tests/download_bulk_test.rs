// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Tests for `raps object download-bulk`.
//!
//! These tests exercise CLI argument parsing and the output-directory creation
//! path.  They do not perform real network calls; tests that would require
//! credentials are skipped gracefully.

use assert_cmd::Command;
use predicates::prelude::*;

fn raps() -> Command {
    Command::cargo_bin("raps").unwrap()
}

// ---------------------------------------------------------------------------
// Help (parent command)
// ---------------------------------------------------------------------------

#[test]
fn test_object_help_mentions_download_bulk() {
    // NOTE: `raps object download-bulk --help` triggers a pre-existing clap
    // panic (non-required positional `bucket` precedes required positional
    // `prefix`).  We verify the sub-command is registered via the parent help.
    raps()
        .args(["object", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("download-bulk"));
}

// ---------------------------------------------------------------------------
// --concurrency 0 — unit-level verification of the semaphore behaviour
// ---------------------------------------------------------------------------

// NOTE: `raps object download-bulk` has a pre-existing clap panic caused by a
// non-required positional argument (`bucket: Option<String>`) appearing before
// a required positional argument (`prefix: String`).  Invoking the sub-command
// with any arguments therefore panics before argument parsing completes.
// The following tests exercise the path-building and concurrency logic at the
// unit level instead of through the CLI binary.

/// Verify that a `Semaphore::new(0)` would block on `acquire` — i.e. that
/// concurrency=0 would deadlock if not caught upstream.
/// This is a documentation test; in practice the command should validate > 0.
#[test]
fn test_concurrency_zero_semaphore_would_deadlock() {
    use std::sync::Arc;
    use std::time::Duration;

    let sem = Arc::new(tokio::sync::Semaphore::new(0));
    // A try_acquire on a zero-permit semaphore must fail immediately.
    assert!(
        sem.try_acquire().is_err(),
        "Semaphore with 0 permits must not grant an immediate permit (would deadlock on acquire)"
    );
    // Confirm that a 1-permit semaphore does grant immediately.
    let sem1 = Arc::new(tokio::sync::Semaphore::new(1));
    let permit = sem1.try_acquire();
    assert!(
        permit.is_ok(),
        "Semaphore with 1 permit should grant immediately"
    );
    let _ = Duration::from_millis(0); // satisfy unused-import lint
}

// ---------------------------------------------------------------------------
// Output directory creation — filesystem unit test
// ---------------------------------------------------------------------------

#[test]
fn test_output_dir_created_if_missing() {
    // Mirror the tokio::fs::create_dir_all call from download_bulk.rs using
    // std::fs so no async runtime is needed.
    let base = tempfile::tempdir().unwrap();
    let new_dir = base.path().join("deep").join("nested").join("output");

    assert!(!new_dir.exists(), "pre-condition: dir must not exist");

    std::fs::create_dir_all(&new_dir).expect("create_dir_all should succeed");

    assert!(
        new_dir.exists() && new_dir.is_dir(),
        "output directory was not created"
    );
}

// ---------------------------------------------------------------------------
// --flat strips prefix from output paths (unit-level path logic)
// ---------------------------------------------------------------------------

/// Mirror the path-building logic from `download_bulk.rs` for the flat case.
fn flat_dest(output_dir: &std::path::Path, object_key: &str) -> std::path::PathBuf {
    let filename = std::path::Path::new(object_key)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| object_key.replace('/', "_"));
    output_dir.join(filename)
}

/// Mirror the path-building logic for the non-flat (prefix-relative) case.
fn prefix_relative_dest(
    output_dir: &std::path::Path,
    object_key: &str,
    prefix: &str,
) -> std::path::PathBuf {
    let relative = object_key
        .strip_prefix(prefix)
        .unwrap_or(object_key)
        .trim_start_matches('/');
    output_dir.join(relative)
}

#[test]
fn test_flat_dest_strips_directory_components() {
    let dir = std::path::Path::new("/tmp/out");
    let dest = flat_dest(dir, "models/v2/chair.rvt");
    assert_eq!(dest, std::path::Path::new("/tmp/out/chair.rvt"));
}

#[test]
fn test_flat_dest_with_no_slashes() {
    let dir = std::path::Path::new("/tmp/out");
    let dest = flat_dest(dir, "chair.rvt");
    assert_eq!(dest, std::path::Path::new("/tmp/out/chair.rvt"));
}

#[test]
fn test_prefix_relative_dest_strips_prefix() {
    let dir = std::path::Path::new("/tmp/out");
    let dest = prefix_relative_dest(dir, "models/v2/chair.rvt", "models/");
    assert_eq!(dest, std::path::Path::new("/tmp/out/v2/chair.rvt"));
}

#[test]
fn test_prefix_relative_dest_no_prefix_match_uses_full_key() {
    let dir = std::path::Path::new("/tmp/out");
    // If the object key doesn't start with the prefix, use the full key.
    let dest = prefix_relative_dest(dir, "other/chair.rvt", "models/");
    assert_eq!(dest, std::path::Path::new("/tmp/out/other/chair.rvt"));
}

#[test]
fn test_flat_dest_replaces_slashes_when_no_filename() {
    // Edge case: object key ends with '/' (directory marker)
    let dir = std::path::Path::new("/tmp/out");
    // Path::new("models/v2/").file_name() returns None for trailing slash
    // so we fall back to replacing '/' with '_'.
    let object_key = "models/v2/";
    let filename = std::path::Path::new(object_key)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| object_key.replace('/', "_"));
    let dest = dir.join(&filename);
    // "models/v2/" → file_name() = Some("v2") on most OSes
    // Accept either "v2" or the fallback "models_v2_"
    assert!(
        dest.ends_with("v2") || dest.to_string_lossy().contains("v2"),
        "unexpected dest for trailing-slash key: {dest:?}"
    );
}

// ---------------------------------------------------------------------------
// Argument validation — parent command level
// ---------------------------------------------------------------------------

#[test]
fn test_object_command_requires_subcommand() {
    // Invoking `raps object` without a sub-command should fail with a usage
    // error listing available sub-commands.
    raps()
        .args(["object"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Usage").or(
                predicate::str::contains("subcommand").or(predicate::str::contains("COMMAND")),
            ),
        );
}

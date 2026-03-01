// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Content-addressed download cache with hardlink materialization.
//!
//! Stores downloaded artifacts by SHA-1 hash so repeated downloads
//! become near-instant hardlink or copy operations.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};

// ---------------------------------------------------------------------------
// Global cache state
// ---------------------------------------------------------------------------

static CACHE_ENABLED: AtomicBool = AtomicBool::new(true);
static OFFLINE_MODE: AtomicBool = AtomicBool::new(false);
static REFRESH_MODE: AtomicBool = AtomicBool::new(false);

/// Initialize global cache flags.
pub fn init(enabled: bool, offline: bool, refresh: bool) {
    CACHE_ENABLED.store(enabled, Ordering::Relaxed);
    OFFLINE_MODE.store(offline, Ordering::Relaxed);
    REFRESH_MODE.store(refresh, Ordering::Relaxed);
}

pub fn is_enabled() -> bool {
    CACHE_ENABLED.load(Ordering::Relaxed)
}

pub fn is_offline() -> bool {
    OFFLINE_MODE.load(Ordering::Relaxed)
}

pub fn is_refresh() -> bool {
    REFRESH_MODE.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Cache directory
// ---------------------------------------------------------------------------

static CACHE_DIR_OVERRIDE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Set a custom cache directory (call before any cache operations).
pub fn set_cache_dir(dir: PathBuf) {
    let _ = CACHE_DIR_OVERRIDE.set(dir);
}

/// Get the cache root directory.
pub fn cache_dir() -> Result<PathBuf> {
    if let Some(dir) = CACHE_DIR_OVERRIDE.get() {
        return Ok(dir.clone());
    }
    let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
        .context("Failed to determine project directories")?;
    Ok(proj_dirs.cache_dir().join("downloads"))
}

// ---------------------------------------------------------------------------
// Cache operations
// ---------------------------------------------------------------------------

/// Return the path inside the cache for a given SHA-1 hash.
/// Uses a two-level directory structure: `ab/abcdef0123...`
fn blob_path(sha1: &str) -> Result<PathBuf> {
    if sha1.len() < 4 {
        anyhow::bail!("SHA-1 hash too short: {}", sha1);
    }
    let dir = cache_dir()?;
    Ok(dir.join(&sha1[..2]).join(sha1))
}

/// Check whether an artifact with the given SHA-1 is already cached.
pub fn contains(sha1: &str) -> bool {
    if !is_enabled() || is_refresh() {
        return false;
    }
    blob_path(sha1).map(|p| p.exists()).unwrap_or(false)
}

/// Store a file in the cache under its SHA-1 hash.
/// The source file is *copied* into the cache (the caller keeps the original).
pub fn store(sha1: &str, source: &Path) -> Result<()> {
    if !is_enabled() {
        return Ok(());
    }
    let target = blob_path(sha1)?;
    if target.exists() {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, &target).context("Failed to copy file into cache")?;
    Ok(())
}

/// Materialize a cached artifact to a destination path.
///
/// Tries hard-link first (zero-copy); falls back to regular copy.
/// Returns `true` if the artifact was found in cache and materialized.
pub fn materialize(sha1: &str, dest: &Path) -> Result<bool> {
    if !is_enabled() || is_refresh() {
        return Ok(false);
    }
    let src = blob_path(sha1)?;
    if !src.exists() {
        if is_offline() {
            anyhow::bail!(
                "Artifact not in cache (sha1={}) and --offline mode is active",
                sha1
            );
        }
        return Ok(false);
    }

    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Try hardlink first, fall back to copy
    if std::fs::hard_link(&src, dest).is_ok() {
        return Ok(true);
    }
    std::fs::copy(&src, dest).context("Failed to copy cached artifact")?;
    Ok(true)
}

/// Return cache statistics: number of entries and total size.
pub fn stats() -> Result<(usize, u64)> {
    let dir = cache_dir()?;
    if !dir.exists() {
        return Ok((0, 0));
    }
    let mut count = 0usize;
    let mut total_size = 0u64;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            for inner in std::fs::read_dir(entry.path())? {
                let inner = inner?;
                if inner.file_type()?.is_file() {
                    count += 1;
                    total_size += inner.metadata()?.len();
                }
            }
        }
    }
    Ok((count, total_size))
}

/// Remove all cached artifacts.
pub fn clear() -> Result<usize> {
    let dir = cache_dir()?;
    if !dir.exists() {
        return Ok(0);
    }
    let mut removed = 0usize;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            for inner in std::fs::read_dir(entry.path())? {
                let inner = inner?;
                if inner.file_type()?.is_file() {
                    std::fs::remove_file(inner.path())?;
                    removed += 1;
                }
            }
            // Remove empty directory
            let _ = std::fs::remove_dir(entry.path());
        }
    }
    Ok(removed)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_cache() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let _ = CACHE_DIR_OVERRIDE.set(tmp.path().to_path_buf());
        init(true, false, false);
        tmp
    }

    #[test]
    fn test_store_and_contains() {
        let _tmp = setup_test_cache();
        let sha1 = "abcdef0123456789abcdef0123456789abcdef01";

        // Create a test file
        let src = _tmp.path().join("test.bin");
        let mut f = std::fs::File::create(&src).unwrap();
        f.write_all(b"hello cache").unwrap();

        assert!(!contains(sha1));
        store(sha1, &src).unwrap();
        assert!(contains(sha1));
    }

    #[test]
    fn test_materialize_hardlink_or_copy() {
        let _tmp = setup_test_cache();
        let sha1 = "1234567890abcdef1234567890abcdef12345678";

        let src = _tmp.path().join("original.txt");
        std::fs::write(&src, b"test data").unwrap();

        store(sha1, &src).unwrap();

        let dest = _tmp.path().join("output.txt");
        let hit = materialize(sha1, &dest).unwrap();
        assert!(hit);
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "test data");
    }

    #[test]
    fn test_materialize_miss() {
        let _tmp = setup_test_cache();
        let dest = _tmp.path().join("missing.txt");
        let hit = materialize("0000000000000000000000000000000000000000", &dest).unwrap();
        assert!(!hit);
    }

    #[test]
    fn test_stats_and_clear() {
        let _tmp = setup_test_cache();
        let sha1a = "aaaa567890abcdef1234567890abcdef12345678";
        let sha1b = "bbbb567890abcdef1234567890abcdef12345678";

        let src = _tmp.path().join("data.bin");
        std::fs::write(&src, b"12345").unwrap();

        store(sha1a, &src).unwrap();
        store(sha1b, &src).unwrap();

        let (count, size) = stats().unwrap();
        assert_eq!(count, 2);
        assert_eq!(size, 10); // 5 bytes each

        let removed = clear().unwrap();
        assert_eq!(removed, 2);

        let (count, _) = stats().unwrap();
        assert_eq!(count, 0);
    }
}

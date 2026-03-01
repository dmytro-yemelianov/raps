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

static CACHE_DIR_OVERRIDE: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// Set a custom cache directory (call before any cache operations).
pub fn set_cache_dir(dir: PathBuf) {
    *CACHE_DIR_OVERRIDE.lock().unwrap() = Some(dir);
}

/// Get the cache root directory.
pub fn cache_dir() -> Result<PathBuf> {
    if let Some(dir) = CACHE_DIR_OVERRIDE.lock().unwrap().as_ref() {
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

/// Remove cached artifacts older than the given duration.
/// Returns the number of entries removed.
pub fn prune_older_than(max_age: std::time::Duration) -> Result<usize> {
    let dir = cache_dir()?;
    if !dir.exists() {
        return Ok(0);
    }
    let now = std::time::SystemTime::now();
    let mut removed = 0usize;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            for inner in std::fs::read_dir(entry.path())? {
                let inner = inner?;
                if inner.file_type()?.is_file() {
                    let modified = inner.metadata()?.modified()?;
                    if let Ok(age) = now.duration_since(modified) {
                        if age > max_age {
                            std::fs::remove_file(inner.path())?;
                            removed += 1;
                        }
                    }
                }
            }
            // Remove directory if now empty
            let _ = std::fs::remove_dir(entry.path());
        }
    }
    Ok(removed)
}

/// Remove oldest cached artifacts until total size is under the given limit.
/// Returns the number of entries removed.
pub fn prune_to_size(max_bytes: u64) -> Result<usize> {
    let dir = cache_dir()?;
    if !dir.exists() {
        return Ok(0);
    }

    // Collect all entries with their sizes and modification times
    let mut entries: Vec<(std::path::PathBuf, u64, std::time::SystemTime)> = Vec::new();
    let mut total_size = 0u64;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            for inner in std::fs::read_dir(entry.path())? {
                let inner = inner?;
                if inner.file_type()?.is_file() {
                    let meta = inner.metadata()?;
                    let size = meta.len();
                    let modified = meta.modified()?;
                    total_size += size;
                    entries.push((inner.path(), size, modified));
                }
            }
        }
    }

    if total_size <= max_bytes {
        return Ok(0);
    }

    // Sort oldest first
    entries.sort_by_key(|(_, _, modified)| *modified);

    let mut removed = 0usize;
    for (path, size, _) in &entries {
        if total_size <= max_bytes {
            break;
        }
        std::fs::remove_file(path)?;
        total_size -= size;
        removed += 1;
        // Try to clean up parent dir if empty
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    }

    Ok(removed)
}

/// Parse a human-readable duration string (e.g. "30d", "7d", "2h", "90m").
pub fn parse_age(s: &str) -> Result<std::time::Duration> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("Empty duration string");
    }
    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: u64 = num_str
        .parse()
        .context("Invalid number in duration")?;
    match unit {
        "s" => Ok(std::time::Duration::from_secs(num)),
        "m" => Ok(std::time::Duration::from_secs(num * 60)),
        "h" => Ok(std::time::Duration::from_secs(num * 3600)),
        "d" => Ok(std::time::Duration::from_secs(num * 86400)),
        "w" => Ok(std::time::Duration::from_secs(num * 604800)),
        _ => anyhow::bail!("Unknown duration unit '{}'. Use s/m/h/d/w.", unit),
    }
}

/// Parse a human-readable size string (e.g. "1G", "500M", "100K").
pub fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("Empty size string");
    }
    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: u64 = num_str
        .parse()
        .context("Invalid number in size")?;
    match unit {
        "B" | "b" => Ok(num),
        "K" | "k" => Ok(num * 1024),
        "M" | "m" => Ok(num * 1024 * 1024),
        "G" | "g" => Ok(num * 1024 * 1024 * 1024),
        _ => {
            // Try parsing the whole string as bytes
            s.parse::<u64>()
                .context("Invalid size. Use a number with B/K/M/G suffix (e.g. 500M, 1G)")
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::MutexGuard;
    use tempfile::TempDir;

    // Cache tests mutate global CACHE_DIR_OVERRIDE, so they must run serially.
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn setup_test_cache() -> (TempDir, MutexGuard<'static, ()>) {
        let guard = TEST_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        set_cache_dir(tmp.path().to_path_buf());
        init(true, false, false);
        (tmp, guard)
    }

    #[test]
    fn test_store_and_contains() {
        let (tmp, _guard) = setup_test_cache();
        let sha1 = "abcdef0123456789abcdef0123456789abcdef01";

        let src = tmp.path().join("test.bin");
        let mut f = std::fs::File::create(&src).unwrap();
        f.write_all(b"hello cache").unwrap();

        assert!(!contains(sha1));
        store(sha1, &src).unwrap();
        assert!(contains(sha1));
    }

    #[test]
    fn test_materialize_hardlink_or_copy() {
        let (tmp, _guard) = setup_test_cache();
        let sha1 = "1234567890abcdef1234567890abcdef12345678";

        let src = tmp.path().join("original.txt");
        std::fs::write(&src, b"test data").unwrap();

        store(sha1, &src).unwrap();

        let dest = tmp.path().join("output.txt");
        let hit = materialize(sha1, &dest).unwrap();
        assert!(hit);
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "test data");
    }

    #[test]
    fn test_materialize_miss() {
        let (tmp, _guard) = setup_test_cache();
        let dest = tmp.path().join("missing.txt");
        let hit = materialize("0000000000000000000000000000000000000000", &dest).unwrap();
        assert!(!hit);
    }

    #[test]
    fn test_stats_and_clear() {
        let (tmp, _guard) = setup_test_cache();
        let sha1a = "aaaa567890abcdef1234567890abcdef12345678";
        let sha1b = "bbbb567890abcdef1234567890abcdef12345678";

        let src = tmp.path().join("data.bin");
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

    #[test]
    fn test_parse_age() {
        assert_eq!(parse_age("30s").unwrap().as_secs(), 30);
        assert_eq!(parse_age("5m").unwrap().as_secs(), 300);
        assert_eq!(parse_age("2h").unwrap().as_secs(), 7200);
        assert_eq!(parse_age("7d").unwrap().as_secs(), 604800);
        assert_eq!(parse_age("1w").unwrap().as_secs(), 604800);
        assert!(parse_age("").is_err());
        assert!(parse_age("abc").is_err());
        assert!(parse_age("5x").is_err());
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("100B").unwrap(), 100);
        assert_eq!(parse_size("1K").unwrap(), 1024);
        assert_eq!(parse_size("500M").unwrap(), 500 * 1024 * 1024);
        assert_eq!(parse_size("1G").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("1k").unwrap(), 1024);
        assert_eq!(parse_size("2g").unwrap(), 2 * 1024 * 1024 * 1024);
        assert_eq!(parse_size("4096").unwrap(), 4096);
        assert!(parse_size("").is_err());
        assert!(parse_size("abc").is_err());
    }

    #[test]
    fn test_prune_older_than() {
        let (tmp, _guard) = setup_test_cache();
        let sha1a = "cccc567890abcdef1234567890abcdef12345678";
        let sha1b = "dddd567890abcdef1234567890abcdef12345678";

        let src = tmp.path().join("data.bin");
        std::fs::write(&src, b"prune me").unwrap();

        store(sha1a, &src).unwrap();
        store(sha1b, &src).unwrap();

        // With a very large max_age, nothing should be pruned
        let removed = prune_older_than(std::time::Duration::from_secs(86400)).unwrap();
        assert_eq!(removed, 0);

        let (count, _) = stats().unwrap();
        assert_eq!(count, 2);

        // With zero max_age, everything should be pruned
        let removed = prune_older_than(std::time::Duration::from_secs(0)).unwrap();
        assert_eq!(removed, 2);

        let (count, _) = stats().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_prune_to_size() {
        let (tmp, _guard) = setup_test_cache();
        let sha1a = "eeee567890abcdef1234567890abcdef12345678";
        let sha1b = "ffff567890abcdef1234567890abcdef12345678";

        let src = tmp.path().join("big.bin");
        std::fs::write(&src, vec![0u8; 1000]).unwrap();

        store(sha1a, &src).unwrap();
        store(sha1b, &src).unwrap();

        let (count, size) = stats().unwrap();
        assert_eq!(count, 2);
        assert_eq!(size, 2000);

        // Prune to 1500 bytes — should remove one entry
        let removed = prune_to_size(1500).unwrap();
        assert_eq!(removed, 1);

        let (count, size) = stats().unwrap();
        assert_eq!(count, 1);
        assert_eq!(size, 1000);
    }
}

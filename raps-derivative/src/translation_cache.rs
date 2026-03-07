// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Local disk cache for completed translation jobs.
//!
//! Keyed by `(urn, output_format)` to avoid re-submitting a job that has
//! already succeeded.  The cache is stored as JSON in the platform-specific
//! cache directory (`~/.cache/com.autodesk.raps/` on Linux).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single cached translation entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTranslation {
    /// The manifest URN returned by the translation response.
    pub manifest_urn: String,
    /// Terminal status that was cached (e.g. `"success"`).
    pub status: String,
    /// Unix timestamp (seconds) when this entry was written.
    pub cached_at: i64,
    /// Output format that was requested (e.g. `"svf2"`).
    pub output_format: String,
}

/// On-disk translation deduplication cache.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TranslationCache {
    /// Key: `"<urn>::<output_format>"`, value: cached entry.
    pub entries: HashMap<String, CachedTranslation>,
}

impl TranslationCache {
    /// Return the path to the cache file, or `None` if the platform does not
    /// expose a suitable cache directory.
    fn cache_path() -> Option<std::path::PathBuf> {
        let dirs = directories::ProjectDirs::from("com", "autodesk", "raps")?;
        Some(dirs.cache_dir().join("translation_cache.json"))
    }

    /// Load the cache from disk.  Returns an empty cache on any error.
    pub fn load() -> Self {
        let path = match Self::cache_path() {
            Some(p) => p,
            None => return Self::default(),
        };
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persist the cache to disk.  Errors are silently ignored so that a
    /// cache write failure never aborts the user's workflow.
    pub fn save(&self) {
        if let Some(path) = Self::cache_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(s) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(path, s);
            }
        }
    }

    /// Compose the HashMap key for `(urn, output_format)`.
    fn key(urn: &str, output_format: &str) -> String {
        format!("{}::{}", urn, output_format)
    }

    /// Look up a cached entry.
    pub fn get(&self, urn: &str, output_format: &str) -> Option<&CachedTranslation> {
        self.entries.get(&Self::key(urn, output_format))
    }

    /// Insert or overwrite a cache entry.
    pub fn insert(&mut self, urn: &str, output_format: &str, manifest_urn: String, status: String) {
        self.entries.insert(
            Self::key(urn, output_format),
            CachedTranslation {
                manifest_urn,
                status,
                cached_at: chrono::Utc::now().timestamp(),
                output_format: output_format.to_string(),
            },
        );
    }

    /// Remove a cache entry (e.g. when `--force` is used).
    pub fn invalidate(&mut self, urn: &str, output_format: &str) {
        self.entries.remove(&Self::key(urn, output_format));
    }
}

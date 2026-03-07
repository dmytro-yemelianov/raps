// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! HTTP response cache with LRU eviction.
//!
//! Caches GET responses for short-lived reuse (metadata lookups, hub
//! info, project lists). Keyed by URL; only caches 2xx responses.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use lru::LruCache;

// ---------------------------------------------------------------------------
// Cache entry
// ---------------------------------------------------------------------------

/// A cached HTTP response body with expiry.
#[derive(Clone, Debug)]
pub struct CachedResponse {
    pub status: u16,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
    expires_at: Instant,
}

impl CachedResponse {
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

// ---------------------------------------------------------------------------
// Response cache
// ---------------------------------------------------------------------------

/// LRU cache for HTTP GET responses.
///
/// Thread-safe via `Mutex<LruCache>`.  Entries expire after a
/// configurable TTL and are evicted LRU when the capacity is reached.
pub struct ResponseCache {
    inner: Mutex<LruCache<String, CachedResponse>>,
    default_ttl: Duration,
}

impl ResponseCache {
    /// Create a new response cache.
    ///
    /// * `capacity` — max number of entries (LRU eviction beyond this)
    /// * `default_ttl` — how long entries live before expiring
    pub fn new(capacity: usize, default_ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(capacity).expect("capacity must be > 0"),
            )),
            default_ttl,
        }
    }

    /// Look up a cached response by URL key.
    ///
    /// Returns `None` if the entry is missing or expired.
    pub fn get(&self, key: &str) -> Option<CachedResponse> {
        let mut cache = self.inner.lock().unwrap();
        if let Some(entry) = cache.get(key) {
            if entry.is_expired() {
                cache.pop(key);
                return None;
            }
            return Some(entry.clone());
        }
        None
    }

    /// Insert a response into the cache.
    pub fn put(&self, key: String, status: u16, body: Vec<u8>, content_type: Option<String>) {
        self.put_with_ttl(key, status, body, content_type, self.default_ttl);
    }

    /// Insert with a custom TTL.
    pub fn put_with_ttl(
        &self,
        key: String,
        status: u16,
        body: Vec<u8>,
        content_type: Option<String>,
        ttl: Duration,
    ) {
        let entry = CachedResponse {
            status,
            body,
            content_type,
            expires_at: Instant::now() + ttl,
        };
        let mut cache = self.inner.lock().unwrap();
        cache.put(key, entry);
    }

    /// Remove a specific entry.
    pub fn invalidate(&self, key: &str) {
        let mut cache = self.inner.lock().unwrap();
        cache.pop(key);
    }

    /// Clear the entire cache.
    pub fn clear(&self) {
        let mut cache = self.inner.lock().unwrap();
        cache.clear();
    }

    /// Number of entries currently cached.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Purge all expired entries.
    pub fn purge_expired(&self) {
        let mut cache = self.inner.lock().unwrap();
        let expired_keys: Vec<String> = cache
            .iter()
            .filter(|(_, v)| v.is_expired())
            .map(|(k, _)| k.clone())
            .collect();
        for key in expired_keys {
            cache.pop(&key);
        }
    }
}

// ---------------------------------------------------------------------------
// Cacheable URL heuristic
// ---------------------------------------------------------------------------

/// Determine if a request URL is eligible for caching.
///
/// Only GET-like metadata endpoints are cached.  Mutations, uploads,
/// and download streams are excluded.
pub fn is_cacheable_url(url: &str, method: &str) -> bool {
    if method != "GET" {
        return false;
    }
    // Don't cache download streams or signed URLs
    if url.contains("/signeds3download") || url.contains("/signeds3upload") {
        return false;
    }
    // Don't cache auth endpoints
    if url.contains("/authentication/") {
        return false;
    }
    true
}

/// Suggested TTL based on the URL pattern.
pub fn ttl_for_url(url: &str) -> Duration {
    // Hub/project metadata changes rarely — longer TTL
    if url.contains("/hubs") || url.contains("/projects") {
        return Duration::from_secs(300); // 5 min
    }
    // Folder contents change more frequently
    if url.contains("/folders") || url.contains("/contents") {
        return Duration::from_secs(60); // 1 min
    }
    // Item/version metadata
    if url.contains("/items") || url.contains("/versions") {
        return Duration::from_secs(120); // 2 min
    }
    // Model derivative manifest — can be long-lived once complete
    if url.contains("/modelderivative/") && url.contains("/manifest") {
        return Duration::from_secs(600); // 10 min
    }
    // Default
    Duration::from_secs(30)
}

// ---------------------------------------------------------------------------
// Global instance
// ---------------------------------------------------------------------------

static RESPONSE_CACHE: std::sync::OnceLock<ResponseCache> = std::sync::OnceLock::new();

/// Get the global response cache.
///
/// Default: 512 entries, 60s TTL.
pub fn cache() -> &'static ResponseCache {
    RESPONSE_CACHE.get_or_init(|| ResponseCache::new(512, Duration::from_secs(60)))
}

// ---------------------------------------------------------------------------
// Disk-backed TTL cache
// ---------------------------------------------------------------------------

/// Serializable on-disk cache entry.
#[derive(serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    /// Raw JSON response body.
    body: String,
    status: u16,
    /// Unix timestamp of when the entry was cached.
    cached_at: i64,
    ttl_secs: u64,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now - self.cached_at > self.ttl_secs as i64
    }
}

/// Disk-backed HTTP response cache with per-entry TTLs.
///
/// Entries are stored as JSON files under the application cache directory,
/// keyed by a SHA-256 hash of the URL.  Expired entries are removed on
/// first access.
pub struct DiskCache {
    cache_dir: std::path::PathBuf,
}

impl DiskCache {
    /// Create a new disk cache, initialising the cache directory.
    ///
    /// Falls back to a `raps-cache` subdirectory of the system temp dir if
    /// the XDG / platform cache dir cannot be determined.
    pub fn new() -> Self {
        let cache_dir = directories::ProjectDirs::from("com", "autodesk", "raps")
            .map(|dirs| dirs.cache_dir().join("response_cache"))
            .unwrap_or_else(|| std::env::temp_dir().join("raps-cache").join("response_cache"));
        let _ = std::fs::create_dir_all(&cache_dir);
        Self { cache_dir }
    }

    fn key_path(&self, key: &str) -> std::path::PathBuf {
        use sha2::Digest;
        let hash = hex::encode(sha2::Sha256::digest(key.as_bytes()));
        self.cache_dir.join(format!("{}.json", hash))
    }

    /// Return the cached body string for `key`, or `None` if missing/expired.
    pub fn get(&self, key: &str) -> Option<String> {
        let path = self.key_path(key);
        let content = std::fs::read_to_string(&path).ok()?;
        let entry: CacheEntry = serde_json::from_str(&content).ok()?;
        if entry.is_expired() {
            let _ = std::fs::remove_file(&path);
            return None;
        }
        Some(entry.body)
    }

    /// Store a response body in the disk cache with a TTL.
    pub fn set(&self, key: &str, body: String, status: u16, ttl_secs: u64) {
        let entry = CacheEntry {
            body,
            status,
            cached_at: chrono::Utc::now().timestamp(),
            ttl_secs,
        };
        if let Ok(json) = serde_json::to_string(&entry) {
            let _ = std::fs::write(self.key_path(key), json);
        }
    }

    /// Remove all entries from the disk cache directory.
    pub fn clear(&self) {
        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for e in entries.flatten() {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

impl Default for DiskCache {
    fn default() -> Self {
        Self::new()
    }
}

static DISK_CACHE: std::sync::OnceLock<DiskCache> = std::sync::OnceLock::new();

/// Get the global disk-backed response cache.
pub fn disk_cache() -> &'static DiskCache {
    DISK_CACHE.get_or_init(DiskCache::new)
}

/// Return the cache TTL in seconds for a given URL, or `None` if the URL
/// should not be cached.
///
/// Only GET requests should be passed here; POST/PUT/DELETE/PATCH must
/// never be cached.
pub fn cache_ttl(url: &str) -> Option<u64> {
    if url.contains("/oss/v2/buckets") && !url.contains("/objects") {
        return Some(60);
    }
    if url.contains("/project/v1/hubs") {
        return Some(120);
    }
    if url.contains("/da/") && url.contains("/engines") {
        return Some(3600);
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_and_get() {
        let cache = ResponseCache::new(10, Duration::from_secs(60));
        cache.put(
            "https://example.com/api".to_string(),
            200,
            b"hello".to_vec(),
            Some("application/json".to_string()),
        );

        let entry = cache.get("https://example.com/api").unwrap();
        assert_eq!(entry.status, 200);
        assert_eq!(entry.body, b"hello");
        assert_eq!(entry.content_type.as_deref(), Some("application/json"));
    }

    #[test]
    fn test_miss() {
        let cache = ResponseCache::new(10, Duration::from_secs(60));
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_expiry() {
        let cache = ResponseCache::new(10, Duration::from_millis(1));
        cache.put("key".to_string(), 200, vec![], None);
        std::thread::sleep(Duration::from_millis(5));
        assert!(cache.get("key").is_none());
    }

    #[test]
    fn test_lru_eviction() {
        let cache = ResponseCache::new(2, Duration::from_secs(60));
        cache.put("a".to_string(), 200, vec![], None);
        cache.put("b".to_string(), 200, vec![], None);
        cache.put("c".to_string(), 200, vec![], None); // evicts "a"

        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
    }

    #[test]
    fn test_invalidate() {
        let cache = ResponseCache::new(10, Duration::from_secs(60));
        cache.put("key".to_string(), 200, vec![], None);
        cache.invalidate("key");
        assert!(cache.get("key").is_none());
    }

    #[test]
    fn test_clear() {
        let cache = ResponseCache::new(10, Duration::from_secs(60));
        cache.put("a".to_string(), 200, vec![], None);
        cache.put("b".to_string(), 200, vec![], None);
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_purge_expired() {
        let cache = ResponseCache::new(10, Duration::from_secs(60));
        cache.put_with_ttl(
            "short".to_string(),
            200,
            vec![],
            None,
            Duration::from_millis(1),
        );
        cache.put("long".to_string(), 200, vec![], None);
        std::thread::sleep(Duration::from_millis(5));
        cache.purge_expired();
        assert!(cache.get("short").is_none());
        assert!(cache.get("long").is_some());
    }

    #[test]
    fn test_is_cacheable_url() {
        assert!(is_cacheable_url(
            "https://developer.api.autodesk.com/data/v1/projects",
            "GET"
        ));
        assert!(!is_cacheable_url(
            "https://developer.api.autodesk.com/data/v1/projects",
            "POST"
        ));
        assert!(!is_cacheable_url(
            "https://developer.api.autodesk.com/oss/v2/signeds3download",
            "GET"
        ));
        assert!(!is_cacheable_url(
            "https://developer.api.autodesk.com/authentication/v2/token",
            "GET"
        ));
    }

    #[test]
    fn test_ttl_for_url() {
        assert_eq!(
            ttl_for_url("https://developer.api.autodesk.com/project/v1/hubs").as_secs(),
            300
        );
        assert_eq!(
            ttl_for_url("https://developer.api.autodesk.com/data/v1/folders/xxx/contents")
                .as_secs(),
            60
        );
        assert_eq!(
            ttl_for_url(
                "https://developer.api.autodesk.com/modelderivative/v2/designdata/xxx/manifest"
            )
            .as_secs(),
            600
        );
        assert_eq!(
            ttl_for_url("https://developer.api.autodesk.com/oss/v2/buckets").as_secs(),
            30
        );
    }
}

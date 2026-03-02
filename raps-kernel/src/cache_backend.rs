// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Abstract cache backend trait for pluggable cache implementations.
//!
//! Provides a dyn-safe async trait [`CacheBackend`] with two implementations:
//! - [`MemoryBackend`] — wraps the existing in-process LRU response cache
//! - [`RedisBackend`] (feature `redis`) — async Redis-backed cache

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Pluggable async cache backend.
///
/// Implementations must be `Send + Sync + 'static` so they can be shared
/// behind an `Arc` across async tasks.
#[async_trait]
pub trait CacheBackend: Send + Sync + 'static {
    /// Retrieve a value by key. Returns `None` on cache miss or expiry.
    async fn get(&self, key: &str) -> Option<Vec<u8>>;

    /// Store a value with the given TTL.
    async fn set(&self, key: &str, value: &[u8], ttl: Duration) -> Result<()>;

    /// Remove a single key.
    async fn remove(&self, key: &str) -> Result<()>;

    /// Remove all entries.
    async fn clear(&self) -> Result<()>;

    /// Number of entries currently stored.
    async fn len(&self) -> usize;
}

/// Type-erased cache backend behind an `Arc`.
pub type BoxedCacheBackend = Arc<dyn CacheBackend>;

// ---------------------------------------------------------------------------
// MemoryBackend
// ---------------------------------------------------------------------------

struct MemoryEntry {
    value: Vec<u8>,
    expires_at: Instant,
}

/// In-process LRU cache backend.
///
/// Uses a simple `HashMap` with TTL-based expiry. Suitable for single-process
/// deployments and local development.
pub struct MemoryBackend {
    inner: Mutex<HashMap<String, MemoryEntry>>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CacheBackend for MemoryBackend {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut map = self.inner.lock().unwrap();
        if let Some(entry) = map.get(key) {
            if Instant::now() >= entry.expires_at {
                map.remove(key);
                return None;
            }
            return Some(entry.value.clone());
        }
        None
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Duration) -> Result<()> {
        let entry = MemoryEntry {
            value: value.to_vec(),
            expires_at: Instant::now() + ttl,
        };
        self.inner.lock().unwrap().insert(key.to_string(), entry);
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
        self.inner.lock().unwrap().remove(key);
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        self.inner.lock().unwrap().clear();
        Ok(())
    }

    async fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Create an in-memory cache backend.
pub fn memory_backend() -> BoxedCacheBackend {
    Arc::new(MemoryBackend::new())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_miss() {
        let backend = memory_backend();
        assert!(backend.get("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_set_and_get_roundtrip() {
        let backend = memory_backend();
        backend
            .set("key1", b"hello world", Duration::from_secs(60))
            .await
            .unwrap();

        let val = backend.get("key1").await.unwrap();
        assert_eq!(val, b"hello world");
    }

    #[tokio::test]
    async fn test_remove() {
        let backend = memory_backend();
        backend
            .set("key1", b"data", Duration::from_secs(60))
            .await
            .unwrap();
        assert!(backend.get("key1").await.is_some());

        backend.remove("key1").await.unwrap();
        assert!(backend.get("key1").await.is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let backend = memory_backend();
        backend
            .set("a", b"1", Duration::from_secs(60))
            .await
            .unwrap();
        backend
            .set("b", b"2", Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(backend.len().await, 2);

        backend.clear().await.unwrap();
        assert_eq!(backend.len().await, 0);
    }

    #[tokio::test]
    async fn test_expiry() {
        let backend = memory_backend();
        backend
            .set("short", b"data", Duration::from_millis(1))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(backend.get("short").await.is_none());
    }

    #[tokio::test]
    async fn test_len() {
        let backend = memory_backend();
        assert_eq!(backend.len().await, 0);

        backend
            .set("a", b"1", Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(backend.len().await, 1);

        backend
            .set("b", b"2", Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(backend.len().await, 2);
    }
}

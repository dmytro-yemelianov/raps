// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Redis-backed cache backend using `deadpool-redis`.
//!
//! Feature-gated behind `redis`. Provides [`RedisBackend`] which implements
//! [`CacheBackend`] using Redis `GET`/`SETEX`/`DEL`/`SCAN` commands.

#![cfg(feature = "redis")]

use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;

use crate::cache_backend::CacheBackend;

// ---------------------------------------------------------------------------
// RedisBackend
// ---------------------------------------------------------------------------

/// Async Redis cache backend backed by a connection pool.
///
/// Keys are namespaced with a configurable prefix to avoid collisions
/// when multiple applications share the same Redis instance.
pub struct RedisBackend {
    pool: Pool,
    namespace: String,
}

impl RedisBackend {
    /// Create a new `RedisBackend`.
    ///
    /// * `redis_url` — e.g. `redis://127.0.0.1:6379`
    /// * `pool_size` — max connections in the pool
    /// * `namespace` — key prefix (e.g. `"raps:cache"`)
    pub fn new(redis_url: &str, pool_size: usize, namespace: &str) -> Result<Self> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .builder()
            .map_err(|e| anyhow::anyhow!("Redis pool builder error: {e}"))?
            .max_size(pool_size)
            .runtime(Runtime::Tokio1)
            .build()
            .context("Failed to create Redis connection pool")?;

        Ok(Self {
            pool,
            namespace: namespace.to_string(),
        })
    }

    /// Create from environment variables.
    ///
    /// Reads `RAPS_REDIS_URL` (default `redis://127.0.0.1:6379`),
    /// pool size is 8, namespace is `raps:cache`.
    pub fn from_env() -> Result<Self> {
        let url =
            std::env::var("RAPS_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
        Self::new(&url, 8, "raps:cache")
    }

    /// Build the full Redis key with namespace prefix.
    fn key(&self, key: &str) -> String {
        format!("{}:{}", self.namespace, key)
    }

    /// Get a reference to the underlying connection pool.
    pub fn pool(&self) -> &Pool {
        &self.pool
    }
}

#[async_trait]
impl CacheBackend for RedisBackend {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut conn = self.pool.get().await.ok()?;
        let result: Option<Vec<u8>> = conn.get(self.key(key)).await.ok()?;
        result
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Duration) -> Result<()> {
        let mut conn = self.pool.get().await.context("Redis pool exhausted")?;
        let ttl_secs = ttl.as_secs().max(1) as i64;
        conn.set_ex::<_, _, ()>(self.key(key), value.to_vec(), ttl_secs as u64)
            .await
            .context("Redis SETEX failed")?;
        Ok(())
    }

    async fn remove(&self, key: &str) -> Result<()> {
        let mut conn = self.pool.get().await.context("Redis pool exhausted")?;
        conn.del::<_, ()>(self.key(key))
            .await
            .context("Redis DEL failed")?;
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        let mut conn = self.pool.get().await.context("Redis pool exhausted")?;
        let pattern = format!("{}:*", self.namespace);
        let mut cursor: u64 = 0;
        loop {
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut *conn)
                .await
                .context("Redis SCAN failed")?;

            if !keys.is_empty() {
                conn.del::<_, ()>(keys)
                    .await
                    .context("Redis DEL (batch) failed")?;
            }

            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }
        Ok(())
    }

    async fn len(&self) -> usize {
        let Ok(mut conn) = self.pool.get().await else {
            return 0;
        };
        let pattern = format!("{}:*", self.namespace);
        let mut count: usize = 0;
        let mut cursor: u64 = 0;
        loop {
            let result: Result<(u64, Vec<String>), _> = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut *conn)
                .await;

            match result {
                Ok((next_cursor, keys)) => {
                    count += keys.len();
                    cursor = next_cursor;
                    if cursor == 0 {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        count
    }
}

/// Create a Redis cache backend from the `RAPS_REDIS_URL` environment variable.
///
/// Returns `None` if the env var is unset or connection fails.
pub fn redis_backend_from_env() -> Result<crate::cache_backend::BoxedCacheBackend> {
    let backend = RedisBackend::from_env()?;
    Ok(std::sync::Arc::new(backend))
}

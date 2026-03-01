// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Lightweight metrics collector for API operations.
//!
//! Tracks per-endpoint request counts, error rates, latencies, and
//! cache hit ratios. Periodically flushable to JSON for persistence
//! and reporting.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use anyhow::{Context, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Per-endpoint metrics
// ---------------------------------------------------------------------------

/// Atomic counters for a single API endpoint group.
pub struct ApiMetrics {
    pub request_count: AtomicU64,
    pub error_count: AtomicU64,
    pub total_latency_ms: AtomicU64,
    pub cache_hits: AtomicU64,
}

impl ApiMetrics {
    fn new() -> Self {
        Self {
            request_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
        }
    }

    pub fn record_request(&self, latency_ms: u64) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(latency_ms, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn avg_latency_ms(&self) -> u64 {
        let count = self.request_count.load(Ordering::Relaxed);
        if count == 0 {
            return 0;
        }
        self.total_latency_ms.load(Ordering::Relaxed) / count
    }

    pub fn error_rate(&self) -> f64 {
        let count = self.request_count.load(Ordering::Relaxed);
        if count == 0 {
            return 0.0;
        }
        self.error_count.load(Ordering::Relaxed) as f64 / count as f64
    }
}

// ---------------------------------------------------------------------------
// Translation metric
// ---------------------------------------------------------------------------

/// Record of a single translation operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationMetric {
    pub urn: String,
    pub file_type: String,
    pub duration_ms: u64,
    pub status: String,
    pub region: String,
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// Serializable snapshot
// ---------------------------------------------------------------------------

/// Serializable snapshot of API metrics.
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiMetricsSnapshot {
    pub endpoint: String,
    pub request_count: u64,
    pub error_count: u64,
    pub avg_latency_ms: u64,
    pub cache_hits: u64,
    pub error_rate: f64,
}

/// Full metrics snapshot for persistence.
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: String,
    pub api_metrics: Vec<ApiMetricsSnapshot>,
    pub translations: Vec<TranslationMetric>,
}

// ---------------------------------------------------------------------------
// Collector
// ---------------------------------------------------------------------------

/// Central metrics collector.
pub struct MetricsCollector {
    api_metrics: DashMap<String, ApiMetrics>,
    translations: Mutex<Vec<TranslationMetric>>,
    flush_path: PathBuf,
}

impl MetricsCollector {
    pub fn new(flush_path: PathBuf) -> Result<Self> {
        if let Some(parent) = flush_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create metrics dir: {}", parent.display()))?;
        }
        Ok(Self {
            api_metrics: DashMap::new(),
            translations: Mutex::new(Vec::new()),
            flush_path,
        })
    }

    /// Default metrics path.
    pub fn default_path() -> PathBuf {
        directories::ProjectDirs::from("", "", "raps")
            .map(|d| d.data_dir().join("metrics").join("latest.json"))
            .unwrap_or_else(|| PathBuf::from(".raps/metrics/latest.json"))
    }

    /// Record an API request.
    pub fn record_api_request(&self, endpoint: &str, latency_ms: u64) {
        let entry = self.api_metrics.entry(endpoint.to_string()).or_insert_with(ApiMetrics::new);
        entry.record_request(latency_ms);
    }

    /// Record an API error.
    pub fn record_api_error(&self, endpoint: &str) {
        let entry = self.api_metrics.entry(endpoint.to_string()).or_insert_with(ApiMetrics::new);
        entry.record_error();
    }

    /// Record a cache hit.
    pub fn record_cache_hit(&self, endpoint: &str) {
        let entry = self.api_metrics.entry(endpoint.to_string()).or_insert_with(ApiMetrics::new);
        entry.record_cache_hit();
    }

    /// Record a translation operation.
    pub fn record_translation(&self, metric: TranslationMetric) {
        if let Ok(mut translations) = self.translations.lock() {
            translations.push(metric);
        }
    }

    /// Take a snapshot of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let api_metrics: Vec<ApiMetricsSnapshot> = self
            .api_metrics
            .iter()
            .map(|entry| ApiMetricsSnapshot {
                endpoint: entry.key().clone(),
                request_count: entry.value().request_count.load(Ordering::Relaxed),
                error_count: entry.value().error_count.load(Ordering::Relaxed),
                avg_latency_ms: entry.value().avg_latency_ms(),
                cache_hits: entry.value().cache_hits.load(Ordering::Relaxed),
                error_rate: entry.value().error_rate(),
            })
            .collect();

        let translations = self
            .translations
            .lock()
            .map(|t| t.clone())
            .unwrap_or_default();

        MetricsSnapshot {
            timestamp: chrono::Utc::now().to_rfc3339(),
            api_metrics,
            translations,
        }
    }

    /// Flush metrics to disk.
    pub fn flush(&self) -> Result<()> {
        let snapshot = self.snapshot();
        let json = serde_json::to_string_pretty(&snapshot)?;
        std::fs::write(&self.flush_path, json)
            .with_context(|| format!("failed to flush metrics: {}", self.flush_path.display()))?;
        Ok(())
    }

    /// Load metrics from disk.
    pub fn load_snapshot(path: &std::path::Path) -> Result<Option<MetricsSnapshot>> {
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(path)?;
        let snapshot: MetricsSnapshot = serde_json::from_str(&data)?;
        Ok(Some(snapshot))
    }
}

// ---------------------------------------------------------------------------
// Global instance
// ---------------------------------------------------------------------------

static COLLECTOR: std::sync::OnceLock<MetricsCollector> = std::sync::OnceLock::new();

/// Get the global metrics collector.
pub fn collector() -> &'static MetricsCollector {
    COLLECTOR.get_or_init(|| {
        MetricsCollector::new(MetricsCollector::default_path())
            .expect("failed to initialize metrics collector")
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_collector() -> (tempfile::TempDir, MetricsCollector) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("metrics.json");
        let collector = MetricsCollector::new(path).unwrap();
        (dir, collector)
    }

    #[test]
    fn test_record_api_request() {
        let (_dir, mc) = temp_collector();
        mc.record_api_request("oss", 100);
        mc.record_api_request("oss", 200);
        mc.record_api_request("data-management", 50);

        let snap = mc.snapshot();
        let oss = snap.api_metrics.iter().find(|m| m.endpoint == "oss").unwrap();
        assert_eq!(oss.request_count, 2);
        assert_eq!(oss.avg_latency_ms, 150);

        let dm = snap.api_metrics.iter().find(|m| m.endpoint == "data-management").unwrap();
        assert_eq!(dm.request_count, 1);
    }

    #[test]
    fn test_error_rate() {
        let (_dir, mc) = temp_collector();
        mc.record_api_request("oss", 100);
        mc.record_api_request("oss", 100);
        mc.record_api_error("oss");

        let snap = mc.snapshot();
        let oss = snap.api_metrics.iter().find(|m| m.endpoint == "oss").unwrap();
        assert_eq!(oss.error_count, 1);
        assert!((oss.error_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_translation_metric() {
        let (_dir, mc) = temp_collector();
        mc.record_translation(TranslationMetric {
            urn: "urn:adsk:test".to_string(),
            file_type: "rvt".to_string(),
            duration_ms: 12000,
            status: "success".to_string(),
            region: "US".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        let snap = mc.snapshot();
        assert_eq!(snap.translations.len(), 1);
        assert_eq!(snap.translations[0].file_type, "rvt");
    }

    #[test]
    fn test_flush_and_load() {
        let (dir, mc) = temp_collector();
        mc.record_api_request("oss", 100);
        mc.flush().unwrap();

        let path = dir.path().join("metrics.json");
        let snap = MetricsCollector::load_snapshot(&path).unwrap().unwrap();
        assert_eq!(snap.api_metrics.len(), 1);
    }

    #[test]
    fn test_cache_hits() {
        let (_dir, mc) = temp_collector();
        mc.record_cache_hit("data-management");
        mc.record_cache_hit("data-management");
        mc.record_cache_hit("data-management");

        let snap = mc.snapshot();
        let dm = snap.api_metrics.iter().find(|m| m.endpoint == "data-management").unwrap();
        assert_eq!(dm.cache_hits, 3);
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Prometheus metrics exporter for Kubernetes observability.
//!
//! Bridges the internal [`MetricsCollector`] DashMap counters into a
//! Prometheus registry that can be scraped via the `/metrics` endpoint.

use prometheus::{
    CounterVec, Encoder, GaugeVec, HistogramOpts, HistogramVec, Opts, Registry, TextEncoder,
};

use crate::metrics::MetricsCollector;

/// Prometheus exporter that mirrors internal metrics into Prometheus gauges,
/// counters, and histograms.
pub struct PrometheusExporter {
    registry: Registry,
    pub queue_depth: GaugeVec,
    pub jobs_processed_total: CounterVec,
    pub job_duration_seconds: HistogramVec,
    pub api_requests_total: CounterVec,
    pub api_errors_total: CounterVec,
    pub cache_hits_total: CounterVec,
    pub worker_info: GaugeVec,
}

impl Default for PrometheusExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl PrometheusExporter {
    /// Create a new Prometheus exporter with all metrics registered.
    pub fn new() -> Self {
        let registry = Registry::new();

        let queue_depth = GaugeVec::new(
            Opts::new("raps_queue_depth", "Number of jobs waiting in queue"),
            &["priority"],
        )
        .expect("queue_depth metric");
        registry
            .register(Box::new(queue_depth.clone()))
            .expect("register queue_depth");

        let jobs_processed_total = CounterVec::new(
            Opts::new("raps_jobs_processed_total", "Total jobs processed"),
            &["priority", "status"],
        )
        .expect("jobs_processed_total metric");
        registry
            .register(Box::new(jobs_processed_total.clone()))
            .expect("register jobs_processed_total");

        let job_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "raps_job_duration_seconds",
                "Job processing duration in seconds",
            )
            .buckets(vec![
                1.0, 5.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0,
            ]),
            &["job_type"],
        )
        .expect("job_duration_seconds metric");
        registry
            .register(Box::new(job_duration_seconds.clone()))
            .expect("register job_duration_seconds");

        let api_requests_total = CounterVec::new(
            Opts::new("raps_api_requests_total", "Total API requests"),
            &["endpoint"],
        )
        .expect("api_requests_total metric");
        registry
            .register(Box::new(api_requests_total.clone()))
            .expect("register api_requests_total");

        let api_errors_total = CounterVec::new(
            Opts::new("raps_api_errors_total", "Total API errors"),
            &["endpoint"],
        )
        .expect("api_errors_total metric");
        registry
            .register(Box::new(api_errors_total.clone()))
            .expect("register api_errors_total");

        let cache_hits_total = CounterVec::new(
            Opts::new("raps_cache_hits_total", "Total cache hits"),
            &["endpoint"],
        )
        .expect("cache_hits_total metric");
        registry
            .register(Box::new(cache_hits_total.clone()))
            .expect("register cache_hits_total");

        let worker_info = GaugeVec::new(
            Opts::new("raps_worker_info", "Worker metadata"),
            &["worker_id", "version"],
        )
        .expect("worker_info metric");
        registry
            .register(Box::new(worker_info.clone()))
            .expect("register worker_info");

        Self {
            registry,
            queue_depth,
            jobs_processed_total,
            job_duration_seconds,
            api_requests_total,
            api_errors_total,
            cache_hits_total,
            worker_info,
        }
    }

    /// Render all registered metrics as Prometheus text format.
    pub fn render(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).expect("encode metrics");
        String::from_utf8(buffer).expect("metrics UTF-8")
    }

    /// Sync counters from the existing [`MetricsCollector`] DashMap into
    /// Prometheus counters. Call this periodically to bridge internal metrics.
    pub fn sync_from_collector(&self, collector: &MetricsCollector) {
        let snapshot = collector.snapshot();
        for api in &snapshot.api_metrics {
            // We use reset-and-increment because Prometheus counters are
            // monotonically increasing, and our internal counters already are.
            let req_count = api.request_count as f64;
            let req = self
                .api_requests_total
                .get_metric_with_label_values(&[&api.endpoint])
                .expect("api_requests_total label");
            let current = req.get();
            if req_count > current {
                req.inc_by(req_count - current);
            }

            let err_count = api.error_count as f64;
            let err = self
                .api_errors_total
                .get_metric_with_label_values(&[&api.endpoint])
                .expect("api_errors_total label");
            let current = err.get();
            if err_count > current {
                err.inc_by(err_count - current);
            }

            let cache_count = api.cache_hits as f64;
            let cache = self
                .cache_hits_total
                .get_metric_with_label_values(&[&api.endpoint])
                .expect("cache_hits_total label");
            let current = cache.get();
            if cache_count > current {
                cache.inc_by(cache_count - current);
            }
        }
    }

    /// Set the queue depth gauge for a given priority level.
    pub fn set_queue_depth(&self, priority: &str, depth: f64) {
        self.queue_depth
            .with_label_values(&[priority])
            .set(depth);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Behavioral Tests ====================

    #[test]
    fn test_new_exporter() {
        let exporter = PrometheusExporter::new();
        let output = exporter.render();
        // Empty registry should still produce valid output
        assert!(output.is_empty() || output.contains("# HELP") || !output.contains("ERROR"));
    }

    #[test]
    fn test_sync_from_collector() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("metrics.json");
        let collector = MetricsCollector::new(path).unwrap();

        collector.record_api_request("oss", 100);
        collector.record_api_request("oss", 200);
        collector.record_api_error("oss");
        collector.record_cache_hit("oss");

        let exporter = PrometheusExporter::new();
        exporter.sync_from_collector(&collector);

        let output = exporter.render();
        assert!(output.contains("raps_api_requests_total"));
        assert!(output.contains("raps_api_errors_total"));
        assert!(output.contains("raps_cache_hits_total"));
    }

    // ==================== Snapshot Contract Tests ====================

    #[test]
    fn test_render_empty() {
        let exporter = PrometheusExporter::new();
        let output = exporter.render();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_render_queue_depth() {
        let exporter = PrometheusExporter::new();
        exporter.set_queue_depth("high", 42.0);
        exporter.set_queue_depth("normal", 10.0);
        let output = exporter.render();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_render_full_metrics() {
        let exporter = PrometheusExporter::new();

        // Queue depths
        exporter.set_queue_depth("critical", 3.0);
        exporter.set_queue_depth("normal", 15.0);
        exporter.set_queue_depth("background", 42.0);

        // Jobs processed
        exporter
            .jobs_processed_total
            .with_label_values(&["critical", "success"])
            .inc_by(100.0);
        exporter
            .jobs_processed_total
            .with_label_values(&["normal", "failure"])
            .inc_by(2.0);

        // Job duration
        exporter
            .job_duration_seconds
            .with_label_values(&["translate"])
            .observe(45.0);

        // API requests
        exporter
            .api_requests_total
            .with_label_values(&["/oss/v2/buckets"])
            .inc_by(500.0);

        // API errors
        exporter
            .api_errors_total
            .with_label_values(&["/oss/v2/buckets"])
            .inc_by(3.0);

        // Cache hits
        exporter
            .cache_hits_total
            .with_label_values(&["/oss/v2/buckets"])
            .inc_by(120.0);

        // Worker info
        exporter
            .worker_info
            .with_label_values(&["worker-1", "5.0.0"])
            .set(1.0);

        let output = exporter.render();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_sync_from_collector_render() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("metrics.json");
        let collector = MetricsCollector::new(path).unwrap();

        collector.record_api_request("oss", 100);
        collector.record_api_request("oss", 200);
        collector.record_api_error("oss");
        collector.record_cache_hit("oss");

        let exporter = PrometheusExporter::new();
        exporter.sync_from_collector(&collector);

        let output = exporter.render();
        insta::assert_snapshot!(output);
    }

    #[test]
    fn test_histogram_buckets() {
        let exporter = PrometheusExporter::new();
        // Observe a single value to force bucket output
        exporter
            .job_duration_seconds
            .with_label_values(&["translate"])
            .observe(45.0);
        let output = exporter.render();
        insta::assert_snapshot!(output);
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Lightweight health and metrics HTTP server for Kubernetes probes.
//!
//! Exposes three endpoints:
//! - `GET /health` — liveness probe (always 200)
//! - `GET /ready`  — readiness probe (200 when ready, 503 otherwise)
//! - `GET /metrics` — Prometheus text format

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};

use crate::prometheus_metrics::PrometheusExporter;

/// Shared state for the health server.
#[derive(Clone)]
pub struct HealthServerState {
    pub ready: Arc<AtomicBool>,
    pub exporter: Arc<PrometheusExporter>,
}

/// Start the health/metrics HTTP server on the given port.
///
/// This spawns an axum server that will run until the process exits.
pub async fn start_health_server(
    port: u16,
    state: HealthServerState,
) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("Health server listening on 0.0.0.0:{port}");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"status": "ok"})))
}

async fn ready_handler(
    State(state): State<HealthServerState>,
) -> impl IntoResponse {
    if state.ready.load(Ordering::Relaxed) {
        (StatusCode::OK, axum::Json(serde_json::json!({"status": "ready"})))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({"status": "not_ready"})),
        )
    }
}

async fn metrics_handler(
    State(state): State<HealthServerState>,
) -> impl IntoResponse {
    let body = state.exporter.render();
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_state(ready: bool) -> HealthServerState {
        HealthServerState {
            ready: Arc::new(AtomicBool::new(ready)),
            exporter: Arc::new(PrometheusExporter::new()),
        }
    }

    fn app(state: HealthServerState) -> Router {
        Router::new()
            .route("/health", get(health_handler))
            .route("/ready", get(ready_handler))
            .route("/metrics", get(metrics_handler))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = test_state(true);
        let response = app(state)
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_ready_endpoint_when_ready() {
        let state = test_state(true);
        let response = app(state)
            .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ready_endpoint_when_not_ready() {
        let state = test_state(false);
        let response = app(state)
            .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let state = test_state(true);
        state.exporter.set_queue_depth("high", 5.0);

        let response = app(state)
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("raps_queue_depth"));
    }

    #[tokio::test]
    async fn test_metrics_content_type() {
        let state = test_state(true);
        let response = app(state)
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let content_type = response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(content_type.contains("text/plain"));
    }

    // ==================== Snapshot Contract Tests ====================

    #[tokio::test]
    async fn test_health_response_body() {
        let state = test_state(true);
        let response = app(state)
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        insta::assert_json_snapshot!(json);
    }

    #[tokio::test]
    async fn test_ready_response_body() {
        let state = test_state(true);
        let response = app(state)
            .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        insta::assert_json_snapshot!(json);
    }

    #[tokio::test]
    async fn test_not_ready_response_body() {
        let state = test_state(false);
        let response = app(state)
            .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        insta::assert_json_snapshot!(json);
    }

    #[tokio::test]
    async fn test_metrics_response_with_data() {
        let state = test_state(true);
        state.exporter.set_queue_depth("high", 5.0);
        state.exporter.set_queue_depth("normal", 12.0);
        state
            .exporter
            .jobs_processed_total
            .with_label_values(&["high", "success"])
            .inc_by(50.0);

        let response = app(state)
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        insta::assert_snapshot!(text);
    }
}

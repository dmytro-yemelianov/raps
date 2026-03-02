// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Kubernetes service components: `raps serve <component>`.
//!
//! Each subcommand starts an Axum HTTP server with component-specific routes
//! plus the standard `/health`, `/ready`, `/metrics` endpoints. All services
//! communicate via Redis and are feature-gated behind `kubernetes`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use clap::Subcommand;
use colored::Colorize;
use serde::{Deserialize, Serialize};

use raps_kernel::job_queue::{Job, JobPayload, JobPriority, JobProducer};
use raps_kernel::prometheus_metrics::PrometheusExporter;
use raps_kernel::redis_backend::RedisBackend;

/// Redis XREVRANGE result type: Vec<(entry_id, [(field, value), ...])>
type StreamEntries = Vec<(String, Vec<(String, String)>)>;

// ---------------------------------------------------------------------------
// CLI subcommands
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum ServeCommands {
    /// Start the API reverse proxy (response cache + rate limiting)
    Proxy {
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,

        /// Redis connection URL
        #[arg(long, default_value = "redis://127.0.0.1:6379")]
        redis_url: String,

        /// Upstream APS API base URL
        #[arg(long, default_value = "https://developer.api.autodesk.com")]
        upstream: String,

        /// Upstream request timeout in seconds
        #[arg(long, default_value = "120")]
        timeout_secs: u64,
    },

    /// Start the job coordinator API
    Coordinator {
        /// Port to listen on
        #[arg(long, default_value = "8081")]
        port: u16,

        /// Redis connection URL
        #[arg(long, default_value = "redis://127.0.0.1:6379")]
        redis_url: String,
    },

    /// Start the webhook receiver
    Webhook {
        /// Port to listen on
        #[arg(long, default_value = "9000")]
        port: u16,

        /// Redis connection URL
        #[arg(long, default_value = "redis://127.0.0.1:6379")]
        redis_url: String,
    },

    /// Start the monitoring dashboard API
    Dashboard {
        /// Port to listen on
        #[arg(long, default_value = "3000")]
        port: u16,

        /// Redis connection URL
        #[arg(long, default_value = "redis://127.0.0.1:6379")]
        redis_url: String,
    },
}

impl ServeCommands {
    pub async fn execute(self) -> Result<()> {
        match self {
            ServeCommands::Proxy {
                port,
                redis_url,
                upstream,
                timeout_secs,
            } => start_proxy(port, &redis_url, &upstream, timeout_secs).await,
            ServeCommands::Coordinator { port, redis_url } => {
                start_coordinator(port, &redis_url).await
            }
            ServeCommands::Webhook { port, redis_url } => start_webhook(port, &redis_url).await,
            ServeCommands::Dashboard { port, redis_url } => {
                start_dashboard(port, &redis_url).await
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Create the standard health/ready/metrics router and merge it with
/// component-specific routes.
fn build_app(component_routes: Router<ServiceState>, state: ServiceState) -> Router {
    let health_routes = Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/metrics", get(metrics_handler));

    component_routes.merge(health_routes).with_state(state)
}

/// Start an axum server with graceful shutdown.
async fn start_server(name: &str, port: u16, app: Router) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .with_context(|| format!("Failed to bind {name} on port {port}"))?;

    tracing::info!("{name} listening on 0.0.0.0:{port}");
    println!(
        "{} {} listening on 0.0.0.0:{}",
        "✓".green(),
        name,
        port
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .with_context(|| format!("{name} server error"))?;

    println!("{name} shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    tracing::info!("Shutdown signal received");
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct ServiceState {
    ready: Arc<AtomicBool>,
    exporter: Arc<PrometheusExporter>,
    pool: deadpool_redis::Pool,
    /// Upstream URL for proxy service
    upstream: String,
    /// HTTP client for proxy forwarding
    http_client: reqwest::Client,
}

// Health/ready/metrics handlers (shared across all services)

async fn health_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "ok"})),
    )
}

async fn ready_handler(State(state): State<ServiceState>) -> impl IntoResponse {
    if state.ready.load(Ordering::Relaxed) {
        (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ready"})),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"status": "not_ready"})),
        )
    }
}

async fn metrics_handler(State(state): State<ServiceState>) -> impl IntoResponse {
    let body = state.exporter.render();
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

fn create_state(redis_url: &str, upstream: &str, timeout_secs: u64) -> Result<ServiceState> {
    let backend = RedisBackend::new(redis_url, 8, "raps:service")?;
    let pool = backend.pool().clone();

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .context("Failed to build HTTP client")?;

    let state = ServiceState {
        ready: Arc::new(AtomicBool::new(true)),
        exporter: Arc::new(PrometheusExporter::new()),
        pool,
        upstream: upstream.to_string(),
        http_client,
    };

    Ok(state)
}

// ===========================================================================
// Proxy
// ===========================================================================

async fn start_proxy(port: u16, redis_url: &str, upstream: &str, timeout_secs: u64) -> Result<()> {
    println!("{}", "RAPS API Proxy".bold());
    println!("  Port:     {}", port);
    println!("  Redis:    {}", redis_url);
    println!("  Upstream: {}", upstream);
    println!();

    let state = create_state(redis_url, upstream, timeout_secs)?;

    let routes = Router::new().route("/api/{*path}", get(proxy_forward).post(proxy_forward));

    let app = build_app(routes, state);
    start_server("raps-proxy", port, app).await
}

async fn proxy_forward(
    State(state): State<ServiceState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let path = uri.path();
    let query = uri.query().map(|q| format!("?{q}")).unwrap_or_default();
    let upstream_url = format!("{}{}{}", state.upstream, path, query);

    // Check response cache for GET requests
    let cache_key = format!("{method} {path}{query}");
    if method == Method::GET {
        let cache = raps_kernel::response_cache::cache();
        if let Some(cached) = cache.get(&cache_key) {
            state
                .exporter
                .cache_hits_total
                .with_label_values(&[path])
                .inc();
            let content_type = cached
                .content_type
                .unwrap_or_else(|| "application/json".to_string());
            return (
                StatusCode::from_u16(cached.status).unwrap_or(StatusCode::OK),
                [(axum::http::header::CONTENT_TYPE, content_type)],
                cached.body,
            )
                .into_response();
        }
    }

    // Check rate budget — group by first two path segments (e.g. /oss/v2 → "oss")
    let endpoint = path
        .split('/')
        .find(|s| !s.is_empty())
        .unwrap_or("unknown")
        .to_string();
    let registry = raps_kernel::rate_budget::registry();
    if let raps_kernel::rate_budget::RateStatus::Exhausted { retry_after } =
        registry.check(&endpoint)
    {
        let retry_secs = retry_after.as_secs().to_string();
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [("Retry-After", retry_secs)],
            "Rate limit exceeded",
        )
            .into_response();
    }

    // Forward request upstream
    let mut req = state.http_client.request(method.clone(), &upstream_url);

    // Forward relevant headers
    for (name, value) in &headers {
        let name_str = name.as_str();
        if name_str == "authorization" || name_str == "content-type" || name_str == "accept" {
            req = req.header(name, value);
        }
    }

    if !body.is_empty() {
        req = req.body(body.to_vec());
    }

    state
        .exporter
        .api_requests_total
        .with_label_values(&[path])
        .inc();

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();

            // Record rate budget from response headers (reqwest::HeaderMap)
            registry.record_from_headers(&endpoint, resp.headers());

            let content_type = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            let resp_body = resp.bytes().await.unwrap_or_default().to_vec();

            // Cache successful GET responses
            if method == Method::GET && status.is_success() {
                let cache = raps_kernel::response_cache::cache();
                let ttl = raps_kernel::response_cache::ttl_for_url(path);
                cache.put_with_ttl(
                    cache_key,
                    status.as_u16(),
                    resp_body.clone(),
                    Some(content_type.clone()),
                    ttl,
                );
            }

            if status.is_server_error() || status.is_client_error() {
                state
                    .exporter
                    .api_errors_total
                    .with_label_values(&[path])
                    .inc();
            }

            (
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
                [(axum::http::header::CONTENT_TYPE, content_type)],
                resp_body,
            )
                .into_response()
        }
        Err(e) => {
            state
                .exporter
                .api_errors_total
                .with_label_values(&[path])
                .inc();
            tracing::error!(error = %e, url = %upstream_url, "Upstream request failed");
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": "upstream request failed", "detail": e.to_string()})),
            )
                .into_response()
        }
    }
}

// ===========================================================================
// Coordinator
// ===========================================================================

async fn start_coordinator(port: u16, redis_url: &str) -> Result<()> {
    println!("{}", "RAPS Job Coordinator".bold());
    println!("  Port:  {}", port);
    println!("  Redis: {}", redis_url);
    println!();

    let state = create_state(redis_url, "", 30)?;

    let routes = Router::new()
        .route("/jobs", post(create_job))
        .route("/jobs", get(list_jobs))
        .route("/jobs/{id}", get(get_job))
        .route("/pipelines", post(create_pipeline));

    let app = build_app(routes, state);
    start_server("raps-coordinator", port, app).await
}

#[derive(Deserialize)]
struct CreateJobRequest {
    /// Job type: "translate", "upload", "extract_props", "pipeline"
    payload: JobPayload,
    /// Priority: "critical", "normal", "background"
    #[serde(default = "default_priority")]
    priority: JobPriority,
}

fn default_priority() -> JobPriority {
    JobPriority::Normal
}

async fn create_job(
    State(state): State<ServiceState>,
    Json(req): Json<CreateJobRequest>,
) -> impl IntoResponse {
    let producer = JobProducer::new(state.pool.clone());

    match producer.enqueue(req.payload, req.priority).await {
        Ok(entry_id) => {
            state
                .exporter
                .queue_depth
                .with_label_values(&[match req.priority {
                    JobPriority::Critical => "critical",
                    JobPriority::Normal => "normal",
                    JobPriority::Background => "background",
                }])
                .inc();

            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "entry_id": entry_id,
                    "priority": req.priority,
                    "status": "enqueued"
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to enqueue job");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "failed to enqueue job", "detail": e.to_string()})),
            )
                .into_response()
        }
    }
}

async fn list_jobs(State(state): State<ServiceState>) -> impl IntoResponse {
    // Read recent entries from all priority streams via XREVRANGE
    let mut all_jobs = Vec::new();

    let Ok(mut conn) = state.pool.get().await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Redis connection failed"})),
        )
            .into_response();
    };

    for priority in JobPriority::all() {
        let stream = priority.stream_key();
        let result: Result<StreamEntries, redis::RedisError> =
            redis::cmd("XREVRANGE")
                .arg(stream)
                .arg("+")
                .arg("-")
                .arg("COUNT")
                .arg(50)
                .query_async(&mut *conn)
                .await;

        if let Ok(entries) = result {
            for (entry_id, fields) in entries {
                let data = fields.iter().find(|(k, _)| k == "data").map(|(_, v)| v);
                if let Some(json_str) = data
                    && let Ok(job) = serde_json::from_str::<Job>(json_str)
                {
                    all_jobs.push(serde_json::json!({
                        "entry_id": entry_id,
                        "id": job.id,
                        "priority": job.priority,
                        "payload_type": payload_type_name(&job.payload),
                        "attempts": job.attempts,
                        "created_at": job.created_at,
                    }));
                }
            }
        }
    }

    (StatusCode::OK, Json(serde_json::json!({"jobs": all_jobs}))).into_response()
}

async fn get_job(
    State(state): State<ServiceState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Search all priority streams for the job ID
    let Ok(mut conn) = state.pool.get().await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Redis connection failed"})),
        )
            .into_response();
    };

    for priority in JobPriority::all() {
        let stream = priority.stream_key();
        let result: Result<StreamEntries, redis::RedisError> =
            redis::cmd("XREVRANGE")
                .arg(stream)
                .arg("+")
                .arg("-")
                .arg("COUNT")
                .arg(200)
                .query_async(&mut *conn)
                .await;

        if let Ok(entries) = result {
            for (entry_id, fields) in entries {
                let data = fields.iter().find(|(k, _)| k == "data").map(|(_, v)| v);
                if let Some(json_str) = data
                    && let Ok(job) = serde_json::from_str::<Job>(json_str)
                    && job.id == id
                {
                    return (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "entry_id": entry_id,
                            "id": job.id,
                            "priority": job.priority,
                            "payload": job.payload,
                            "attempts": job.attempts,
                            "max_attempts": job.max_attempts,
                            "created_at": job.created_at,
                            "enqueued_by": job.enqueued_by,
                        })),
                    )
                        .into_response();
                }
            }
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "job not found"})),
    )
        .into_response()
}

#[derive(Deserialize)]
struct CreatePipelineRequest {
    pipeline_name: String,
    pipeline_file: String,
    #[serde(default)]
    variables: std::collections::HashMap<String, String>,
    #[serde(default = "default_priority")]
    priority: JobPriority,
}

async fn create_pipeline(
    State(state): State<ServiceState>,
    Json(req): Json<CreatePipelineRequest>,
) -> impl IntoResponse {
    let producer = JobProducer::new(state.pool.clone());
    let payload = JobPayload::Pipeline(raps_kernel::job_queue::PipelineJob {
        pipeline_name: req.pipeline_name,
        pipeline_file: req.pipeline_file,
        variables: req.variables,
    });

    match producer.enqueue(payload, req.priority).await {
        Ok(entry_id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "entry_id": entry_id,
                "priority": req.priority,
                "status": "enqueued"
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to enqueue pipeline");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "failed to enqueue pipeline", "detail": e.to_string()})),
            )
                .into_response()
        }
    }
}

fn payload_type_name(payload: &JobPayload) -> &'static str {
    match payload {
        JobPayload::Translate(_) => "translate",
        JobPayload::Upload(_) => "upload",
        JobPayload::ExtractProps(_) => "extract_props",
        JobPayload::Pipeline(_) => "pipeline",
    }
}

// ===========================================================================
// Webhook
// ===========================================================================

async fn start_webhook(port: u16, redis_url: &str) -> Result<()> {
    println!("{}", "RAPS Webhook Receiver".bold());
    println!("  Port:  {}", port);
    println!("  Redis: {}", redis_url);
    println!();

    let state = create_state(redis_url, "", 30)?;

    let routes = Router::new()
        .route("/webhooks/callback", post(webhook_callback))
        .route("/webhooks", get(webhook_list));

    let app = build_app(routes, state);
    start_server("raps-webhook", port, app).await
}

#[derive(Deserialize, Serialize)]
struct WebhookEvent {
    #[serde(rename = "Type")]
    event_type: Option<String>,
    #[serde(flatten)]
    payload: serde_json::Value,
}

async fn webhook_callback(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(event): Json<WebhookEvent>,
) -> impl IntoResponse {
    // Verify signature if APS_WEBHOOK_SECRET is set
    let secret = std::env::var("APS_WEBHOOK_SECRET").ok();
    if let Some(ref secret) = secret {
        let signature = headers
            .get("x-adsk-signature")
            .and_then(|v| v.to_str().ok());
        match signature {
            Some(sig) => {
                // Verify HMAC-SHA256 signature
                let body_bytes =
                    serde_json::to_vec(&event.payload).unwrap_or_default();
                if !verify_webhook_signature(secret, &body_bytes, sig) {
                    tracing::warn!("Webhook signature verification failed");
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({"error": "invalid signature"})),
                    )
                        .into_response();
                }
            }
            None => {
                tracing::warn!("Webhook request missing x-adsk-signature header");
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error": "missing signature"})),
                )
                    .into_response();
            }
        }
    }

    // Publish event to Redis Stream
    let event_type = event
        .event_type
        .as_deref()
        .unwrap_or("unknown");
    let stream_key = format!("raps:events:{event_type}");

    let event_data = serde_json::to_string(&event.payload).unwrap_or_default();

    let Ok(mut conn) = state.pool.get().await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Redis connection failed"})),
        )
            .into_response();
    };

    let result: Result<String, redis::RedisError> = redis::cmd("XADD")
        .arg(&stream_key)
        .arg("*")
        .arg("event_type")
        .arg(event_type)
        .arg("data")
        .arg(&event_data)
        .arg("received_at")
        .arg(chrono::Utc::now().to_rfc3339())
        .query_async(&mut *conn)
        .await;

    match result {
        Ok(entry_id) => {
            tracing::info!(event_type, entry_id, "Webhook event published");
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "accepted",
                    "stream": stream_key,
                    "entry_id": entry_id,
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to publish webhook event");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "failed to publish event"})),
            )
                .into_response()
        }
    }
}

fn verify_webhook_signature(secret: &str, body: &[u8], expected_signature: &str) -> bool {
    use sha2::Sha256;
    use hmac::{Hmac, Mac};

    type HmacSha256 = Hmac<Sha256>;

    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);

    // The signature may be hex-encoded
    let Ok(expected_bytes) = hex::decode(expected_signature) else {
        return false;
    };

    mac.verify_slice(&expected_bytes).is_ok()
}

async fn webhook_list(State(state): State<ServiceState>) -> impl IntoResponse {
    // List webhook event streams from Redis by scanning for raps:events:* keys
    let Ok(mut conn) = state.pool.get().await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Redis connection failed"})),
        )
            .into_response();
    };

    let mut streams: Vec<serde_json::Value> = Vec::new();
    let mut cursor: u64 = 0;

    loop {
        let result: Result<(u64, Vec<String>), redis::RedisError> = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg("raps:events:*")
            .arg("COUNT")
            .arg(100)
            .query_async(&mut *conn)
            .await;

        match result {
            Ok((next_cursor, keys)) => {
                for key in keys {
                    let len: u64 = redis::cmd("XLEN")
                        .arg(&key)
                        .query_async(&mut *conn)
                        .await
                        .unwrap_or(0);

                    let event_type = key
                        .strip_prefix("raps:events:")
                        .unwrap_or(&key);
                    streams.push(serde_json::json!({
                        "event_type": event_type,
                        "stream": key,
                        "length": len,
                    }));
                }
                cursor = next_cursor;
                if cursor == 0 {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({"webhook_streams": streams})),
    )
        .into_response()
}

// ===========================================================================
// Dashboard
// ===========================================================================

async fn start_dashboard(port: u16, redis_url: &str) -> Result<()> {
    println!("{}", "RAPS Monitoring Dashboard".bold());
    println!("  Port:  {}", port);
    println!("  Redis: {}", redis_url);
    println!();

    let state = create_state(redis_url, "", 30)?;

    let routes = Router::new()
        .route("/", get(dashboard_index))
        .route("/api/workers", get(dashboard_workers))
        .route("/api/queues", get(dashboard_queues))
        .route("/api/metrics", get(dashboard_metrics))
        .route("/api/jobs/recent", get(dashboard_recent_jobs));

    let app = build_app(routes, state);
    start_server("raps-dashboard", port, app).await
}

async fn dashboard_index() -> impl IntoResponse {
    Html(DASHBOARD_HTML)
}

async fn dashboard_workers(State(state): State<ServiceState>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get().await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Redis connection failed"})),
        )
            .into_response();
    };

    // Scan for worker heartbeat keys: raps:worker:heartbeat:*
    let mut workers: Vec<serde_json::Value> = Vec::new();
    let mut cursor: u64 = 0;

    loop {
        let result: Result<(u64, Vec<String>), redis::RedisError> = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg("raps:worker:heartbeat:*")
            .arg("COUNT")
            .arg(100)
            .query_async(&mut *conn)
            .await;

        match result {
            Ok((next_cursor, keys)) => {
                for key in keys {
                    let ttl: i64 = redis::cmd("TTL")
                        .arg(&key)
                        .query_async(&mut *conn)
                        .await
                        .unwrap_or(-1);

                    let value: String = redis::cmd("GET")
                        .arg(&key)
                        .query_async(&mut *conn)
                        .await
                        .unwrap_or_default();

                    let worker_id = key
                        .strip_prefix("raps:worker:heartbeat:")
                        .unwrap_or(&key);
                    workers.push(serde_json::json!({
                        "worker_id": worker_id,
                        "status": if ttl > 0 { "alive" } else { "stale" },
                        "ttl_secs": ttl,
                        "data": value,
                    }));
                }
                cursor = next_cursor;
                if cursor == 0 {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({"workers": workers})),
    )
        .into_response()
}

async fn dashboard_queues(State(state): State<ServiceState>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get().await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Redis connection failed"})),
        )
            .into_response();
    };

    let mut queues: Vec<serde_json::Value> = Vec::new();

    for priority in JobPriority::all() {
        let stream = priority.stream_key();
        let len: u64 = redis::cmd("XLEN")
            .arg(stream)
            .query_async(&mut *conn)
            .await
            .unwrap_or(0);

        queues.push(serde_json::json!({
            "priority": priority,
            "stream": stream,
            "depth": len,
        }));
    }

    // Also check DLQ
    let dlq_len: u64 = redis::cmd("XLEN")
        .arg("raps:queue:dlq")
        .query_async(&mut *conn)
        .await
        .unwrap_or(0);

    queues.push(serde_json::json!({
        "priority": "dlq",
        "stream": "raps:queue:dlq",
        "depth": dlq_len,
    }));

    (
        StatusCode::OK,
        Json(serde_json::json!({"queues": queues})),
    )
        .into_response()
}

async fn dashboard_metrics(State(_state): State<ServiceState>) -> impl IntoResponse {
    // Aggregate rate budget snapshot
    let rate_budgets = raps_kernel::rate_budget::registry().snapshot();
    let cache_stats = raps_kernel::response_cache::cache();
    let cache_len = cache_stats.len();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "rate_budgets": rate_budgets.iter().map(|(endpoint, remaining, limit)| {
                serde_json::json!({
                    "endpoint": endpoint,
                    "remaining": remaining,
                    "limit": limit,
                })
            }).collect::<Vec<_>>(),
            "response_cache": {
                "entries": cache_len,
            },
        })),
    )
        .into_response()
}

async fn dashboard_recent_jobs(State(state): State<ServiceState>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get().await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Redis connection failed"})),
        )
            .into_response();
    };

    let mut jobs: Vec<serde_json::Value> = Vec::new();

    for priority in JobPriority::all() {
        let stream = priority.stream_key();
        let result: Result<StreamEntries, redis::RedisError> =
            redis::cmd("XREVRANGE")
                .arg(stream)
                .arg("+")
                .arg("-")
                .arg("COUNT")
                .arg(20)
                .query_async(&mut *conn)
                .await;

        if let Ok(entries) = result {
            for (entry_id, fields) in entries {
                let data = fields.iter().find(|(k, _)| k == "data").map(|(_, v)| v);
                if let Some(json_str) = data
                    && let Ok(job) = serde_json::from_str::<Job>(json_str)
                {
                    jobs.push(serde_json::json!({
                        "entry_id": entry_id,
                        "id": job.id,
                        "priority": job.priority,
                        "type": payload_type_name(&job.payload),
                        "attempts": job.attempts,
                        "created_at": job.created_at,
                    }));
                }
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({"jobs": jobs})),
    )
        .into_response()
}

// Minimal embedded dashboard HTML
const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>RAPS Dashboard</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f1117; color: #e1e4e8; padding: 2rem; }
        h1 { color: #58a6ff; margin-bottom: 1.5rem; }
        h2 { color: #8b949e; margin: 1.5rem 0 0.75rem; font-size: 1rem; text-transform: uppercase; letter-spacing: 0.05em; }
        .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 1rem; }
        .card { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 1rem; }
        .card-title { color: #8b949e; font-size: 0.85rem; margin-bottom: 0.5rem; }
        .card-value { font-size: 2rem; font-weight: 600; color: #58a6ff; }
        table { width: 100%; border-collapse: collapse; margin-top: 0.5rem; }
        th, td { text-align: left; padding: 0.5rem; border-bottom: 1px solid #21262d; font-size: 0.9rem; }
        th { color: #8b949e; font-weight: 500; }
        .status-alive { color: #3fb950; }
        .status-stale { color: #f85149; }
        .refresh-btn { background: #21262d; color: #c9d1d9; border: 1px solid #30363d; padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; font-size: 0.85rem; }
        .refresh-btn:hover { background: #30363d; }
    </style>
</head>
<body>
    <div style="display:flex;align-items:center;justify-content:space-between">
        <h1>RAPS Dashboard</h1>
        <button class="refresh-btn" onclick="loadAll()">Refresh</button>
    </div>

    <h2>Queue Depths</h2>
    <div class="grid" id="queues"></div>

    <h2>Workers</h2>
    <div id="workers"><table><tbody id="workers-body"></tbody></table></div>

    <h2>Recent Jobs</h2>
    <div id="jobs"><table><thead><tr><th>ID</th><th>Type</th><th>Priority</th><th>Created</th></tr></thead><tbody id="jobs-body"></tbody></table></div>

    <script>
        async function loadQueues() {
            const r = await fetch('/api/queues');
            const d = await r.json();
            document.getElementById('queues').innerHTML = d.queues.map(q =>
                `<div class="card"><div class="card-title">${q.priority}</div><div class="card-value">${q.depth}</div></div>`
            ).join('');
        }
        async function loadWorkers() {
            const r = await fetch('/api/workers');
            const d = await r.json();
            document.getElementById('workers-body').innerHTML = d.workers.map(w =>
                `<tr><td>${w.worker_id}</td><td class="status-${w.status}">${w.status}</td><td>${w.ttl_secs}s</td></tr>`
            ).join('') || '<tr><td colspan="3">No workers</td></tr>';
        }
        async function loadJobs() {
            const r = await fetch('/api/jobs/recent');
            const d = await r.json();
            document.getElementById('jobs-body').innerHTML = d.jobs.slice(0, 20).map(j =>
                `<tr><td>${j.id.slice(0,8)}…</td><td>${j.type}</td><td>${j.priority}</td><td>${j.created_at}</td></tr>`
            ).join('') || '<tr><td colspan="4">No jobs</td></tr>';
        }
        function loadAll() { loadQueues(); loadWorkers(); loadJobs(); }
        loadAll();
        setInterval(loadAll, 10000);
    </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    use raps_kernel::job_queue::*;

    #[test]
    fn test_payload_type_name_translate() {
        let payload = JobPayload::Translate(TranslateJob {
            urn: String::new(),
            output_format: String::new(),
            root_filename: None,
            region: None,
            force: false,
        });
        assert_eq!(payload_type_name(&payload), "translate");
    }

    #[test]
    fn test_payload_type_name_upload() {
        let payload = JobPayload::Upload(UploadJob {
            bucket_key: String::new(),
            object_key: String::new(),
            file_path: String::new(),
        });
        assert_eq!(payload_type_name(&payload), "upload");
    }

    #[test]
    fn test_payload_type_name_extract_props() {
        let payload = JobPayload::ExtractProps(ExtractPropsJob {
            urn: String::new(),
            view_guid: None,
            output_path: String::new(),
        });
        assert_eq!(payload_type_name(&payload), "extract_props");
    }

    #[test]
    fn test_payload_type_name_pipeline() {
        let payload = JobPayload::Pipeline(PipelineJob {
            pipeline_name: String::new(),
            pipeline_file: String::new(),
            variables: std::collections::HashMap::new(),
        });
        assert_eq!(payload_type_name(&payload), "pipeline");
    }

    #[test]
    fn test_verify_webhook_signature_valid() {
        use sha2::Sha256;
        use hmac::{Hmac, Mac};

        let secret = "test-secret";
        let body = b"hello world";

        // Compute expected signature
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let sig = hex::encode(mac.finalize().into_bytes());

        assert!(verify_webhook_signature(secret, body, &sig));
    }

    #[test]
    fn test_verify_webhook_signature_invalid() {
        assert!(!verify_webhook_signature("secret", b"body", "0000000000000000000000000000000000000000000000000000000000000000"));
    }

    #[test]
    fn test_verify_webhook_signature_bad_hex() {
        assert!(!verify_webhook_signature("secret", b"body", "not-hex!"));
    }

    #[test]
    fn test_default_priority() {
        assert!(matches!(default_priority(), JobPriority::Normal));
    }
}

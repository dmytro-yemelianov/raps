// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Swarm orchestration commands.
//!
//! Introspect resilience layer state: circuit breakers, rate budgets,
//! response cache, metrics, checkpoints. When built with the `redis`
//! feature, also provides distributed worker management.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum SwarmCommands {
    /// Show circuit breaker states, rate budgets, cache stats
    Status,

    /// Show API latency, error rates, translation stats
    Metrics,

    /// Show all batch operations: pending, active, and completed
    Queue {
        /// Filter by workflow type (e.g. "upload", "translate", "permissions")
        #[arg(long, short)]
        r#type: Option<String>,

        /// Show only incomplete operations
        #[arg(long)]
        pending: bool,
    },

    /// List incomplete batch operations that can be resumed
    Resume,

    /// Show audit log entries for today (or a specific date)
    Audit {
        /// Date to show (YYYY-MM-DD). Defaults to today.
        #[arg(long)]
        date: Option<String>,

        /// Number of most recent entries to show.
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Reset swarm state: circuit breakers, caches, or all
    Reset {
        /// What to reset: "circuit-breakers", "cache", "rate-budgets", or "all"
        #[arg(default_value = "all")]
        target: String,
    },

    /// Start a distributed worker that consumes jobs from Redis Streams
    #[cfg(feature = "redis")]
    Worker {
        #[command(subcommand)]
        cmd: WorkerCommands,
    },
}

/// Worker subcommands (requires `redis` feature).
#[cfg(feature = "redis")]
#[derive(Debug, Subcommand)]
pub enum WorkerCommands {
    /// Start consuming jobs from Redis Streams
    Start {
        /// Redis connection URL
        #[arg(long, env = "RAPS_REDIS_URL", default_value = "redis://127.0.0.1:6379")]
        redis_url: String,

        /// Maximum concurrent jobs
        #[arg(long, default_value = "4")]
        concurrency: usize,

        /// Heartbeat interval in seconds
        #[arg(long, default_value = "30")]
        heartbeat_secs: u64,

        /// Comma-separated queue priorities to consume (critical,normal,background)
        #[arg(long, default_value = "critical,normal,background")]
        queues: String,

        /// Port for health/metrics HTTP server (requires kubernetes feature)
        #[cfg_attr(feature = "kubernetes", arg(long, default_value = "9091"))]
        #[cfg_attr(not(feature = "kubernetes"), arg(long))]
        metrics_port: Option<u16>,
    },
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Serialize, schemars::JsonSchema)]
struct CircuitBreakerInfo {
    endpoint: String,
    state: String,
    failures: u32,
}

#[derive(Serialize, schemars::JsonSchema)]
struct RateBudgetInfo {
    endpoint: String,
    remaining: u32,
    limit: u32,
}

#[derive(Serialize, schemars::JsonSchema)]
struct SwarmStatusOutput {
    circuit_breakers: Vec<CircuitBreakerInfo>,
    rate_budgets: Vec<RateBudgetInfo>,
    response_cache_entries: usize,
}

#[derive(Serialize, schemars::JsonSchema)]
struct CheckpointInfo {
    workflow_id: String,
    workflow_type: String,
    total: usize,
    completed: usize,
    failed: usize,
    remaining: usize,
    progress_pct: f64,
    updated_at: String,
}

impl SwarmCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            SwarmCommands::Status => swarm_status(output_format),
            SwarmCommands::Metrics => swarm_metrics(output_format),
            SwarmCommands::Queue { r#type, pending } => swarm_queue(r#type, pending, output_format),
            SwarmCommands::Resume => swarm_resume(output_format),
            SwarmCommands::Audit { date, limit } => swarm_audit(date, limit, output_format),
            SwarmCommands::Reset { target } => swarm_reset(&target, output_format),
            #[cfg(feature = "redis")]
            SwarmCommands::Worker { cmd } => cmd.execute(output_format).await,
        }
    }
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

fn swarm_status(output_format: OutputFormat) -> Result<()> {
    let cb_snap = raps_kernel::circuit_breaker::registry().snapshot();
    let rb_snap = raps_kernel::rate_budget::registry().snapshot();
    let cache_len = raps_kernel::response_cache::cache().len();

    let output = SwarmStatusOutput {
        circuit_breakers: cb_snap
            .iter()
            .map(|(name, state, failures)| CircuitBreakerInfo {
                endpoint: name.clone(),
                state: state.to_string(),
                failures: *failures,
            })
            .collect(),
        rate_budgets: rb_snap
            .iter()
            .map(|(name, remaining, limit)| RateBudgetInfo {
                endpoint: name.clone(),
                remaining: *remaining,
                limit: *limit,
            })
            .collect(),
        response_cache_entries: cache_len,
    };

    match output_format {
        OutputFormat::Table => {
            println!("{}", "Circuit Breakers".bold());
            if output.circuit_breakers.is_empty() {
                println!("  No circuit breakers active");
            } else {
                for cb in &output.circuit_breakers {
                    let state_colored = match cb.state.as_str() {
                        "Closed" => cb.state.green().to_string(),
                        "Open" => cb.state.red().to_string(),
                        "HalfOpen" => cb.state.yellow().to_string(),
                        _ => cb.state.clone(),
                    };
                    println!(
                        "  {} {} (failures: {})",
                        cb.endpoint, state_colored, cb.failures
                    );
                }
            }

            println!("\n{}", "Rate Budgets".bold());
            if output.rate_budgets.is_empty() {
                println!("  No rate budget data");
            } else {
                for rb in &output.rate_budgets {
                    let pct = if rb.limit > 0 {
                        rb.remaining as f64 / rb.limit as f64 * 100.0
                    } else {
                        100.0
                    };
                    let status = if pct > 20.0 {
                        format!("{:.0}%", pct).green().to_string()
                    } else if pct > 0.0 {
                        format!("{:.0}%", pct).yellow().to_string()
                    } else {
                        "EXHAUSTED".red().to_string()
                    };
                    println!(
                        "  {} {}/{} ({})",
                        rb.endpoint, rb.remaining, rb.limit, status
                    );
                }
            }

            println!("\n{}", "Response Cache".bold());
            println!("  Entries: {}", cache_len);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

fn swarm_metrics(output_format: OutputFormat) -> Result<()> {
    let metrics_path = raps_kernel::metrics::MetricsCollector::default_path();
    let snapshot = raps_kernel::metrics::MetricsCollector::load_snapshot(&metrics_path)?;

    if let Some(snap) = snapshot {
        match output_format {
            OutputFormat::Table => {
                println!("{}", "API Metrics".bold());
                if snap.api_metrics.is_empty() {
                    println!("  No API metrics recorded yet");
                } else {
                    println!(
                        "  {:<25} {:>8} {:>8} {:>10} {:>8} {:>8}",
                        "Endpoint", "Requests", "Errors", "Avg ms", "Err %", "Cache"
                    );
                    for m in &snap.api_metrics {
                        println!(
                            "  {:<25} {:>8} {:>8} {:>10} {:>7.1}% {:>8}",
                            m.endpoint,
                            m.request_count,
                            m.error_count,
                            m.avg_latency_ms,
                            m.error_rate * 100.0,
                            m.cache_hits,
                        );
                    }
                }

                if !snap.translations.is_empty() {
                    println!("\n{}", "Recent Translations".bold());
                    for t in snap.translations.iter().rev().take(10) {
                        println!(
                            "  {} {} {} {}ms ({})",
                            t.timestamp.chars().take(19).collect::<String>(),
                            t.file_type,
                            t.status,
                            t.duration_ms,
                            t.region,
                        );
                    }
                }
            }
            _ => {
                output_format.write(&snap)?;
            }
        }
    } else {
        println!("No metrics data found. Metrics are recorded during API operations.");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Resume
// ---------------------------------------------------------------------------

fn swarm_resume(output_format: OutputFormat) -> Result<()> {
    let store_dir = raps_kernel::checkpoint::CheckpointStore::default_dir();
    let store = raps_kernel::checkpoint::CheckpointStore::new(store_dir)?;
    let checkpoints = store.list()?;

    let incomplete: Vec<CheckpointInfo> = checkpoints
        .iter()
        .filter(|cp| !cp.is_complete())
        .map(|cp| CheckpointInfo {
            workflow_id: cp.workflow_id.clone(),
            workflow_type: cp.workflow_type.clone(),
            total: cp.total_units,
            completed: cp.completed.len(),
            failed: cp.failed.len(),
            remaining: cp.remaining().len(),
            progress_pct: cp.progress() * 100.0,
            updated_at: cp.updated_at.clone(),
        })
        .collect();

    match output_format {
        OutputFormat::Table => {
            if incomplete.is_empty() {
                println!("No incomplete batch operations to resume.");
            } else {
                println!("{}", "Resumable Operations".bold());
                for info in &incomplete {
                    println!(
                        "  {} [{}] {:.0}% ({}/{} done, {} failed, {} remaining)",
                        info.workflow_id,
                        info.workflow_type,
                        info.progress_pct,
                        info.completed,
                        info.total,
                        info.failed,
                        info.remaining,
                    );
                    println!("    Last updated: {}", info.updated_at);
                }
            }
        }
        _ => {
            output_format.write(&incomplete)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Audit
// ---------------------------------------------------------------------------

fn swarm_audit(date: Option<String>, limit: usize, output_format: OutputFormat) -> Result<()> {
    let audit_dir = raps_kernel::audit::AuditLogger::default_dir();
    let logger = raps_kernel::audit::AuditLogger::new(audit_dir, 90)?;

    let date = date.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    let entries = logger.read_date(&date)?;
    let tail: Vec<_> = entries.iter().rev().take(limit).cloned().collect();

    match output_format {
        OutputFormat::Table => {
            if tail.is_empty() {
                println!("No audit entries for {date}.");
            } else {
                println!("{} ({})", "Audit Log".bold(), date);
                for entry in &tail {
                    let result_colored = if entry.result == "success" {
                        entry.result.green().to_string()
                    } else {
                        entry.result.red().to_string()
                    };
                    let ts = entry.timestamp.chars().take(19).collect::<String>();
                    println!(
                        "  {} {} {} {} {}ms",
                        ts, entry.operation, entry.resource, result_colored, entry.duration_ms,
                    );
                }
            }
        }
        _ => {
            output_format.write(&tail)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Queue
// ---------------------------------------------------------------------------

#[derive(Serialize, schemars::JsonSchema)]
struct QueueItem {
    workflow_id: String,
    workflow_type: String,
    status: String,
    total: usize,
    completed: usize,
    failed: usize,
    remaining: usize,
    progress_pct: f64,
    created_at: String,
    updated_at: String,
}

fn swarm_queue(
    type_filter: Option<String>,
    pending_only: bool,
    output_format: OutputFormat,
) -> Result<()> {
    let store_dir = raps_kernel::checkpoint::CheckpointStore::default_dir();
    let store = raps_kernel::checkpoint::CheckpointStore::new(store_dir)?;
    let checkpoints = store.list()?;

    let items: Vec<QueueItem> = checkpoints
        .iter()
        .filter(|cp| {
            if let Some(ref t) = type_filter
                && !cp.workflow_type.to_lowercase().contains(&t.to_lowercase())
            {
                return false;
            }
            if pending_only && cp.is_complete() {
                return false;
            }
            true
        })
        .map(|cp| {
            let status = if cp.is_complete() {
                if cp.failed.is_empty() {
                    "completed".to_string()
                } else {
                    "completed (failures)".to_string()
                }
            } else if cp.completed.is_empty() && cp.failed.is_empty() {
                "pending".to_string()
            } else {
                "active".to_string()
            };
            QueueItem {
                workflow_id: cp.workflow_id.clone(),
                workflow_type: cp.workflow_type.clone(),
                status,
                total: cp.total_units,
                completed: cp.completed.len(),
                failed: cp.failed.len(),
                remaining: cp.remaining().len(),
                progress_pct: cp.progress() * 100.0,
                created_at: cp.created_at.clone(),
                updated_at: cp.updated_at.clone(),
            }
        })
        .collect();

    match output_format {
        OutputFormat::Table => {
            if items.is_empty() {
                println!("No batch operations in the queue.");
            } else {
                println!("{}", "Swarm Queue".bold());
                println!(
                    "  {:<20} {:<12} {:<10} {:>6} {:>6} {:>6} {:>8}",
                    "Workflow", "Type", "Status", "Done", "Fail", "Left", "Progress"
                );
                println!("  {}", "─".repeat(74));
                for item in &items {
                    let status_colored = match item.status.as_str() {
                        "completed" => item.status.green().to_string(),
                        "active" => item.status.yellow().to_string(),
                        "pending" => item.status.cyan().to_string(),
                        _ => item.status.clone(),
                    };
                    let id_short = if item.workflow_id.len() > 18 {
                        format!("{}…", &item.workflow_id[..17])
                    } else {
                        item.workflow_id.clone()
                    };
                    println!(
                        "  {:<20} {:<12} {:<10} {:>6} {:>6} {:>6} {:>7.0}%",
                        id_short,
                        item.workflow_type,
                        status_colored,
                        item.completed,
                        item.failed,
                        item.remaining,
                        item.progress_pct,
                    );
                }
                println!(
                    "\n  Total: {} operations ({} active, {} completed)",
                    items.len(),
                    items
                        .iter()
                        .filter(|i| i.status == "active" || i.status == "pending")
                        .count(),
                    items
                        .iter()
                        .filter(|i| i.status.starts_with("completed"))
                        .count(),
                );
            }
        }
        _ => {
            output_format.write(&items)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Reset
// ---------------------------------------------------------------------------

fn swarm_reset(target: &str, _output_format: OutputFormat) -> Result<()> {
    match target {
        "circuit-breakers" | "cb" => {
            raps_kernel::circuit_breaker::registry().reset_all();
            println!("{} Circuit breakers reset", "✓".green());
        }
        "cache" => {
            raps_kernel::response_cache::cache().clear();
            println!("{} Response cache cleared", "✓".green());
        }
        "rate-budgets" | "rb" => {
            raps_kernel::rate_budget::registry().reset_all();
            println!("{} Rate budgets reset", "✓".green());
        }
        "all" => {
            raps_kernel::circuit_breaker::registry().reset_all();
            raps_kernel::response_cache::cache().clear();
            raps_kernel::rate_budget::registry().reset_all();
            println!(
                "{} All swarm state reset (circuit breakers, cache, rate budgets)",
                "✓".green()
            );
        }
        other => {
            anyhow::bail!(
                "Unknown reset target: '{other}'. Use: circuit-breakers, cache, rate-budgets, or all"
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Worker (feature = "redis")
// ---------------------------------------------------------------------------

#[cfg(feature = "redis")]
impl WorkerCommands {
    pub async fn execute(self, _output_format: OutputFormat) -> Result<()> {
        match self {
            WorkerCommands::Start {
                redis_url,
                concurrency,
                heartbeat_secs,
                queues: _,
                metrics_port,
            } => worker_start(&redis_url, concurrency, heartbeat_secs, metrics_port).await,
        }
    }
}

#[cfg(feature = "redis")]
async fn worker_start(
    redis_url: &str,
    concurrency: usize,
    heartbeat_secs: u64,
    metrics_port: Option<u16>,
) -> Result<()> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::Semaphore;

    use raps_kernel::job_queue::JobConsumer;
    use raps_kernel::redis_backend::RedisBackend;

    let consumer_id = format!(
        "{}-{}",
        hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".into()),
        std::process::id(),
    );

    println!("{}", "RAPS Distributed Worker".bold());
    println!("  Redis:       {}", redis_url);
    println!("  Consumer:    {}", consumer_id);
    println!("  Concurrency: {}", concurrency);
    println!("  Heartbeat:   {}s", heartbeat_secs);
    println!();

    // Build Redis pool via RedisBackend (reuse the pool)
    let backend = RedisBackend::new(redis_url, concurrency + 2, "raps:worker")?;
    let pool = backend.pool().clone();

    // Consumer setup
    let consumer = Arc::new(JobConsumer::new(pool.clone(), consumer_id.clone()));
    consumer.ensure_consumer_groups().await?;
    println!("{} Consumer groups ready", "✓".green());

    // Start health/metrics server if kubernetes feature is enabled
    #[cfg(feature = "kubernetes")]
    {
        let port = metrics_port.unwrap_or(9091);
        let exporter = Arc::new(raps_kernel::prometheus_metrics::PrometheusExporter::new());
        let ready = Arc::new(AtomicBool::new(false));

        let state = raps_kernel::health_server::HealthServerState {
            ready: ready.clone(),
            exporter: exporter.clone(),
        };

        // Spawn health server in background
        tokio::spawn(async move {
            if let Err(e) = raps_kernel::health_server::start_health_server(port, state).await {
                eprintln!("Health server error: {e}");
            }
        });

        println!(
            "{} Health server listening on 0.0.0.0:{}",
            "✓".green(),
            port
        );

        // Mark as ready
        ready.store(true, Ordering::Relaxed);

        // Spawn queue depth polling task (every 15s)
        let exporter_poll = exporter.clone();
        let redis_url_poll = redis_url.to_string();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
            loop {
                interval.tick().await;
                // Sync internal metrics to Prometheus
                let collector = raps_kernel::metrics::collector();
                exporter_poll.sync_from_collector(collector);

                // Poll queue depths via XLEN
                if let Ok(client) = redis::Client::open(redis_url_poll.as_str()) {
                    if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                        for priority in &["critical", "normal", "background"] {
                            let stream_key = format!("raps:queue:{priority}");
                            let len: Result<f64, _> = redis::cmd("XLEN")
                                .arg(&stream_key)
                                .query_async(&mut conn)
                                .await;
                            if let Ok(depth) = len {
                                exporter_poll.set_queue_depth(priority, depth);
                            }
                        }
                    }
                }
            }
        });

        // Set worker info gauge
        exporter
            .worker_info
            .with_label_values(&[&consumer_id, env!("CARGO_PKG_VERSION")])
            .set(1.0);
    }

    #[cfg(not(feature = "kubernetes"))]
    if metrics_port.is_some() {
        println!(
            "{} --metrics-port requires the 'kubernetes' feature. Ignoring.",
            "!".yellow(),
        );
    }

    // Concurrency limiter
    let semaphore = Arc::new(Semaphore::new(concurrency));

    // Graceful shutdown flag
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_signal = shutdown.clone();

    // SIGTERM / Ctrl+C handler
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        println!(
            "\n{} Shutdown signal received, draining in-flight jobs...",
            "!".yellow()
        );
        shutdown_signal.store(true, Ordering::SeqCst);
    });

    // Heartbeat task
    let hb_pool = pool.clone();
    let hb_id = consumer_id.clone();
    let hb_shutdown = shutdown.clone();
    tokio::spawn(async move {
        let ttl = heartbeat_secs * 3;
        loop {
            if hb_shutdown.load(Ordering::SeqCst) {
                break;
            }
            if let Ok(mut conn) = hb_pool.get().await {
                let key = format!("raps:worker:heartbeat:{}", hb_id);
                let ts = chrono::Utc::now().to_rfc3339();
                let _: Result<String, redis::RedisError> = redis::cmd("SETEX")
                    .arg(&key)
                    .arg(ttl)
                    .arg(&ts)
                    .query_async(&mut *conn)
                    .await;
            }
            tokio::time::sleep(std::time::Duration::from_secs(heartbeat_secs)).await;
        }
    });

    println!("{} Worker running — press Ctrl+C to stop", "✓".green());

    // Main consume loop
    loop {
        if shutdown.load(Ordering::SeqCst) {
            // Wait for all in-flight jobs to complete
            let _ = semaphore.acquire_many(concurrency as u32).await;
            println!("{} All in-flight jobs drained. Shutting down.", "✓".green());
            break;
        }

        // Acquire permit before blocking on dequeue
        let permit = semaphore.clone().acquire_owned().await?;

        let consumer = consumer.clone();
        let shutdown = shutdown.clone();

        tokio::spawn(async move {
            let _permit = permit; // held until job completes

            match consumer.dequeue_one(2000).await {
                Ok(Some((priority, entry_id, job))) => {
                    tracing::info!(job_id = %job.id, ?priority, "Processing job");
                    let result = process_job(&job).await;
                    match result {
                        Ok(()) => {
                            if let Err(e) = consumer.ack(priority, &entry_id).await {
                                tracing::error!(error = %e, "Failed to ACK job");
                            }
                        }
                        Err(e) => {
                            tracing::error!(job_id = %job.id, error = %e, "Job failed");
                            if job.attempts >= job.max_attempts {
                                let _ = consumer.nack_to_dlq(&job, &e.to_string()).await;
                            }
                        }
                    }
                }
                Ok(None) => {
                    // No job available within timeout — normal, just loop
                }
                Err(e) => {
                    if !shutdown.load(Ordering::SeqCst) {
                        tracing::error!(error = %e, "Dequeue error");
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }
        });
    }

    Ok(())
}

/// Dispatch a job to the appropriate handler.
#[cfg(feature = "redis")]
async fn process_job(job: &raps_kernel::job_queue::Job) -> Result<()> {
    use raps_kernel::job_queue::JobPayload;

    match &job.payload {
        JobPayload::Translate(t) => {
            tracing::info!(urn = %t.urn, format = %t.output_format, "Processing translate job");
            println!(
                "  {} Translate job {} — URN: {} -> {}",
                "▶".cyan(),
                job.id,
                t.urn,
                t.output_format,
            );
            Ok(())
        }
        JobPayload::Upload(u) => {
            tracing::info!(bucket = %u.bucket_key, object = %u.object_key, "Processing upload job");
            println!(
                "  {} Upload job {} — {}/{}",
                "▶".cyan(),
                job.id,
                u.bucket_key,
                u.object_key,
            );
            Ok(())
        }
        JobPayload::ExtractProps(e) => {
            tracing::info!(urn = %e.urn, "Processing extract-props job");
            println!(
                "  {} ExtractProps job {} — URN: {}",
                "▶".cyan(),
                job.id,
                e.urn
            );
            Ok(())
        }
        JobPayload::Pipeline(p) => {
            tracing::info!(pipeline = %p.pipeline_name, "Processing pipeline job");
            println!(
                "  {} Pipeline job {} — {}",
                "▶".cyan(),
                job.id,
                p.pipeline_name,
            );
            Ok(())
        }
    }
}

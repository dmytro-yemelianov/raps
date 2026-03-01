// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Swarm orchestration commands.
//!
//! Introspect resilience layer state: circuit breakers, rate budgets,
//! response cache, metrics, checkpoints.

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
    pub fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            SwarmCommands::Status => swarm_status(output_format),
            SwarmCommands::Metrics => swarm_metrics(output_format),
            SwarmCommands::Resume => swarm_resume(output_format),
            SwarmCommands::Audit { date, limit } => swarm_audit(date, limit, output_format),
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
                    println!("  {} {} (failures: {})", cb.endpoint, state_colored, cb.failures);
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
                    println!("  {} {}/{} ({})", rb.endpoint, rb.remaining, rb.limit, status);
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

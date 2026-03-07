// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Aggregate usage statistics dashboard.
//!
//! Reads from existing cache files:
//! - `~/.cache/raps/endpoint_stats.json` — per-endpoint request/failure/latency data
//! - `~/.cache/xyz.rapscli.raps/history.json` — recent command invocations
//! - `~/.cache/raps/throughput_cache.json` — last observed upload throughput

use anyhow::Result;
use colored::Colorize;
use serde::Deserialize;
use std::collections::HashMap;

use crate::output::OutputFormat;
use raps_kernel::endpoint_stats::EndpointStats;

// ── Local deserialization types ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct HistoryEntry {
    #[allow(dead_code)]
    index: usize,
    #[allow(dead_code)]
    timestamp: String,
    args: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ThroughputCache {
    bytes_per_second: f64,
    measured_at: i64,
}

// ── Data loading helpers ──────────────────────────────────────────────────────

fn load_history() -> Vec<HistoryEntry> {
    let path = directories::ProjectDirs::from("xyz", "rapscli", "raps")
        .map(|d| d.cache_dir().join("history.json"));
    let path = match path {
        Some(p) => p,
        None => return vec![],
    };
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    serde_json::from_str::<Vec<HistoryEntry>>(&data).unwrap_or_default()
}

fn load_throughput() -> Option<ThroughputCache> {
    let path = directories::ProjectDirs::from("com", "autodesk", "raps")
        .map(|d| d.cache_dir().join("throughput_cache.json"))?;
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

// ── Formatting helpers ────────────────────────────────────────────────────────

fn fmt_ms(ms: u64) -> String {
    if ms >= 1000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}ms", ms)
    }
}

fn fmt_bytes_per_sec(bps: f64) -> String {
    const MB: f64 = 1_000_000.0;
    const KB: f64 = 1_000.0;
    if bps >= MB {
        format!("{:.1} MB/s", bps / MB)
    } else if bps >= KB {
        format!("{:.1} KB/s", bps / KB)
    } else {
        format!("{:.0} B/s", bps)
    }
}

fn fmt_chunk(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    format!("{} MB", bytes / MB)
}

fn suggested_chunk(bps: f64) -> u64 {
    if bps > 50_000_000.0 {
        25 * 1024 * 1024
    } else if bps > 10_000_000.0 {
        10 * 1024 * 1024
    } else {
        5 * 1024 * 1024
    }
}

// ── Command entry point ───────────────────────────────────────────────────────

pub fn execute(output_format: OutputFormat) -> Result<()> {
    let endpoint_stats = EndpointStats::load();
    let history = load_history();
    let throughput = load_throughput();

    match output_format {
        OutputFormat::Table => print_table(&endpoint_stats, &history, throughput.as_ref()),
        _ => print_structured(&endpoint_stats, &history, throughput.as_ref(), output_format),
    }
}

// ── Table output ──────────────────────────────────────────────────────────────

fn print_table(
    endpoint_stats: &EndpointStats,
    history: &[HistoryEntry],
    throughput: Option<&ThroughputCache>,
) -> Result<()> {
    let sep = "─".repeat(45);

    println!();
    println!("{}", "RAPS Usage Statistics".bold());
    println!("{}", sep);

    // ── Commands section ──────────────────────────────────────────────────────
    println!();
    println!("{}", "Commands (last 100)".bold().underline());

    let total_commands = history.len();
    println!("  {:<20} {}", "Total run:".bold(), total_commands);

    if !history.is_empty() {
        // Group by first two args (e.g. "raps bucket")
        let mut counts: HashMap<String, usize> = HashMap::new();
        for entry in history {
            let key = entry.args.iter().take(2).cloned().collect::<Vec<_>>().join(" ");
            *counts.entry(key).or_default() += 1;
        }
        let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        let top: Vec<String> = sorted
            .iter()
            .take(3)
            .map(|(cmd, n)| format!("{} ({}×)", cmd, n))
            .collect();
        println!("  {:<20} {}", "Most used:".bold(), top.join(", "));
    }

    // ── API Endpoints section ─────────────────────────────────────────────────
    println!();
    println!("{}", "API Endpoints".bold().underline());

    let records = &endpoint_stats.records;
    if records.is_empty() {
        println!("  {}", "No endpoint data recorded yet.".dimmed());
    } else {
        let total_requests: u64 = records.values().map(|r| r.requests).sum();
        let total_failures: u64 = records.values().map(|r| r.failures).sum();
        let total_ms: u64 = records.values().map(|r| r.total_ms).sum();
        let avg_ms = if total_requests > 0 { total_ms / total_requests } else { 0 };

        let failure_pct = if total_requests > 0 {
            total_failures as f64 / total_requests as f64 * 100.0
        } else {
            0.0
        };

        println!(
            "  {:<20} {}",
            "Total requests:".bold(),
            total_requests
        );
        println!(
            "  {:<20} {} ({:.1}%)",
            "Total failures:".bold(),
            total_failures,
            failure_pct
        );
        println!(
            "  {:<20} {}",
            "Avg response:".bold(),
            fmt_ms(avg_ms)
        );

        println!();
        println!("  {}", "Top 5 by call count:".bold());

        let mut sorted_records: Vec<(&String, &raps_kernel::endpoint_stats::EndpointRecord)> =
            records.iter().collect();
        sorted_records.sort_by(|a, b| b.1.requests.cmp(&a.1.requests));

        for (key, rec) in sorted_records.iter().take(5) {
            let avg = fmt_ms(rec.avg_ms());
            let failures_str = if rec.failures > 0 {
                format!("{} failures", rec.failures).red().to_string()
            } else {
                "0 failures".green().to_string()
            };
            println!(
                "  {:50} {:>8} calls  avg {:>6}  {}",
                key, rec.requests, avg, failures_str
            );
        }
    }

    // ── Upload throughput section ─────────────────────────────────────────────
    println!();
    println!("{}", "Upload throughput".bold().underline());

    match throughput {
        Some(t) if t.bytes_per_second > 0.0 => {
            let measured = chrono::DateTime::from_timestamp(t.measured_at, 0)
                .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%d %H:%M UTC").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let chunk = suggested_chunk(t.bytes_per_second);
            println!(
                "  {:<20} {}",
                "Last measured:".bold(),
                fmt_bytes_per_sec(t.bytes_per_second)
            );
            println!("  {:<20} {}", "Measured at:".bold(), measured);
            println!(
                "  {:<20} {}",
                "Recommended chunk:".bold(),
                fmt_chunk(chunk)
            );
        }
        _ => {
            println!("  {}", "No throughput data recorded yet.".dimmed());
            println!(
                "  {}",
                "Run `raps object upload` to measure throughput.".dimmed()
            );
        }
    }

    println!();
    Ok(())
}

// ── Structured output (JSON/YAML/CSV) ────────────────────────────────────────

fn print_structured(
    endpoint_stats: &EndpointStats,
    history: &[HistoryEntry],
    throughput: Option<&ThroughputCache>,
    output_format: OutputFormat,
) -> Result<()> {
    // Build command frequency map
    let mut cmd_counts: HashMap<String, usize> = HashMap::new();
    for entry in history {
        let key = entry.args.iter().take(2).cloned().collect::<Vec<_>>().join(" ");
        *cmd_counts.entry(key).or_default() += 1;
    }
    let mut cmd_sorted: Vec<(String, usize)> = cmd_counts.into_iter().collect();
    cmd_sorted.sort_by(|a, b| b.1.cmp(&a.1));
    let top_commands: Vec<serde_json::Value> = cmd_sorted
        .iter()
        .take(5)
        .map(|(cmd, n)| serde_json::json!({"command": cmd, "count": n}))
        .collect();

    let records = &endpoint_stats.records;
    let total_requests: u64 = records.values().map(|r| r.requests).sum();
    let total_failures: u64 = records.values().map(|r| r.failures).sum();
    let total_ms: u64 = records.values().map(|r| r.total_ms).sum();
    let avg_ms = if total_requests > 0 { total_ms / total_requests } else { 0 };

    let mut endpoints_sorted: Vec<(&String, &raps_kernel::endpoint_stats::EndpointRecord)> =
        records.iter().collect();
    endpoints_sorted.sort_by(|a, b| b.1.requests.cmp(&a.1.requests));
    let top_endpoints: Vec<serde_json::Value> = endpoints_sorted
        .iter()
        .take(5)
        .map(|(key, rec)| {
            serde_json::json!({
                "endpoint": key,
                "requests": rec.requests,
                "failures": rec.failures,
                "avg_ms": rec.avg_ms(),
            })
        })
        .collect();

    let throughput_json = match throughput {
        Some(t) if t.bytes_per_second > 0.0 => serde_json::json!({
            "bytes_per_second": t.bytes_per_second,
            "measured_at": t.measured_at,
            "recommended_chunk_bytes": suggested_chunk(t.bytes_per_second),
        }),
        _ => serde_json::Value::Null,
    };

    let output = serde_json::json!({
        "commands": {
            "total": history.len(),
            "top": top_commands,
        },
        "api_endpoints": {
            "total_requests": total_requests,
            "total_failures": total_failures,
            "avg_ms": avg_ms,
            "top": top_endpoints,
        },
        "upload_throughput": throughput_json,
    });

    output_format.write(&output)
}

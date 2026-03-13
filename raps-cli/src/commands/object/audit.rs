// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! `raps object audit <bucket>` — cost and access analysis for an OSS bucket.

use anyhow::Result;
use chrono::{DateTime, Utc};
use colored::Colorize;
use serde::Serialize;
use std::collections::HashMap;

use crate::output::OutputFormat;
use raps_oss::OssClient;

use super::format_size;

// ── Output types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AuditReport {
    pub bucket: String,
    pub total_objects: usize,
    pub total_size_bytes: u64,
    pub average_size_bytes: u64,
    pub largest_objects: Vec<LargestObject>,
    pub by_extension: Vec<ExtensionGroup>,
    pub by_age: AgeBrackets,
    pub stale_candidates: Vec<StaleObject>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct LargestObject {
    pub object_key: String,
    pub size_bytes: u64,
    pub size_human: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ExtensionGroup {
    pub extension: String,
    pub count: usize,
    pub total_size_bytes: u64,
    pub total_size_human: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AgeBrackets {
    pub less_than_1_day: usize,
    pub less_than_1_week: usize,
    pub less_than_1_month: usize,
    pub less_than_1_year: usize,
    pub older: usize,
    pub unknown: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct StaleObject {
    pub object_key: String,
    pub size_bytes: u64,
    pub size_human: String,
    pub days_since_modified: Option<u64>,
}

// ── Main handler ─────────────────────────────────────────────────────────────

pub(super) async fn audit_bucket(
    client: &OssClient,
    bucket: String,
    output_format: OutputFormat,
) -> Result<()> {
    eprintln!("{} Listing objects in bucket '{}'…", "→".cyan(), bucket);
    let objects = client.list_objects(&bucket).await?;

    if objects.is_empty() {
        println!("Bucket '{}' is empty.", bucket);
        return Ok(());
    }

    let now = Utc::now();
    let stale_threshold_days: u64 = 90;

    let total_objects = objects.len();
    let total_size_bytes: u64 = objects.iter().map(|o| o.size).sum();
    let average_size_bytes = if total_objects > 0 {
        total_size_bytes / total_objects as u64
    } else {
        0
    };

    // ── Largest 10 objects ─────────────────────────────────────────────────
    // Sort indices by descending size to avoid requiring Clone on ObjectItem
    let mut indices: Vec<usize> = (0..objects.len()).collect();
    indices.sort_by(|&a, &b| objects[b].size.cmp(&objects[a].size));
    let largest_objects: Vec<LargestObject> = indices
        .iter()
        .take(10)
        .map(|&i| LargestObject {
            object_key: objects[i].object_key.clone(),
            size_bytes: objects[i].size,
            size_human: format_size(objects[i].size),
        })
        .collect();

    // ── Group by file extension ────────────────────────────────────────────
    let mut ext_map: HashMap<String, (usize, u64)> = HashMap::new();
    for obj in &objects {
        let ext = std::path::Path::new(&obj.object_key)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.to_lowercase()))
            .unwrap_or_else(|| "(no extension)".to_string());
        let entry = ext_map.entry(ext).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += obj.size;
    }
    let mut by_extension: Vec<ExtensionGroup> = ext_map
        .into_iter()
        .map(|(ext, (count, total_size_bytes))| ExtensionGroup {
            extension: ext,
            count,
            total_size_bytes,
            total_size_human: format_size(total_size_bytes),
        })
        .collect();
    by_extension.sort_by(|a, b| b.total_size_bytes.cmp(&a.total_size_bytes));

    // ── Age brackets — based on object_key alphabetical heuristic ──────────
    // OSS does not return timestamps in the list response; we fetch details
    // for age analysis lazily by inspecting the object_id or key. Since the
    // list API returns no timestamps we use a best-effort approach: try to
    // extract a date from the object_key, otherwise classify as "unknown".
    // For a production implementation the details endpoint would be called
    // per-object, but that would be N+1 HTTP calls which is too slow for
    // large buckets.  We therefore work with what the list gives us.
    let mut age = AgeBrackets {
        less_than_1_day: 0,
        less_than_1_week: 0,
        less_than_1_month: 0,
        less_than_1_year: 0,
        older: 0,
        unknown: 0,
    };

    let mut stale_candidates: Vec<StaleObject> = Vec::new();

    for obj in &objects {
        // Attempt to parse a date embedded in the object key (YYYY-MM-DD or YYYYMMdd)
        let parsed_date = extract_date_from_key(&obj.object_key);

        match parsed_date {
            Some(dt) => {
                let days = (now - dt).num_days().unsigned_abs();
                if days < 1 {
                    age.less_than_1_day += 1;
                } else if days < 7 {
                    age.less_than_1_week += 1;
                } else if days < 30 {
                    age.less_than_1_month += 1;
                } else if days < 365 {
                    age.less_than_1_year += 1;
                } else {
                    age.older += 1;
                }
                if days >= stale_threshold_days {
                    stale_candidates.push(StaleObject {
                        object_key: obj.object_key.clone(),
                        size_bytes: obj.size,
                        size_human: format_size(obj.size),
                        days_since_modified: Some(days),
                    });
                }
            }
            None => {
                age.unknown += 1;
                // Without a date we conservatively add to stale list with None
                stale_candidates.push(StaleObject {
                    object_key: obj.object_key.clone(),
                    size_bytes: obj.size,
                    size_human: format_size(obj.size),
                    days_since_modified: None,
                });
            }
        }
    }

    // Sort stale candidates: known-age first (oldest first), then unknowns
    stale_candidates.sort_by(
        |a, b| match (a.days_since_modified, b.days_since_modified) {
            (Some(da), Some(db)) => db.cmp(&da),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.object_key.cmp(&b.object_key),
        },
    );

    let report = AuditReport {
        bucket: bucket.clone(),
        total_objects,
        total_size_bytes,
        average_size_bytes,
        largest_objects,
        by_extension,
        by_age: age,
        stale_candidates,
    };

    match output_format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Csv => {
            print_csv(&report)?;
        }
        _ => {
            print_table(&report);
        }
    }

    Ok(())
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn print_table(r: &AuditReport) {
    println!();
    println!("{}", format!("Audit: {}", r.bucket).bold().underline());
    println!();

    // Summary
    println!("{}", "Summary".bold());
    println!(
        "  {:25} {}",
        "Total objects:",
        r.total_objects.to_string().cyan()
    );
    println!(
        "  {:25} {}",
        "Total size:",
        format_size(r.total_size_bytes).cyan()
    );
    println!(
        "  {:25} {}",
        "Average size:",
        format_size(r.average_size_bytes).cyan()
    );
    println!();

    // Top 10 largest
    println!("{}", "Largest 10 Objects".bold());
    println!("  {:<12}  {}", "Size", "Key");
    println!("  {}", "-".repeat(60));
    for obj in &r.largest_objects {
        println!("  {:<12}  {}", obj.size_human.yellow(), obj.object_key);
    }
    println!();

    // By extension
    println!("{}", "By File Extension".bold());
    println!(
        "  {:<16}  {:>8}  {:>14}",
        "Extension", "Count", "Total Size"
    );
    println!("  {}", "-".repeat(44));
    for g in &r.by_extension {
        println!(
            "  {:<16}  {:>8}  {:>14}",
            g.extension.cyan(),
            g.count,
            g.total_size_human.yellow()
        );
    }
    println!();

    // By age
    println!("{}", "By Age Bracket".bold());
    println!("  {:<20}  {:>8}", "Bracket", "Count");
    println!("  {}", "-".repeat(30));
    println!(
        "  {:<20}  {:>8}",
        "< 1 day",
        r.by_age.less_than_1_day.to_string().green()
    );
    println!(
        "  {:<20}  {:>8}",
        "< 1 week",
        r.by_age.less_than_1_week.to_string().green()
    );
    println!(
        "  {:<20}  {:>8}",
        "< 1 month",
        r.by_age.less_than_1_month.to_string().yellow()
    );
    println!(
        "  {:<20}  {:>8}",
        "< 1 year",
        r.by_age.less_than_1_year.to_string().yellow()
    );
    println!(
        "  {:<20}  {:>8}",
        ">= 1 year",
        r.by_age.older.to_string().red()
    );
    println!(
        "  {:<20}  {:>8}",
        "unknown (no date in key)",
        r.by_age.unknown.to_string().dimmed()
    );
    println!();

    // Stale candidates
    if r.stale_candidates.is_empty() {
        println!("{}", "No stale candidates (>90 days).".green());
    } else {
        println!(
            "{}",
            format!(
                "Stale Candidates (>90 days): {} objects",
                r.stale_candidates.len()
            )
            .bold()
            .yellow()
        );
        println!("  {:<12}  {:>8}  {}", "Days Old", "Size", "Key");
        println!("  {}", "-".repeat(60));
        for obj in r.stale_candidates.iter().take(20) {
            let days_str = obj
                .days_since_modified
                .map(|d| d.to_string())
                .unwrap_or_else(|| "?".to_string());
            println!(
                "  {:<12}  {:>8}  {}",
                days_str.red(),
                obj.size_human,
                obj.object_key
            );
        }
        if r.stale_candidates.len() > 20 {
            println!(
                "  … and {} more (use --output json for full list)",
                r.stale_candidates.len() - 20
            );
        }
    }
    println!();
}

fn print_csv(r: &AuditReport) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    wtr.write_record(["section", "key", "value"])?;
    wtr.write_record(["summary", "total_objects", &r.total_objects.to_string()])?;
    wtr.write_record([
        "summary",
        "total_size_bytes",
        &r.total_size_bytes.to_string(),
    ])?;
    wtr.write_record([
        "summary",
        "average_size_bytes",
        &r.average_size_bytes.to_string(),
    ])?;
    for obj in &r.largest_objects {
        wtr.write_record(["largest", &obj.object_key, &obj.size_bytes.to_string()])?;
    }
    for g in &r.by_extension {
        wtr.write_record([
            "by_extension",
            &g.extension,
            &format!("{} files, {} bytes", g.count, g.total_size_bytes),
        ])?;
    }
    for obj in &r.stale_candidates {
        wtr.write_record([
            "stale",
            &obj.object_key,
            &obj.days_since_modified
                .map(|d| d.to_string())
                .unwrap_or_default(),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Try to extract a UTC date from common patterns embedded in an object key.
/// Looks for YYYY-MM-DD or YYYYMMdd substrings.
fn extract_date_from_key(key: &str) -> Option<DateTime<Utc>> {
    // Pattern: YYYY-MM-DD
    let re_dashed = regex::Regex::new(r"(\d{4})-(\d{2})-(\d{2})").ok()?;
    if let Some(cap) = re_dashed.captures(key) {
        let y: i32 = cap[1].parse().ok()?;
        let m: u32 = cap[2].parse().ok()?;
        let d: u32 = cap[3].parse().ok()?;
        let naive = chrono::NaiveDate::from_ymd_opt(y, m, d)?.and_hms_opt(0, 0, 0)?;
        return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
    }

    // Pattern: YYYYMMdd (8-digit run)
    let re_compact = regex::Regex::new(r"(\d{4})(\d{2})(\d{2})").ok()?;
    if let Some(cap) = re_compact.captures(key) {
        let y: i32 = cap[1].parse().ok()?;
        let m: u32 = cap[2].parse().ok()?;
        let d: u32 = cap[3].parse().ok()?;
        // Guard against false positives (e.g. UUIDs)
        if m >= 1 && m <= 12 && d >= 1 && d <= 31 {
            let naive = chrono::NaiveDate::from_ymd_opt(y, m, d)?.and_hms_opt(0, 0, 0)?;
            return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_date_dashed() {
        let dt = extract_date_from_key("models/2023-06-15/arch.rvt");
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2023-06-15");
    }

    #[test]
    fn test_extract_date_compact() {
        let dt = extract_date_from_key("backup_20220301_model.dwg");
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2022-03-01");
    }

    #[test]
    fn test_extract_date_none() {
        let dt = extract_date_from_key("model-no-date.rvt");
        assert!(dt.is_none());
    }
}

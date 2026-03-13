// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Translation timeline: show all derivatives for a URN as a table.

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_derivative::DerivativeClient;

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct DerivativeTimelineEntry {
    format: String,
    status: String,
    progress: Option<String>,
    name: Option<String>,
    output_files: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<Vec<TimelineFileEntry>>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct TimelineFileEntry {
    name: Option<String>,
    role: String,
    size: Option<u64>,
    mime: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct TimelineOutput {
    urn: String,
    manifest_status: String,
    manifest_progress: String,
    region: String,
    derivatives: Vec<DerivativeTimelineEntry>,
}

pub(super) async fn show_timeline(
    client: &DerivativeClient,
    urn: &str,
    verbose: bool,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching translation manifest...".dimmed());
    }

    let manifest = client.get_manifest(urn).await?;

    let entries: Vec<DerivativeTimelineEntry> = manifest
        .derivatives
        .iter()
        .map(|d| {
            // Count leaf output files (children without further children that have a urn)
            let file_entries: Vec<TimelineFileEntry> = collect_files(&d.children);
            let file_count = file_entries.len();
            DerivativeTimelineEntry {
                format: d.output_type.clone(),
                status: d.status.clone(),
                progress: d.progress.clone(),
                name: d.name.clone(),
                output_files: file_count,
                files: if verbose && !file_entries.is_empty() {
                    Some(file_entries)
                } else {
                    None
                },
            }
        })
        .collect();

    let output = TimelineOutput {
        urn: manifest.urn.clone(),
        manifest_status: manifest.status.clone(),
        manifest_progress: manifest.progress.clone(),
        region: manifest.region.clone(),
        derivatives: entries,
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Translation Timeline".bold());
            println!("{}", "─".repeat(75));
            println!("  {} {}", "URN:".bold(), truncate_str(urn, 60).dimmed());
            let status_icon = match manifest.status.as_str() {
                "success" => "\u{2713}".green().bold().to_string(),
                "failed" | "timeout" => "X".red().bold().to_string(),
                "inprogress" | "pending" => "...".yellow().bold().to_string(),
                _ => "?".dimmed().to_string(),
            };
            println!("  {} {} {}", "Status:".bold(), status_icon, manifest.status);
            println!("  {} {}", "Progress:".bold(), manifest.progress);
            println!("  {} {}", "Region:".bold(), manifest.region);

            if output.derivatives.is_empty() {
                println!(
                    "\n{}",
                    "No derivatives found. Run 'raps translate start' first.".yellow()
                );
            } else {
                println!("\n{}", "Derivatives:".bold());
                println!("{}", "─".repeat(75));
                println!(
                    "{:<12} {:<14} {:<12} {:<30} {:>6}",
                    "Format".bold(),
                    "Status".bold(),
                    "Progress".bold(),
                    "Name".bold(),
                    "Files".bold()
                );
                println!("{}", "─".repeat(75));

                for entry in &output.derivatives {
                    let status_str = match entry.status.as_str() {
                        "success" => entry.status.green().to_string(),
                        "failed" | "timeout" => entry.status.red().to_string(),
                        "inprogress" | "pending" => entry.status.yellow().to_string(),
                        _ => entry.status.clone(),
                    };
                    let progress = entry.progress.as_deref().unwrap_or("-");
                    let name = entry.name.as_deref().unwrap_or("-");
                    println!(
                        "{:<12} {:<14} {:<12} {:<30} {:>6}",
                        entry.format.cyan(),
                        status_str,
                        progress,
                        truncate_str(name, 30),
                        entry.output_files
                    );

                    if verbose {
                        if let Some(ref files) = entry.files {
                            for f in files {
                                let fname = f.name.as_deref().unwrap_or("-");
                                let size_str = f
                                    .size
                                    .map(|s| format_size(s))
                                    .unwrap_or_else(|| "-".to_string());
                                let mime = f.mime.as_deref().unwrap_or("");
                                println!(
                                    "    {} {} ({}) {}",
                                    "↳".dimmed(),
                                    fname.dimmed(),
                                    f.role.dimmed(),
                                    if mime.is_empty() {
                                        size_str.dimmed().to_string()
                                    } else {
                                        format!("{} | {}", size_str, mime).dimmed().to_string()
                                    }
                                );
                            }
                        }
                    }
                }

                println!("{}", "─".repeat(75));
                println!(
                    "{} {} derivative(s) found",
                    "→".cyan(),
                    output.derivatives.len()
                );
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

fn collect_files(children: &[raps_derivative::DerivativeChild]) -> Vec<TimelineFileEntry> {
    let mut out = Vec::new();
    for child in children {
        if child.children.is_empty() {
            out.push(TimelineFileEntry {
                name: child.name.clone(),
                role: child.role.clone(),
                size: child.size,
                mime: child.mime.clone(),
            });
        } else {
            out.extend(collect_files(&child.children));
        }
    }
    out
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

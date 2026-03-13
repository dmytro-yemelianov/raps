// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Bulk export ACC project data: issues, RFIs, submittals, checklists.

use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;

use crate::output::OutputFormat;
use raps_acc::{AccClient, IssuesClient, RfiClient};

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct ExportSummary {
    project_id: String,
    output_dir: String,
    issues: usize,
    rfis: usize,
    submittals: usize,
    checklists: usize,
}

fn make_progress_bar(label: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message(format!("Exporting {}...", label));
    pb
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn export_project(
    acc_client: &AccClient,
    issues_client: &IssuesClient,
    rfi_client: &RfiClient,
    project_id: &str,
    output_dir: Option<PathBuf>,
    output_format: OutputFormat,
) -> Result<()> {
    // Build output directory path
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let dir = output_dir
        .unwrap_or_else(|| PathBuf::from(format!("acc-export-{}-{}", project_id, timestamp)));

    tokio::fs::create_dir_all(&dir)
        .await
        .with_context(|| format!("Failed to create output directory: {}", dir.display()))?;

    if output_format.supports_colors() {
        println!(
            "{} Exporting project {} to {}",
            "→".cyan(),
            project_id.cyan(),
            dir.display().to_string().cyan()
        );
    }

    // Export Issues
    let pb = make_progress_bar("issues");
    let issues = issues_client
        .list_issues(project_id, None)
        .await
        .unwrap_or_default();
    let issues_count = issues.len();
    let issues_json =
        serde_json::to_string_pretty(&issues).context("Failed to serialize issues")?;
    let issues_path = dir.join("issues.json");
    tokio::fs::write(&issues_path, issues_json)
        .await
        .with_context(|| format!("Failed to write {}", issues_path.display()))?;
    pb.finish_with_message(format!(
        "{} {} issues exported → {}",
        "\u{2713}".green(),
        issues_count,
        issues_path.display()
    ));

    // Export RFIs
    let pb = make_progress_bar("RFIs");
    let rfis = rfi_client.list_rfis(project_id).await.unwrap_or_default();
    let rfis_count = rfis.len();
    let rfis_json = serde_json::to_string_pretty(&rfis).context("Failed to serialize RFIs")?;
    let rfis_path = dir.join("rfis.json");
    tokio::fs::write(&rfis_path, rfis_json)
        .await
        .with_context(|| format!("Failed to write {}", rfis_path.display()))?;
    pb.finish_with_message(format!(
        "{} {} RFIs exported → {}",
        "\u{2713}".green(),
        rfis_count,
        rfis_path.display()
    ));

    // Export Submittals
    let pb = make_progress_bar("submittals");
    let submittals = acc_client
        .list_submittals(project_id)
        .await
        .unwrap_or_default();
    let submittals_count = submittals.len();
    let submittals_json =
        serde_json::to_string_pretty(&submittals).context("Failed to serialize submittals")?;
    let submittals_path = dir.join("submittals.json");
    tokio::fs::write(&submittals_path, submittals_json)
        .await
        .with_context(|| format!("Failed to write {}", submittals_path.display()))?;
    pb.finish_with_message(format!(
        "{} {} submittals exported → {}",
        "\u{2713}".green(),
        submittals_count,
        submittals_path.display()
    ));

    // Export Checklists
    let pb = make_progress_bar("checklists");
    let checklists = acc_client
        .list_checklists(project_id)
        .await
        .unwrap_or_default();
    let checklists_count = checklists.len();
    let checklists_json =
        serde_json::to_string_pretty(&checklists).context("Failed to serialize checklists")?;
    let checklists_path = dir.join("checklists.json");
    tokio::fs::write(&checklists_path, checklists_json)
        .await
        .with_context(|| format!("Failed to write {}", checklists_path.display()))?;
    pb.finish_with_message(format!(
        "{} {} checklists exported → {}",
        "\u{2713}".green(),
        checklists_count,
        checklists_path.display()
    ));

    let summary = ExportSummary {
        project_id: project_id.to_string(),
        output_dir: dir.display().to_string(),
        issues: issues_count,
        rfis: rfis_count,
        submittals: submittals_count,
        checklists: checklists_count,
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Export complete!".bold());
            println!("{}", "─".repeat(50));
            println!("  {} {}", "Project:".bold(), project_id.cyan());
            println!("  {} {}", "Output dir:".bold(), dir.display());
            println!(
                "  {} {}",
                "Issues:".bold(),
                issues_count.to_string().green()
            );
            println!("  {} {}", "RFIs:".bold(), rfis_count.to_string().green());
            println!(
                "  {} {}",
                "Submittals:".bold(),
                submittals_count.to_string().green()
            );
            println!(
                "  {} {}",
                "Checklists:".bold(),
                checklists_count.to_string().green()
            );
            println!("{}", "─".repeat(50));
            println!(
                "  {} {} items exported",
                "\u{2713}".green().bold(),
                (issues_count + rfis_count + submittals_count + checklists_count)
                    .to_string()
                    .green()
                    .bold()
            );
        }
        _ => {
            output_format.write(&summary)?;
        }
    }

    Ok(())
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! CRUD operations for RFIs: list, get, create, update, delete.

use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

use crate::output::OutputFormat;
use raps_acc::{CreateRfiRequest, RfiClient, UpdateRfiRequest};

use super::{RfiOutput, truncate_str};

pub(super) async fn list_rfis(
    client: &RfiClient,
    project_id: &str,
    status_filter: Option<&str>,
    since: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching RFIs...".dimmed());
    }

    let rfis = client.list_rfis(project_id).await.context(format!(
        "Failed to list RFIs for project '{}'. Verify project ID (without 'b.' prefix)",
        project_id
    ))?;

    // Filter by status if provided
    let filtered: Vec<_> = if let Some(status) = status_filter {
        rfis.into_iter()
            .filter(|r| r.status.to_lowercase() == status.to_lowercase())
            .collect()
    } else {
        rfis
    };

    // Apply since filter if provided
    let filtered: Vec<_> = if let Some(ref since_date) = since {
        filtered
            .into_iter()
            .filter(|r| {
                r.created_at
                    .as_ref()
                    .map(|d| d.as_str() >= since_date.as_str())
                    .unwrap_or(true)
            })
            .collect()
    } else {
        filtered
    };

    let outputs: Vec<RfiOutput> = filtered
        .iter()
        .map(|r| RfiOutput {
            id: r.id.clone(),
            number: r.number.clone(),
            title: r.title.clone(),
            status: r.status.clone(),
            priority: r.priority.clone(),
            question: r.question.clone(),
            answer: r.answer.clone(),
            due_date: r.due_date.clone(),
            assigned_to_name: r.assigned_to_name.clone(),
            created_at: r.created_at.clone(),
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No RFIs found.".yellow()),
            _ => output_format.write(&Vec::<RfiOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "RFIs:".bold());
            println!("{}", "─".repeat(100));
            println!(
                "{:<10} {:<40} {:<12} {:<10} {}",
                "Number".bold(),
                "Title".bold(),
                "Status".bold(),
                "Priority".bold(),
                "Due Date".bold()
            );
            println!("{}", "─".repeat(100));

            for rfi in &outputs {
                let number = rfi.number.as_deref().unwrap_or("-");
                let priority = rfi.priority.as_deref().unwrap_or("-");
                let due = rfi.due_date.as_deref().unwrap_or("-");
                let status_color = match rfi.status.to_lowercase().as_str() {
                    "closed" => rfi.status.green().to_string(),
                    "answered" => rfi.status.cyan().to_string(),
                    "open" => rfi.status.yellow().to_string(),
                    "void" => rfi.status.dimmed().to_string(),
                    _ => rfi.status.clone(),
                };
                let priority_color = match priority.to_lowercase().as_str() {
                    "critical" => priority.red().bold().to_string(),
                    "high" => priority.red().to_string(),
                    "normal" => priority.to_string(),
                    "low" => priority.dimmed().to_string(),
                    _ => priority.to_string(),
                };

                println!(
                    "{:<10} {:<40} {:<12} {:<10} {}",
                    number.cyan(),
                    truncate_str(&rfi.title, 40),
                    status_color,
                    priority_color,
                    due.dimmed()
                );
            }

            println!("{}", "─".repeat(100));
            println!("{} {} RFI(s) found", "→".cyan(), outputs.len());
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

pub(super) async fn get_rfi(
    client: &RfiClient,
    project_id: &str,
    rfi_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching RFI details...".dimmed());
    }

    let rfi = client.get_rfi(project_id, rfi_id).await.context(format!(
        "Failed to get RFI '{}'. Verify the RFI ID exists",
        rfi_id
    ))?;

    let output = RfiOutput {
        id: rfi.id.clone(),
        number: rfi.number.clone(),
        title: rfi.title.clone(),
        status: rfi.status.clone(),
        priority: rfi.priority.clone(),
        question: rfi.question.clone(),
        answer: rfi.answer.clone(),
        due_date: rfi.due_date.clone(),
        assigned_to_name: rfi.assigned_to_name.clone(),
        created_at: rfi.created_at.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "RFI Details:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "ID:".bold(), rfi.id.cyan());
            println!(
                "{:<15} {}",
                "Number:".bold(),
                rfi.number.as_deref().unwrap_or("-")
            );
            println!("{:<15} {}", "Title:".bold(), rfi.title);
            println!("{:<15} {}", "Status:".bold(), rfi.status);
            println!(
                "{:<15} {}",
                "Priority:".bold(),
                rfi.priority.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Due Date:".bold(),
                rfi.due_date.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Assigned To:".bold(),
                rfi.assigned_to_name.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Created At:".bold(),
                rfi.created_at.as_deref().unwrap_or("-")
            );

            if let Some(q) = &rfi.question {
                println!("{}", "─".repeat(60));
                println!("{}", "Question:".bold());
                println!("{}", q);
            }

            if let Some(a) = &rfi.answer {
                println!("{}", "─".repeat(60));
                println!("{}", "Answer:".bold().green());
                println!("{}", a);
            }

            println!("{}", "─".repeat(60));
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

#[derive(serde::Deserialize)]
pub(super) struct CsvRfiRow {
    pub(super) title: String,
    pub(super) description: Option<String>,
    pub(super) assigned_to: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn create_rfi(
    client: &RfiClient,
    project_id: &str,
    title: Option<String>,
    question: Option<String>,
    priority: &str,
    due_date: Option<String>,
    assigned_to: Option<String>,
    location: Option<String>,
    discipline: Option<String>,
    from_csv: Option<PathBuf>,
    output_format: OutputFormat,
) -> Result<()> {
    // CSV bulk import flow
    if let Some(csv_path) = from_csv {
        return create_rfis_from_csv(client, project_id, &csv_path, output_format).await;
    }

    let rfi_title = match title {
        Some(t) => t,
        None => {
            anyhow::bail!("RFI title is required. Use --title flag or --from-csv for bulk import.");
        }
    };

    if output_format.supports_colors() {
        println!("{}", "Creating RFI...".dimmed());
    }

    let request = CreateRfiRequest {
        title: rfi_title.to_string(),
        question,
        priority: Some(priority.to_string()),
        due_date,
        assigned_to,
        location,
        discipline,
    };

    let rfi = client
        .create_rfi(project_id, request)
        .await
        .context("Failed to create RFI. Verify your permissions on this project")?;

    let output = RfiOutput {
        id: rfi.id.clone(),
        number: rfi.number.clone(),
        title: rfi.title.clone(),
        status: rfi.status.clone(),
        priority: rfi.priority.clone(),
        question: rfi.question.clone(),
        answer: rfi.answer.clone(),
        due_date: rfi.due_date.clone(),
        assigned_to_name: rfi.assigned_to_name.clone(),
        created_at: rfi.created_at.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{} RFI created successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), rfi.id.cyan());
            println!(
                "{:<15} {}",
                "Number:".bold(),
                rfi.number.as_deref().unwrap_or("-")
            );
            println!("{:<15} {}", "Title:".bold(), rfi.title);
            println!("{:<15} {}", "Status:".bold(), rfi.status);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

async fn create_rfis_from_csv(
    client: &RfiClient,
    project_id: &str,
    csv_path: &PathBuf,
    output_format: OutputFormat,
) -> Result<()> {
    // Parse CSV file
    let mut reader = csv::Reader::from_path(csv_path)
        .with_context(|| format!("Failed to open CSV file: {}", csv_path.display()))?;

    let mut rows: Vec<CsvRfiRow> = Vec::new();
    let mut validation_errors: Vec<String> = Vec::new();

    for (i, result) in reader.deserialize().enumerate() {
        match result {
            Ok(row) => {
                let row: CsvRfiRow = row;
                // Validate title is non-empty
                if row.title.trim().is_empty() {
                    validation_errors.push(format!("Row {}: title is empty", i + 2));
                    continue;
                }
                rows.push(row);
            }
            Err(e) => {
                validation_errors.push(format!("Row {}: parse error: {}", i + 2, e));
            }
        }
    }

    // Report validation errors
    if !validation_errors.is_empty() {
        if output_format.supports_colors() {
            println!("{} CSV validation errors:", "✗".red().bold());
            for err in &validation_errors {
                println!("  {} {}", "•".red(), err);
            }
        }
        anyhow::bail!(
            "CSV validation failed with {} error(s). Fix errors before proceeding.",
            validation_errors.len()
        );
    }

    if rows.is_empty() {
        anyhow::bail!("No valid rows found in CSV file");
    }

    if output_format.supports_colors() {
        println!(
            "\n{} Bulk RFI creation: {} RFIs from {}",
            "→".cyan(),
            rows.len().to_string().green(),
            csv_path.display().to_string().cyan()
        );
        println!();
    }

    let mut created = 0usize;
    let mut failed = 0usize;
    let mut errors: Vec<String> = Vec::new();

    // Progress bar
    let progress_bar = if output_format.supports_colors() {
        let pb = ProgressBar::new(rows.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                .expect("valid progress template")
                .progress_chars("=>-"),
        );
        Some(pb)
    } else {
        None
    };

    for row in &rows {
        if let Some(ref pb) = progress_bar {
            pb.set_message(truncate_str(&row.title, 30));
        }

        let request = CreateRfiRequest {
            title: row.title.clone(),
            question: row.description.clone(),
            priority: Some("normal".to_string()),
            due_date: None,
            assigned_to: row.assigned_to.clone(),
            location: None,
            discipline: None,
        };

        match client.create_rfi(project_id, request).await {
            Ok(_) => {
                created += 1;
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("{}: {}", row.title, e));
            }
        }

        if let Some(ref pb) = progress_bar {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    // Display summary
    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Bulk RFI Creation Results:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "Total:".bold(), rows.len());
            println!("{:<15} {}", "Created:".bold(), created.to_string().green());
            println!("{:<15} {}", "Failed:".bold(), failed.to_string().red());
            println!("{}", "─".repeat(60));

            if !errors.is_empty() {
                println!("\n{}", "Errors:".red().bold());
                for err in &errors {
                    println!("  {} {}", "✗".red(), err.dimmed());
                }
            }

            if failed == 0 {
                println!(
                    "\n{} All {} RFI(s) created successfully!",
                    "✓".green().bold(),
                    created
                );
            }
        }
        _ => {
            #[derive(Serialize)]
            struct BulkCreateResult {
                total: usize,
                created: usize,
                failed: usize,
                errors: Vec<String>,
            }

            let result = BulkCreateResult {
                total: rows.len(),
                created,
                failed,
                errors,
            };
            output_format.write(&result)?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn update_rfi(
    client: &RfiClient,
    project_id: &str,
    rfi_id: &str,
    title: Option<String>,
    question: Option<String>,
    answer: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    due_date: Option<String>,
    assigned_to: Option<String>,
    location: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Updating RFI...".dimmed());
    }

    let request = UpdateRfiRequest {
        title,
        question,
        answer,
        status,
        priority,
        due_date,
        assigned_to,
        location,
    };

    let rfi = client
        .update_rfi(project_id, rfi_id, request)
        .await
        .context(format!(
            "Failed to update RFI '{}'. Check permissions",
            rfi_id
        ))?;

    let output = RfiOutput {
        id: rfi.id.clone(),
        number: rfi.number.clone(),
        title: rfi.title.clone(),
        status: rfi.status.clone(),
        priority: rfi.priority.clone(),
        question: rfi.question.clone(),
        answer: rfi.answer.clone(),
        due_date: rfi.due_date.clone(),
        assigned_to_name: rfi.assigned_to_name.clone(),
        created_at: rfi.created_at.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{} RFI updated successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), rfi.id.cyan());
            println!("{:<15} {}", "Title:".bold(), rfi.title);
            println!("{:<15} {}", "Status:".bold(), rfi.status);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

pub(super) async fn delete_rfi(
    client: &RfiClient,
    project_id: &str,
    rfi_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Deleting RFI...".dimmed());
    }

    client
        .delete_rfi(project_id, rfi_id)
        .await
        .context(format!(
            "Failed to delete RFI '{}'. Verify the RFI ID exists and you have permissions",
            rfi_id
        ))?;

    #[derive(Serialize)]
    struct DeleteRfiOutput {
        success: bool,
        rfi_id: String,
        message: String,
    }

    let output = DeleteRfiOutput {
        success: true,
        rfi_id: rfi_id.to_string(),
        message: "RFI deleted successfully".to_string(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{} {}", "✓".green().bold(), output.message);
            println!("{:<15} {}", "ID:".bold(), rfi_id.cyan());
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

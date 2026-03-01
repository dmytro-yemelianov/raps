// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Submittal CRUD command handlers

use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_acc::{AccClient, CreateSubmittalRequest, UpdateSubmittalRequest};

use super::truncate_str;

#[derive(Serialize, schemars::JsonSchema)]
struct SubmittalOutput {
    id: String,
    title: String,
    number: Option<String>,
    status: String,
    due_date: Option<String>,
}

pub(super) async fn list_submittals(
    client: &AccClient,
    project_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching submittals...".dimmed());
    }

    let submittals = client.list_submittals(project_id).await.context(format!(
        "Failed to list submittals for project '{}'",
        project_id
    ))?;

    let outputs: Vec<SubmittalOutput> = submittals
        .iter()
        .map(|s| SubmittalOutput {
            id: s.id.clone(),
            title: s.title.clone(),
            number: s.number.clone(),
            status: s.status.clone(),
            due_date: s.due_date.clone(),
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No submittals found.".yellow()),
            _ => output_format.write(&Vec::<SubmittalOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Submittals:".bold());
            println!("{}", "─".repeat(90));
            println!(
                "{:<10} {:<45} {:<15} {}",
                "Number".bold(),
                "Title".bold(),
                "Status".bold(),
                "Due Date".bold()
            );
            println!("{}", "─".repeat(90));

            for submittal in &outputs {
                let number = submittal.number.as_deref().unwrap_or("-");
                let due = submittal.due_date.as_deref().unwrap_or("-");
                let status_color = match submittal.status.to_lowercase().as_str() {
                    "approved" => submittal.status.green().to_string(),
                    "rejected" => submittal.status.red().to_string(),
                    "pending" => submittal.status.yellow().to_string(),
                    _ => submittal.status.clone(),
                };
                println!(
                    "{:<10} {:<45} {:<15} {}",
                    number.cyan(),
                    truncate_str(&submittal.title, 45),
                    status_color,
                    due.dimmed()
                );
            }

            println!("{}", "─".repeat(90));
            println!("{} {} submittal(s) found", "→".cyan(), outputs.len());
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

pub(super) async fn get_submittal(
    client: &AccClient,
    project_id: &str,
    submittal_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching submittal details...".dimmed());
    }

    let submittal = client
        .get_submittal(project_id, submittal_id)
        .await
        .context(format!(
            "Failed to get submittal '{}'. Verify the submittal ID exists",
            submittal_id
        ))?;

    let output = SubmittalOutput {
        id: submittal.id.clone(),
        title: submittal.title.clone(),
        number: submittal.number.clone(),
        status: submittal.status.clone(),
        due_date: submittal.due_date.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Submittal Details:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "ID:".bold(), submittal.id.cyan());
            println!(
                "{:<15} {}",
                "Number:".bold(),
                submittal.number.as_deref().unwrap_or("-")
            );
            println!("{:<15} {}", "Title:".bold(), submittal.title);
            println!("{:<15} {}", "Status:".bold(), submittal.status);
            println!(
                "{:<15} {}",
                "Due Date:".bold(),
                submittal.due_date.as_deref().unwrap_or("-")
            );
            println!("{}", "─".repeat(60));
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct CsvSubmittalRow {
    title: String,
    description: Option<String>,
    spec_id: Option<String>,
}

pub(super) async fn create_submittal(
    client: &AccClient,
    project_id: &str,
    title: Option<String>,
    spec_section: Option<String>,
    due_date: Option<String>,
    from_csv: Option<PathBuf>,
    output_format: OutputFormat,
) -> Result<()> {
    // CSV bulk import flow
    if let Some(csv_path) = from_csv {
        return create_submittals_from_csv(client, project_id, &csv_path, output_format).await;
    }

    let submittal_title = match title {
        Some(t) => t,
        None => {
            anyhow::bail!(
                "Submittal title is required. Use --title flag or --from-csv for bulk import."
            );
        }
    };

    if output_format.supports_colors() {
        println!("{}", "Creating submittal...".dimmed());
    }

    let request = CreateSubmittalRequest {
        title: submittal_title,
        spec_section,
        due_date,
    };

    let submittal = client
        .create_submittal(project_id, request)
        .await
        .context("Failed to create submittal. Verify your permissions on this project")?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Submittal created successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), submittal.id.cyan());
            println!("{:<15} {}", "Title:".bold(), submittal.title);
        }
        _ => {
            output_format.write(&serde_json::json!({
                "id": submittal.id,
                "title": submittal.title,
                "created": true
            }))?;
        }
    }

    Ok(())
}

async fn create_submittals_from_csv(
    client: &AccClient,
    project_id: &str,
    csv_path: &PathBuf,
    output_format: OutputFormat,
) -> Result<()> {
    // Parse CSV file
    let mut reader = csv::Reader::from_path(csv_path)
        .with_context(|| format!("Failed to open CSV file: {}", csv_path.display()))?;

    let mut rows: Vec<CsvSubmittalRow> = Vec::new();
    let mut validation_errors: Vec<String> = Vec::new();

    for (i, result) in reader.deserialize().enumerate() {
        match result {
            Ok(row) => {
                let row: CsvSubmittalRow = row;
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
            "\n{} Bulk submittal creation: {} submittals from {}",
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
        Some(raps_kernel::progress::bulk_progress(rows.len() as u64, ""))
    } else {
        None
    };

    for row in &rows {
        if let Some(ref pb) = progress_bar {
            pb.set_message(truncate_str(&row.title, 30));
        }

        let request = CreateSubmittalRequest {
            title: row.title.clone(),
            spec_section: row.spec_id.clone(),
            due_date: None,
        };

        match client.create_submittal(project_id, request).await {
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
            println!("\n{}", "Bulk Submittal Creation Results:".bold());
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
                    "\n{} All {} submittal(s) created successfully!",
                    "✓".green().bold(),
                    created
                );
            }
        }
        _ => {
            #[derive(Serialize, schemars::JsonSchema)]
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

pub(super) async fn update_submittal(
    client: &AccClient,
    project_id: &str,
    submittal_id: &str,
    title: Option<String>,
    status: Option<String>,
    due_date: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Updating submittal...".dimmed());
    }

    let request = UpdateSubmittalRequest {
        title,
        status,
        due_date,
    };

    let submittal = client
        .update_submittal(project_id, submittal_id, request)
        .await
        .context(format!(
            "Failed to update submittal '{}'. Check permissions",
            submittal_id
        ))?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Submittal updated successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), submittal.id.cyan());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "id": submittal.id,
                "updated": true
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn delete_submittal(
    client: &AccClient,
    project_id: &str,
    submittal_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Deleting submittal...".dimmed());
    }

    client
        .delete_submittal(project_id, submittal_id)
        .await
        .context(format!(
            "Failed to delete submittal '{}'. Verify the submittal ID and your permissions",
            submittal_id
        ))?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Submittal deleted successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), submittal_id.cyan());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "id": submittal_id,
                "deleted": true
            }))?;
        }
    }

    Ok(())
}

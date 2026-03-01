// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Issue CRUD operations: list, create, update, delete, and issue types

use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::{Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

use crate::output::OutputFormat;
use raps_acc::{CreateIssueRequest, IssuesClient, UpdateIssueRequest};
use raps_kernel::interactive;

use super::truncate_str;

pub(super) async fn list_issues(
    client: &IssuesClient,
    project_id: &str,
    status: Option<String>,
    since: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching issues...".dimmed());
    }

    let filter = status.as_ref().map(|s| format!("status={}", s));
    let issues = client
        .list_issues(project_id, filter.as_deref())
        .await
        .context(format!(
            "Failed to list issues for project '{}'. Verify the project ID (without 'b.' prefix)",
            project_id
        ))?;

    // Apply since filter if provided
    let issues: Vec<_> = if let Some(ref since_date) = since {
        issues
            .into_iter()
            .filter(|i| {
                i.created_at
                    .as_ref()
                    .map(|d| d.as_str() >= since_date.as_str())
                    .unwrap_or(true)
            })
            .collect()
    } else {
        issues
    };

    if issues.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No issues found.".yellow()),
            OutputFormat::Json => println!("[]"),
            OutputFormat::Yaml => println!("[]"),
            OutputFormat::Csv => {
                println!("id,display_id,title,status,assigned_to,created_at,updated_at")
            }
            OutputFormat::Plain => println!("No issues found"),
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Issues:".bold());
            println!("{}", "─".repeat(90));
            println!(
                "{:<8} {:<12} {:<40} {}",
                "ID".bold(),
                "Status".bold(),
                "Title".bold(),
                "Assigned To".bold()
            );
            println!("{}", "─".repeat(90));

            for issue in &issues {
                let display_id = issue
                    .display_id
                    .map(|n| format!("#{}", n))
                    .unwrap_or_else(|| "-".to_string());

                let status_colored = match issue.status.as_str() {
                    "open" => issue.status.yellow(),
                    "closed" => issue.status.green(),
                    "answered" => issue.status.cyan(),
                    _ => issue.status.normal(),
                };

                let assigned = issue.assigned_to.as_deref().unwrap_or("-");

                println!(
                    "{:<8} {:<12} {:<40} {}",
                    display_id.cyan(),
                    status_colored,
                    truncate_str(&issue.title, 40),
                    assigned.dimmed()
                );
            }

            println!("{}", "─".repeat(90));
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&issues)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&issues)?);
        }
        OutputFormat::Csv => {
            println!("id,display_id,title,status,assigned_to,created_at,updated_at");
            for issue in &issues {
                let display_id = issue
                    .display_id
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "".to_string());

                let assigned = issue.assigned_to.as_deref().unwrap_or("");

                // Properly escape CSV fields that might contain commas or quotes
                let title = format!("\"{}\"", issue.title.replace("\"", "\"\""));
                let assigned = format!("\"{}\"", assigned.replace("\"", "\"\""));

                println!(
                    "{},{},{},{},{},{},{}",
                    issue.id,
                    display_id,
                    title,
                    issue.status,
                    assigned,
                    issue.created_at.clone().unwrap_or_default(),
                    issue.updated_at.clone().unwrap_or_default()
                );
            }
        }
        OutputFormat::Plain => {
            for issue in &issues {
                let display_id = issue
                    .display_id
                    .map(|n| format!("#{}", n))
                    .unwrap_or_else(|| "-".to_string());

                let assigned = issue.assigned_to.as_deref().unwrap_or("-");

                println!(
                    "{} {} {} {}",
                    display_id, issue.status, issue.title, assigned
                );
            }
        }
    }

    Ok(())
}

#[derive(serde::Deserialize)]
pub(super) struct CsvIssueRow {
    pub(super) title: String,
    pub(super) description: Option<String>,
    pub(super) status: Option<String>,
}

pub(super) async fn create_issue(
    client: &IssuesClient,
    project_id: &str,
    title: Option<String>,
    description: Option<String>,
    from_csv: Option<PathBuf>,
    output_format: OutputFormat,
) -> Result<()> {
    // CSV bulk import flow
    if let Some(csv_path) = from_csv {
        return create_issues_from_csv(client, project_id, &csv_path, output_format).await;
    }

    // Single issue creation flow
    // Get title
    let issue_title = match title {
        Some(t) => t,
        None => {
            // In non-interactive mode, require the title
            if interactive::is_non_interactive() {
                anyhow::bail!("Issue title is required in non-interactive mode. Use --title flag.");
            }
            raps_kernel::prompts::spawn_prompt(|| {
                Ok(Input::new()
                    .with_prompt("Enter issue title")
                    .interact_text()?)
            })
            .await?
        }
    };

    // Get description (optional)
    let issue_desc = match description {
        Some(d) => Some(d),
        None => {
            // In non-interactive mode, description is optional (None)
            if interactive::is_non_interactive() {
                None
            } else {
                let desc: String = raps_kernel::prompts::spawn_prompt(|| {
                    Ok(Input::new()
                        .with_prompt("Enter description (optional)")
                        .allow_empty(true)
                        .interact_text()?)
                })
                .await?;
                if desc.is_empty() { None } else { Some(desc) }
            }
        }
    };

    println!("{}", "Creating issue...".dimmed());

    let request = CreateIssueRequest {
        title: issue_title,
        description: issue_desc,
        status: "open".to_string(),
        issue_type_id: None,
        issue_subtype_id: None,
        assigned_to: None,
        assigned_to_type: None,
        due_date: None,
    };

    let issue = client
        .create_issue(project_id, request)
        .await
        .context("Failed to create issue. Verify your permissions on this project")?;

    println!("{} Issue created!", "✓".green().bold());
    println!("  {} {}", "ID:".bold(), issue.id);
    println!("  {} {}", "Title:".bold(), issue.title.cyan());
    println!("  {} {}", "Status:".bold(), issue.status);

    Ok(())
}

async fn create_issues_from_csv(
    client: &IssuesClient,
    project_id: &str,
    csv_path: &PathBuf,
    output_format: OutputFormat,
) -> Result<()> {
    // Parse CSV from file or stdin
    let csv_content = if csv_path.as_os_str() == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .lock()
            .read_to_string(&mut buf)
            .context("Failed to read CSV from stdin")?;
        buf
    } else {
        std::fs::read_to_string(csv_path)
            .with_context(|| format!("Failed to open CSV file: {}", csv_path.display()))?
    };
    let mut reader = csv::Reader::from_reader(csv_content.as_bytes());

    let mut rows: Vec<CsvIssueRow> = Vec::new();
    let mut validation_errors: Vec<String> = Vec::new();

    for (i, result) in reader.deserialize().enumerate() {
        match result {
            Ok(row) => {
                let row: CsvIssueRow = row;
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
            "\n{} Bulk issue creation: {} issues from {}",
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

        let status = row.status.as_deref().unwrap_or("open").to_string();

        let request = CreateIssueRequest {
            title: row.title.clone(),
            description: row.description.clone(),
            status,
            issue_type_id: None,
            issue_subtype_id: None,
            assigned_to: None,
            assigned_to_type: None,
            due_date: None,
        };

        match client.create_issue(project_id, request).await {
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
            println!("\n{}", "Bulk Issue Creation Results:".bold());
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
                    "\n{} All {} issue(s) created successfully!",
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

pub(super) async fn update_issue(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    status: Option<String>,
    title: Option<String>,
    _output_format: OutputFormat,
) -> Result<()> {
    // Get current issue
    let current = client
        .get_issue(project_id, issue_id)
        .await
        .context(format!(
            "Failed to get issue '{}'. Verify the issue ID exists",
            issue_id
        ))?;

    // Determine new status
    let new_status = match status {
        Some(s) => Some(s),
        None if title.is_none() => {
            if interactive::is_non_interactive() {
                anyhow::bail!(
                    "Status is required in non-interactive mode. Use --status <open|answered|closed>."
                );
            }
            // Prompt for status if no updates provided
            let new = raps_kernel::prompts::spawn_prompt(|| {
                let statuses = vec!["open", "answered", "closed"];
                let selection = Select::new()
                    .with_prompt("Select new status")
                    .items(&statuses)
                    .default(0)
                    .interact()?;
                Ok(statuses[selection].to_string())
            })
            .await?;
            Some(new)
        }
        None => None,
    };

    println!("{}", "Updating issue...".dimmed());

    let request = UpdateIssueRequest {
        title,
        description: None,
        status: new_status.clone(),
        assigned_to: None,
        due_date: None,
    };

    let issue = client
        .update_issue(project_id, issue_id, request)
        .await
        .context(format!(
            "Failed to update issue '{}'. Check permissions and that the status transition is valid",
            issue_id
        ))?;

    println!("{} Issue updated!", "✓".green().bold());
    println!("  {} {}", "Title:".bold(), issue.title);
    println!(
        "  {} {} → {}",
        "Status:".bold(),
        current.status.dimmed(),
        issue.status.cyan()
    );

    Ok(())
}

pub(super) async fn delete_issue(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Deleting issue...".dimmed());
    }

    client
        .delete_issue(project_id, issue_id)
        .await
        .context(format!(
            "Failed to delete issue '{}'. Verify the issue ID exists and you have delete permissions",
            issue_id
        ))?;

    #[derive(Serialize)]
    struct DeleteIssueOutput {
        success: bool,
        issue_id: String,
        message: String,
    }

    let output = DeleteIssueOutput {
        success: true,
        issue_id: issue_id.to_string(),
        message: "Issue deleted successfully".to_string(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} {}", "✓".green().bold(), output.message);
            println!("  {} {}", "ID:".bold(), issue_id.cyan());
        }
        _ => output_format.write(&output)?,
    }

    Ok(())
}

pub(super) async fn list_issue_types(
    client: &IssuesClient,
    project_id: &str,
    _output_format: OutputFormat,
) -> Result<()> {
    println!("{}", "Fetching issue types...".dimmed());

    let types = client.list_issue_types(project_id).await.context(format!(
        "Failed to list issue types for project '{}'",
        project_id
    ))?;

    if types.is_empty() {
        println!("{}", "No issue types found.".yellow());
        return Ok(());
    }

    println!("\n{}", "Issue Types (Categories):".bold());
    println!("{}", "─".repeat(60));

    for issue_type in types {
        let active = if issue_type.is_active.unwrap_or(true) {
            "".to_string()
        } else {
            " (inactive)".dimmed().to_string()
        };

        println!("  {} {}{}", "•".cyan(), issue_type.title.bold(), active);
        println!("    {} {}", "ID:".dimmed(), issue_type.id);

        if let Some(ref subtypes) = issue_type.subtypes {
            for subtype in subtypes {
                let sub_active = if subtype.is_active.unwrap_or(true) {
                    "".to_string()
                } else {
                    " (inactive)".dimmed().to_string()
                };
                println!("    {} {}{}", "└".dimmed(), subtype.title, sub_active);
            }
        }
    }

    println!("{}", "─".repeat(60));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_issue_row_deserialization() {
        let csv_data = "title,description,status\nBroken pipe,Water leak in B2,open\n";
        let mut rdr = csv::ReaderBuilder::new().from_reader(csv_data.as_bytes());
        let row: CsvIssueRow = rdr.deserialize().next().unwrap().unwrap();
        assert_eq!(row.title, "Broken pipe");
        assert_eq!(row.description.unwrap(), "Water leak in B2");
        assert_eq!(row.status.unwrap(), "open");
    }

    #[test]
    fn test_csv_issue_row_minimal() {
        let csv_data = "title,description,status\nBroken pipe,,\n";
        let mut rdr = csv::ReaderBuilder::new().from_reader(csv_data.as_bytes());
        let row: CsvIssueRow = rdr.deserialize().next().unwrap().unwrap();
        assert_eq!(row.title, "Broken pipe");
        // Optional fields may be None or empty string depending on csv crate behavior
        assert!(
            row.description.is_none() || row.description.as_deref() == Some(""),
            "Expected None or empty for description, got {:?}",
            row.description
        );
    }

    #[test]
    fn test_csv_issue_row_multiple_rows() {
        let csv_data = "\
title,description,status
Broken pipe,Water leak in B2,open
Missing rebar,Check section C,closed
Crack in wall,,open
";
        let mut rdr = csv::ReaderBuilder::new().from_reader(csv_data.as_bytes());
        let rows: Vec<CsvIssueRow> = rdr.deserialize().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].title, "Broken pipe");
        assert_eq!(rows[1].title, "Missing rebar");
        assert_eq!(rows[1].status.as_deref(), Some("closed"));
        assert_eq!(rows[2].title, "Crack in wall");
    }
}

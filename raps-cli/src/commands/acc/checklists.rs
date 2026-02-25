// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Checklist CRUD command handlers

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_acc::{AccClient, CreateChecklistRequest, UpdateChecklistRequest};

use super::truncate_str;

#[derive(Serialize)]
struct ChecklistOutput {
    id: String,
    title: String,
    status: String,
    location: Option<String>,
    due_date: Option<String>,
}

#[derive(Serialize)]
struct TemplateOutput {
    id: String,
    title: String,
    description: Option<String>,
}

pub(super) async fn list_checklists(
    client: &AccClient,
    project_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching checklists...".dimmed());
    }

    let checklists = client.list_checklists(project_id).await.context(format!(
        "Failed to list checklists for project '{}'",
        project_id
    ))?;

    let outputs: Vec<ChecklistOutput> = checklists
        .iter()
        .map(|c| ChecklistOutput {
            id: c.id.clone(),
            title: c.title.clone(),
            status: c.status.clone(),
            location: c.location.clone(),
            due_date: c.due_date.clone(),
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No checklists found.".yellow()),
            _ => output_format.write(&Vec::<ChecklistOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Checklists:".bold());
            println!("{}", "─".repeat(90));
            println!(
                "{:<45} {:<15} {:<20} {}",
                "Title".bold(),
                "Status".bold(),
                "Location".bold(),
                "Due Date".bold()
            );
            println!("{}", "─".repeat(90));

            for checklist in &outputs {
                let location = checklist.location.as_deref().unwrap_or("-");
                let due = checklist.due_date.as_deref().unwrap_or("-");
                let status_color = match checklist.status.to_lowercase().as_str() {
                    "complete" | "completed" => checklist.status.green().to_string(),
                    "open" | "in_progress" => checklist.status.yellow().to_string(),
                    _ => checklist.status.clone(),
                };
                println!(
                    "{:<45} {:<15} {:<20} {}",
                    truncate_str(&checklist.title, 45).cyan(),
                    status_color,
                    truncate_str(location, 20),
                    due.dimmed()
                );
            }

            println!("{}", "─".repeat(90));
            println!("{} {} checklist(s) found", "→".cyan(), outputs.len());
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

pub(super) async fn get_checklist(
    client: &AccClient,
    project_id: &str,
    checklist_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching checklist details...".dimmed());
    }

    let checklist = client
        .get_checklist(project_id, checklist_id)
        .await
        .context(format!(
            "Failed to get checklist '{}'. Verify the checklist ID exists",
            checklist_id
        ))?;

    let output = ChecklistOutput {
        id: checklist.id.clone(),
        title: checklist.title.clone(),
        status: checklist.status.clone(),
        location: checklist.location.clone(),
        due_date: checklist.due_date.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Checklist Details:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "ID:".bold(), checklist.id.cyan());
            println!("{:<15} {}", "Title:".bold(), checklist.title);
            println!("{:<15} {}", "Status:".bold(), checklist.status);
            println!(
                "{:<15} {}",
                "Location:".bold(),
                checklist.location.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Due Date:".bold(),
                checklist.due_date.as_deref().unwrap_or("-")
            );
            println!("{}", "─".repeat(60));
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn create_checklist(
    client: &AccClient,
    project_id: &str,
    title: &str,
    template_id: Option<String>,
    location: Option<String>,
    due_date: Option<String>,
    assignee_id: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Creating checklist...".dimmed());
    }

    let request = CreateChecklistRequest {
        title: title.to_string(),
        template_id,
        location,
        due_date,
        assignee_id,
    };

    let checklist = client
        .create_checklist(project_id, request)
        .await
        .context("Failed to create checklist. Verify your permissions on this project")?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Checklist created successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), checklist.id.cyan());
            println!("{:<15} {}", "Title:".bold(), checklist.title);
        }
        _ => {
            output_format.write(&serde_json::json!({
                "id": checklist.id,
                "title": checklist.title,
                "created": true
            }))?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn update_checklist(
    client: &AccClient,
    project_id: &str,
    checklist_id: &str,
    title: Option<String>,
    status: Option<String>,
    location: Option<String>,
    due_date: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Updating checklist...".dimmed());
    }

    let request = UpdateChecklistRequest {
        title,
        status,
        location,
        due_date,
        assignee_id: None,
    };

    let checklist = client
        .update_checklist(project_id, checklist_id, request)
        .await
        .context(format!(
            "Failed to update checklist '{}'. Check permissions",
            checklist_id
        ))?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Checklist updated successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), checklist.id.cyan());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "id": checklist.id,
                "updated": true
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn list_templates(
    client: &AccClient,
    project_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching checklist templates...".dimmed());
    }

    let templates = client
        .list_checklist_templates(project_id)
        .await
        .context(format!(
            "Failed to list checklist templates for project '{}'",
            project_id
        ))?;

    let outputs: Vec<TemplateOutput> = templates
        .iter()
        .map(|t| TemplateOutput {
            id: t.id.clone(),
            title: t.title.clone(),
            description: t.description.clone(),
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No checklist templates found.".yellow()),
            _ => output_format.write(&Vec::<TemplateOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Checklist Templates:".bold());
            println!("{}", "─".repeat(80));
            println!("{:<40} {}", "Title".bold(), "Description".bold());
            println!("{}", "─".repeat(80));

            for template in &outputs {
                let desc = template.description.as_deref().unwrap_or("-");
                println!(
                    "{:<40} {}",
                    template.title.cyan(),
                    truncate_str(desc, 40).dimmed()
                );
            }

            println!("{}", "─".repeat(80));
            println!("{} {} template(s) found", "→".cyan(), outputs.len());
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

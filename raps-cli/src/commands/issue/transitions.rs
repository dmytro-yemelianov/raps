// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Issue state transition operations

use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::Select;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_acc::{IssuesClient, UpdateIssueRequest};
use raps_kernel::interactive;

/// Allowed issue status transitions
const STATUS_TRANSITIONS: &[(&str, &[&str])] = &[
    ("open", &["answered", "closed"]),
    ("answered", &["open", "closed"]),
    ("closed", &["open"]),
    ("draft", &["open"]),
];

fn get_allowed_transitions(current_status: &str) -> Vec<&'static str> {
    for (status, transitions) in STATUS_TRANSITIONS {
        if *status == current_status.to_lowercase() {
            return transitions.to_vec();
        }
    }
    // Default: allow any common transitions
    vec!["open", "answered", "closed"]
}

pub(super) async fn transition_issue(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    target_status: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    // Get current issue to determine valid transitions
    let current_issue = client
        .get_issue(project_id, issue_id)
        .await
        .context(format!("Failed to get issue '{}' for transition", issue_id))?;
    let current_status = current_issue.status.clone();
    let allowed = get_allowed_transitions(&current_status);

    // Get target status
    let new_status = match target_status {
        Some(s) => {
            let s_lower = s.to_lowercase();
            if !allowed.contains(&s_lower.as_str()) {
                anyhow::bail!(
                    "Cannot transition from '{}' to '{}'. Allowed transitions: {:?}",
                    current_status,
                    s,
                    allowed
                );
            }
            s_lower
        }
        None => {
            // In non-interactive mode, require the target status
            if interactive::is_non_interactive() {
                anyhow::bail!(
                    "Target status is required. Current: '{}'. Allowed: {:?}",
                    current_status,
                    allowed
                );
            }

            // Interactive: show allowed transitions
            println!("{} Current status: {}", "→".cyan(), current_status.bold());

            let selection = Select::new()
                .with_prompt("Select new status")
                .items(&allowed)
                .interact()?;

            allowed[selection].to_string()
        }
    };

    if output_format.supports_colors() {
        println!("{}", "Transitioning issue...".dimmed());
    }

    let request = UpdateIssueRequest {
        title: None,
        description: None,
        status: Some(new_status.clone()),
        assigned_to: None,
        due_date: None,
    };

    let updated_issue = client
        .update_issue(project_id, issue_id, request)
        .await
        .context(format!(
            "Failed to transition issue '{}' to '{}'",
            issue_id, new_status
        ))?;

    #[derive(Serialize)]
    struct TransitionOutput {
        success: bool,
        issue_id: String,
        from_status: String,
        to_status: String,
    }

    let output = TransitionOutput {
        success: true,
        issue_id: updated_issue.id.clone(),
        from_status: current_status.clone(),
        to_status: updated_issue.status.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Issue transitioned!", "✓".green().bold());
            println!(
                "  {} {} → {}",
                "Status:".bold(),
                output.from_status.dimmed(),
                output.to_status.cyan()
            );
        }
        _ => output_format.write(&output)?,
    }

    Ok(())
}

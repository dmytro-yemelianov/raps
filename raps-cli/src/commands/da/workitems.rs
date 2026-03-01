// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! WorkItem handlers for Design Automation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_da::{DesignAutomationClient, WorkItemArgument};
use raps_kernel::progress;

pub(super) async fn list_workitems(
    client: &DesignAutomationClient,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching workitems...".dimmed());
    }

    let workitems = client.list_workitems().await?;

    #[derive(Serialize, schemars::JsonSchema)]
    struct WorkitemOutput {
        id: String,
        status: String,
        progress: String,
    }

    let workitem_outputs: Vec<WorkitemOutput> = workitems
        .iter()
        .map(|w| WorkitemOutput {
            id: w.id.clone(),
            status: w.status.clone(),
            progress: w.progress.clone().unwrap_or_default(),
        })
        .collect();

    if workitem_outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No workitems found.".yellow()),
            _ => {
                output_format.write(&Vec::<WorkitemOutput>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Workitems:".bold());
            println!(
                "{:<40} {:<15} {}",
                "ID".bold(),
                "Status".bold(),
                "Progress".bold()
            );
            println!("{}", "-".repeat(70));

            for item in &workitem_outputs {
                let status_colored = match item.status.as_str() {
                    "success" => item.status.green().to_string(),
                    "failed" | "cancelled" => item.status.red().to_string(),
                    "inprogress" | "pending" => item.status.yellow().to_string(),
                    _ => item.status.clone(),
                };
                println!("{:<40} {:<15} {}", item.id, status_colored, item.progress);
            }

            println!("{}", "-".repeat(70));
        }
        _ => {
            output_format.write(&workitem_outputs)?;
        }
    }
    Ok(())
}

pub(super) async fn run_workitem(
    client: &DesignAutomationClient,
    activity_id: &str,
    inputs: Vec<(String, String)>,
    outputs: Vec<(String, String)>,
    wait: bool,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Creating work item...".dimmed());
        println!("  {} {}", "Activity:".bold(), activity_id.cyan());
    }

    let mut arguments: HashMap<String, WorkItemArgument> = HashMap::new();

    // Process input arguments
    for (name, value) in inputs {
        let (url, verb) = if value.starts_with("@") {
            // Local file - would need OSS upload (simplified for now)
            anyhow::bail!(
                "Local file inputs (starting with @) require OSS upload. Please upload to OSS first and provide the signed URL."
            );
        } else {
            // Assume it's a URL
            (value, Some("get".to_string()))
        };

        arguments.insert(
            name,
            WorkItemArgument {
                url,
                verb,
                headers: None,
            },
        );
    }

    // Process output arguments
    for (name, value) in outputs {
        // For outputs, the value is typically a signed URL for PUT
        arguments.insert(
            name,
            WorkItemArgument {
                url: value,
                verb: Some("put".to_string()),
                headers: None,
            },
        );
    }

    // Qualify bare activity ID with nickname.name+default
    let qualified_activity = if activity_id.contains('.') || activity_id.contains('+') {
        activity_id.to_string()
    } else {
        let nickname = client.effective_nickname().await?;
        format!("{nickname}.{activity_id}+default")
    };

    let workitem = client
        .create_workitem(&qualified_activity, arguments)
        .await?;

    #[derive(Serialize, schemars::JsonSchema)]
    struct RunOutput {
        success: bool,
        workitem_id: String,
        status: String,
    }

    let output = RunOutput {
        success: true,
        workitem_id: workitem.id.clone(),
        status: workitem.status.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Work item submitted!", "\u{2713}".green().bold());
            println!("  {} {}", "Work Item ID:".bold(), output.workitem_id.cyan());
            println!("  {} {}", "Status:".bold(), output.status);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    // If wait mode, poll for completion
    if wait {
        println!();
        check_status(client, &workitem.id, true, false, None, output_format).await?;
    }

    Ok(())
}

pub(super) async fn check_status(
    client: &DesignAutomationClient,
    workitem_id: &str,
    wait: bool,
    _download: bool,
    _output_dir: Option<PathBuf>,
    _output_format: OutputFormat,
) -> Result<()> {
    if wait {
        // Spinner hidden in non-interactive mode
        let spinner = progress::spinner("Checking work item status...");
        let timeout = Duration::from_secs(1800); // 30-minute timeout
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                spinner.finish_with_message(format!(
                    "{} Timed out after {} seconds waiting for work item",
                    "X".red().bold(),
                    timeout.as_secs()
                ));
                break;
            }

            let workitem = client.get_workitem_status(workitem_id).await?;
            let progress = workitem.progress.as_deref().unwrap_or("");
            spinner.set_message(format!("Status: {} {}", workitem.status, progress));

            match workitem.status.as_str() {
                "success" => {
                    spinner.finish_with_message(format!(
                        "{} Work item completed successfully!",
                        "\u{2713}".green().bold()
                    ));
                    if let Some(url) = workitem.report_url {
                        println!("  {} {}", "Report:".bold(), url);
                    }
                    break;
                }
                "failed" | "cancelled" | "failedLimitDataSize" | "failedLimitProcessingTime" => {
                    spinner.finish_with_message(format!(
                        "{} Work item failed: {}",
                        "X".red().bold(),
                        workitem.status
                    ));
                    if let Some(url) = workitem.report_url {
                        println!("  {} {}", "Report:".bold(), url);
                    }
                    break;
                }
                _ => {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    } else {
        let workitem = client.get_workitem_status(workitem_id).await?;

        let status_icon = match workitem.status.as_str() {
            "success" => "\u{2713}".green().bold(),
            "failed" | "cancelled" => "X".red().bold(),
            "inprogress" | "pending" => "...".yellow().bold(),
            _ => "?".dimmed(),
        };

        println!("{} {}", status_icon, workitem.status);

        if let Some(progress) = workitem.progress {
            println!("  {} {}", "Progress:".bold(), progress);
        }

        if let Some(url) = workitem.report_url {
            println!("  {} {}", "Report:".bold(), url);
        }
    }

    Ok(())
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Activity handlers for Design Automation.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_da::{ActivityParameter, CreateActivityRequest, DesignAutomationClient};

use super::ActivityDefinition;

pub(super) async fn list_activities(
    client: &DesignAutomationClient,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching activities...".dimmed());
    }

    let activities = client.list_activities().await?;

    if activities.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No activities found.".yellow()),
            _ => {
                output_format.write(&Vec::<String>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Activities:".bold());
            println!("{}", "-".repeat(60));

            for activity in &activities {
                println!("  {} {}", "-".cyan(), activity);
            }

            println!("{}", "-".repeat(60));
        }
        _ => {
            output_format.write(&activities)?;
        }
    }
    Ok(())
}

pub(super) async fn delete_activity(
    client: &DesignAutomationClient,
    id: &str,
    _output_format: OutputFormat,
) -> Result<()> {
    println!("{}", "Deleting activity...".dimmed());

    client.delete_activity(id).await?;

    println!(
        "{} Activity '{}' deleted!",
        "\u{2713}".green().bold(),
        id
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn create_activity(
    client: &DesignAutomationClient,
    file: Option<PathBuf>,
    id: Option<String>,
    engine: Option<String>,
    appbundle: Option<String>,
    command: Option<String>,
    description: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let activity_def = if let Some(file_path) = file {
        load_activity_from_file(&file_path)?
    } else {
        build_activity_from_args(id, engine, appbundle, command, description)?
    };

    if output_format.supports_colors() {
        println!("{}", "Creating activity...".dimmed());
        println!("  {} {}", "ID:".bold(), activity_def.id);
        println!("  {} {}", "Engine:".bold(), activity_def.engine);
    }

    // Convert to API request
    let parameters: HashMap<String, ActivityParameter> = activity_def
        .parameters
        .into_iter()
        .map(|(name, param)| {
            (
                name,
                ActivityParameter {
                    verb: param.verb,
                    local_name: param.local_name,
                    description: param.description,
                    required: param.required,
                    zip: param.zip,
                },
            )
        })
        .collect();

    // Qualify bare appbundle names with nickname.name+default
    let nickname = client.effective_nickname().await?;
    let qualified_bundles: Vec<String> = activity_def
        .app_bundles
        .into_iter()
        .map(|b| {
            if b.contains('.') || b.contains('+') {
                b // Already qualified
            } else {
                format!("{nickname}.{b}+default")
            }
        })
        .collect();

    let request = CreateActivityRequest {
        id: activity_def.id.clone(),
        engine: activity_def.engine,
        command_line: activity_def.command_line,
        app_bundles: qualified_bundles,
        parameters,
        description: activity_def.description,
    };

    let activity = client.create_activity(request).await?;

    // Auto-create a "default" alias so the activity can be referenced
    if let Some(version) = activity.version {
        match client
            .create_activity_alias(&activity_def.id, "default", version)
            .await
        {
            Ok(()) => {
                if output_format.supports_colors() {
                    println!(
                        "  {} Alias '{}' created",
                        "\u{2713}".green(),
                        "default".cyan()
                    );
                }
            }
            Err(e) => {
                if output_format.supports_colors() {
                    println!("  {} Could not create alias: {}", "!".yellow(), e);
                }
            }
        }
    }

    #[derive(Serialize)]
    struct CreateActivityOutput {
        success: bool,
        id: String,
        engine: String,
        version: Option<i32>,
    }

    let output = CreateActivityOutput {
        success: true,
        id: activity.id.clone(),
        engine: activity.engine.clone(),
        version: activity.version,
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Activity created!", "\u{2713}".green().bold());
            println!("  {} {}", "ID:".bold(), output.id);
            println!("  {} {}", "Engine:".bold(), output.engine.cyan());
            if let Some(v) = output.version {
                println!("  {} {}", "Version:".bold(), v);
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

/// Load an activity definition from a JSON or YAML file (or stdin via `-`).
fn load_activity_from_file(file_path: &PathBuf) -> Result<ActivityDefinition> {
    let content = if file_path.as_os_str() == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .lock()
            .read_to_string(&mut buf)
            .context("Failed to read activity definition from stdin")?;
        buf
    } else {
        std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read activity file: {}", file_path.display()))?
    };

    let is_yaml = file_path.as_os_str() == "-"
        || file_path
            .extension()
            .map(|e| e == "yaml" || e == "yml")
            .unwrap_or(false);

    let def: ActivityDefinition = if is_yaml {
        serde_yaml::from_str(&content).with_context(|| {
            format!(
                "Failed to parse YAML activity file: {}",
                file_path.display()
            )
        })?
    } else {
        serde_json::from_str(&content).with_context(|| {
            format!(
                "Failed to parse JSON activity file: {}",
                file_path.display()
            )
        })?
    };

    // Validate required fields
    if def.id.is_empty() {
        anyhow::bail!("Activity definition must have an 'id' field");
    }
    if def.engine.is_empty() {
        anyhow::bail!("Activity definition must have an 'engine' field");
    }
    if def.command_line.is_empty() {
        anyhow::bail!("Activity definition must have a 'commandLine' field");
    }

    Ok(def)
}

/// Build an activity definition from individual CLI arguments.
fn build_activity_from_args(
    id: Option<String>,
    engine: Option<String>,
    appbundle: Option<String>,
    command: Option<String>,
    description: Option<String>,
) -> Result<ActivityDefinition> {
    let activity_id =
        id.ok_or_else(|| anyhow::anyhow!("--id is required when not using --file"))?;
    let activity_engine =
        engine.ok_or_else(|| anyhow::anyhow!("--engine is required when not using --file"))?;
    let activity_command =
        command.ok_or_else(|| anyhow::anyhow!("--command is required when not using --file"))?;

    let app_bundles = match appbundle {
        Some(bundle) => vec![bundle],
        None => Vec::new(),
    };

    Ok(ActivityDefinition {
        id: activity_id,
        engine: activity_engine,
        command_line: vec![activity_command],
        app_bundles,
        parameters: HashMap::new(),
        description,
    })
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Design Automation commands
//!
//! Commands for managing engines, app bundles, activities, and work items.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::api::DesignAutomationClient;
use crate::api::design_automation::{ActivityParameter, CreateActivityRequest};
use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum DaCommands {
    /// List available engines
    Engines,

    /// List app bundles
    Appbundles,

    /// Create an app bundle
    #[command(name = "appbundle-create")]
    AppbundleCreate {
        /// App bundle ID
        #[arg(short, long)]
        id: Option<String>,

        /// Engine ID (e.g., Autodesk.AutoCAD+24)
        #[arg(short, long)]
        engine: Option<String>,

        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Delete an app bundle
    #[command(name = "appbundle-delete")]
    AppbundleDelete {
        /// App bundle ID to delete
        id: String,
    },

    /// List activities
    Activities,

    /// Create an activity from JSON or YAML definition
    #[command(name = "activity-create")]
    ActivityCreate {
        /// Path to JSON or YAML activity definition file
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Activity ID (required if not using --file)
        #[arg(long)]
        id: Option<String>,

        /// Engine ID (required if not using --file)
        #[arg(long)]
        engine: Option<String>,

        /// App bundle ID to use (required if not using --file)
        #[arg(long)]
        appbundle: Option<String>,

        /// Command line (required if not using --file)
        #[arg(long)]
        command: Option<String>,

        /// Description
        #[arg(long)]
        description: Option<String>,
    },

    /// Delete an activity
    #[command(name = "activity-delete")]
    ActivityDelete {
        /// Activity ID to delete
        id: String,
    },

    /// Submit a work item to run an activity
    #[command(name = "run")]
    Run {
        /// Activity ID (fully qualified, e.g., owner.activity+alias)
        activity: String,

        /// Input arguments as key=value pairs (use @file.dwg for file inputs)
        #[arg(short, long, value_parser = parse_argument)]
        input: Vec<(String, String)>,

        /// Output arguments as key=value pairs (local file paths)
        #[arg(short, long, value_parser = parse_argument)]
        output: Vec<(String, String)>,

        /// Wait for completion and download results
        #[arg(short, long)]
        wait: bool,
    },

    /// Check work item status
    Status {
        /// Work item ID
        workitem_id: String,

        /// Wait for completion
        #[arg(short, long)]
        wait: bool,

        /// Download outputs on completion
        #[arg(short, long)]
        download: bool,

        /// Output directory for downloads
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
}

/// Parse key=value argument pairs
fn parse_argument(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid argument format '{}'. Use key=value", s));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Activity definition structure for JSON/YAML parsing
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityDefinition {
    id: String,
    engine: String,
    #[serde(default)]
    command_line: Vec<String>,
    #[serde(default)]
    app_bundles: Vec<String>,
    #[serde(default)]
    parameters: HashMap<String, ParameterDefinition>,
    description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParameterDefinition {
    verb: String,
    local_name: Option<String>,
    description: Option<String>,
    required: Option<bool>,
    zip: Option<bool>,
}

impl DaCommands {
    pub async fn execute(
        self,
        client: &DesignAutomationClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            DaCommands::Engines => list_engines(client, output_format).await,
            DaCommands::Appbundles => list_appbundles(client, output_format).await,
            DaCommands::AppbundleCreate {
                id,
                engine,
                description,
            } => create_appbundle(client, id, engine, description, output_format).await,
            DaCommands::AppbundleDelete { id } => {
                delete_appbundle(client, &id, output_format).await
            }
            DaCommands::Activities => list_activities(client, output_format).await,
            DaCommands::ActivityCreate {
                file,
                id,
                engine,
                appbundle,
                command,
                description,
            } => {
                create_activity(
                    client,
                    file,
                    id,
                    engine,
                    appbundle,
                    command,
                    description,
                    output_format,
                )
                .await
            }
            DaCommands::ActivityDelete { id } => delete_activity(client, &id, output_format).await,
            DaCommands::Run {
                activity,
                input,
                output,
                wait,
            } => run_workitem(client, &activity, input, output, wait, output_format).await,
            DaCommands::Status {
                workitem_id,
                wait,
                download,
                output_dir,
            } => {
                check_status(
                    client,
                    &workitem_id,
                    wait,
                    download,
                    output_dir,
                    output_format,
                )
                .await
            }
        }
    }
}

async fn list_engines(client: &DesignAutomationClient, output_format: OutputFormat) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching engines...".dimmed());
    }

    let engines = client.list_engines().await?;

    #[derive(Serialize)]
    struct EngineOutput {
        id: String,
        description: Option<String>,
        product_version: Option<String>,
    }

    let engine_outputs: Vec<EngineOutput> = engines
        .iter()
        .map(|e| EngineOutput {
            id: e.id.clone(),
            description: e.description.clone(),
            product_version: e.product_version.clone(),
        })
        .collect();

    if engine_outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No engines found.".yellow()),
            _ => {
                output_format.write(&Vec::<EngineOutput>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Available Engines:".bold());
            println!("{}", "─".repeat(80));

            // Group by product
            let mut autocad_engines = Vec::new();
            let mut revit_engines = Vec::new();
            let mut inventor_engines = Vec::new();
            let mut max_engines = Vec::new();
            let mut other_engines = Vec::new();

            for engine in &engines {
                if engine.id.contains("AutoCAD") {
                    autocad_engines.push(engine);
                } else if engine.id.contains("Revit") {
                    revit_engines.push(engine);
                } else if engine.id.contains("Inventor") {
                    inventor_engines.push(engine);
                } else if engine.id.contains("3dsMax") {
                    max_engines.push(engine);
                } else {
                    other_engines.push(engine);
                }
            }

            if !autocad_engines.is_empty() {
                println!("\n{}", "AutoCAD:".cyan().bold());
                for engine in autocad_engines {
                    println!("  {} {}", "•".dimmed(), engine.id);
                }
            }

            if !revit_engines.is_empty() {
                println!("\n{}", "Revit:".cyan().bold());
                for engine in revit_engines {
                    println!("  {} {}", "•".dimmed(), engine.id);
                }
            }

            if !inventor_engines.is_empty() {
                println!("\n{}", "Inventor:".cyan().bold());
                for engine in inventor_engines {
                    println!("  {} {}", "•".dimmed(), engine.id);
                }
            }

            if !max_engines.is_empty() {
                println!("\n{}", "3ds Max:".cyan().bold());
                for engine in max_engines {
                    println!("  {} {}", "•".dimmed(), engine.id);
                }
            }

            if !other_engines.is_empty() {
                println!("\n{}", "Other:".cyan().bold());
                for engine in other_engines {
                    println!("  {} {}", "•".dimmed(), engine.id);
                }
            }

            println!("{}", "─".repeat(80));
        }
        _ => {
            output_format.write(&engine_outputs)?;
        }
    }
    Ok(())
}

async fn list_appbundles(
    client: &DesignAutomationClient,
    _output_format: OutputFormat,
) -> Result<()> {
    println!("{}", "Fetching app bundles...".dimmed());

    let appbundles = client.list_appbundles().await?;

    if appbundles.is_empty() {
        println!("{}", "No app bundles found.".yellow());
        return Ok(());
    }

    println!("\n{}", "App Bundles:".bold());
    println!("{}", "─".repeat(60));

    for bundle in appbundles {
        println!("  {} {}", "•".cyan(), bundle);
    }

    println!("{}", "─".repeat(60));
    Ok(())
}

async fn create_appbundle(
    client: &DesignAutomationClient,
    id: Option<String>,
    engine: Option<String>,
    description: Option<String>,
    _output_format: OutputFormat,
) -> Result<()> {
    // Get engine first to help with ID suggestion
    let selected_engine = match engine {
        Some(e) => e,
        None => {
            println!("{}", "Fetching engines...".dimmed());
            let engines = client.list_engines().await?;

            let engine_ids: Vec<&str> = engines.iter().map(|e| e.id.as_str()).collect();

            let selection = Select::new()
                .with_prompt("Select engine")
                .items(&engine_ids)
                .interact()?;

            engines[selection].id.clone()
        }
    };

    // Get bundle ID
    let bundle_id = match id {
        Some(i) => i,
        None => Input::new()
            .with_prompt("Enter app bundle ID")
            .interact_text()?,
    };

    println!("{}", "Creating app bundle...".dimmed());

    let bundle = client
        .create_appbundle(&bundle_id, &selected_engine, description.as_deref())
        .await?;

    println!("{} App bundle created!", "✓".green().bold());
    println!("  {} {}", "ID:".bold(), bundle.id);
    println!("  {} {}", "Engine:".bold(), bundle.engine.cyan());
    println!("  {} {}", "Version:".bold(), bundle.version);

    if let Some(upload) = bundle.upload_parameters {
        println!("\n{}", "Upload your bundle ZIP to:".yellow());
        println!("  {}", upload.endpoint_url);
    }

    Ok(())
}

async fn delete_appbundle(
    client: &DesignAutomationClient,
    id: &str,
    _output_format: OutputFormat,
) -> Result<()> {
    println!("{}", "Deleting app bundle...".dimmed());

    client.delete_appbundle(id).await?;

    println!("{} App bundle '{}' deleted!", "✓".green().bold(), id);
    Ok(())
}

async fn list_activities(
    client: &DesignAutomationClient,
    _output_format: OutputFormat,
) -> Result<()> {
    println!("{}", "Fetching activities...".dimmed());

    let activities = client.list_activities().await?;

    if activities.is_empty() {
        println!("{}", "No activities found.".yellow());
        return Ok(());
    }

    println!("\n{}", "Activities:".bold());
    println!("{}", "─".repeat(60));

    for activity in activities {
        println!("  {} {}", "•".cyan(), activity);
    }

    println!("{}", "─".repeat(60));
    Ok(())
}

async fn delete_activity(
    client: &DesignAutomationClient,
    id: &str,
    _output_format: OutputFormat,
) -> Result<()> {
    println!("{}", "Deleting activity...".dimmed());

    client.delete_activity(id).await?;

    println!("{} Activity '{}' deleted!", "✓".green().bold(), id);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create_activity(
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
        // Load activity definition from file
        let content = std::fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read activity file: {}", file_path.display()))?;

        let def: ActivityDefinition = if file_path
            .extension()
            .map(|e| e == "yaml" || e == "yml")
            .unwrap_or(false)
        {
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

        def
    } else {
        // Build activity from command line arguments
        let activity_id =
            id.ok_or_else(|| anyhow::anyhow!("--id is required when not using --file"))?;
        let activity_engine =
            engine.ok_or_else(|| anyhow::anyhow!("--engine is required when not using --file"))?;
        let activity_command = command
            .ok_or_else(|| anyhow::anyhow!("--command is required when not using --file"))?;

        let app_bundles = if let Some(bundle) = appbundle {
            vec![bundle]
        } else {
            Vec::new()
        };

        ActivityDefinition {
            id: activity_id,
            engine: activity_engine,
            command_line: vec![activity_command],
            app_bundles,
            parameters: HashMap::new(),
            description,
        }
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

    let request = CreateActivityRequest {
        id: activity_def.id.clone(),
        engine: activity_def.engine,
        command_line: activity_def.command_line,
        app_bundles: activity_def.app_bundles,
        parameters,
        description: activity_def.description,
    };

    let activity = client.create_activity(request).await?;

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
            println!("{} Activity created!", "✓".green().bold());
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

async fn run_workitem(
    client: &DesignAutomationClient,
    activity_id: &str,
    inputs: Vec<(String, String)>,
    outputs: Vec<(String, String)>,
    wait: bool,
    output_format: OutputFormat,
) -> Result<()> {
    use crate::api::design_automation::WorkItemArgument;

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

    let workitem = client.create_workitem(activity_id, arguments).await?;

    #[derive(Serialize)]
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
            println!("{} Work item submitted!", "✓".green().bold());
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

async fn check_status(
    client: &DesignAutomationClient,
    workitem_id: &str,
    wait: bool,
    _download: bool,
    _output_dir: Option<PathBuf>,
    _output_format: OutputFormat,
) -> Result<()> {
    if wait {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.enable_steady_tick(Duration::from_millis(100));

        loop {
            let workitem = client.get_workitem_status(workitem_id).await?;
            let progress = workitem.progress.as_deref().unwrap_or("");
            spinner.set_message(format!("Status: {} {}", workitem.status, progress));

            match workitem.status.as_str() {
                "success" => {
                    spinner.finish_with_message(format!(
                        "{} Work item completed successfully!",
                        "✓".green().bold()
                    ));
                    if let Some(url) = workitem.report_url {
                        println!("  {} {}", "Report:".bold(), url);
                    }
                    break;
                }
                "failed" | "cancelled" | "failedLimitDataSize" | "failedLimitProcessingTime" => {
                    spinner.finish_with_message(format!(
                        "{} Work item failed: {}",
                        "✗".red().bold(),
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
            "success" => "✓".green().bold(),
            "failed" | "cancelled" => "✗".red().bold(),
            "inprogress" | "pending" => "⋯".yellow().bold(),
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

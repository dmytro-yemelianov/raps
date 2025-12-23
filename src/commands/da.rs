//! Design Automation commands
//! 
//! Commands for managing engines, app bundles, activities, and work items.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

use crate::api::DesignAutomationClient;

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
    
    /// Delete an activity
    #[command(name = "activity-delete")]
    ActivityDelete {
        /// Activity ID to delete
        id: String,
    },
    
    /// Check work item status
    Status {
        /// Work item ID
        workitem_id: String,
        
        /// Wait for completion
        #[arg(short, long)]
        wait: bool,
    },
}

impl DaCommands {
    pub async fn execute(self, client: &DesignAutomationClient) -> Result<()> {
        match self {
            DaCommands::Engines => list_engines(client).await,
            DaCommands::Appbundles => list_appbundles(client).await,
            DaCommands::AppbundleCreate { id, engine, description } => {
                create_appbundle(client, id, engine, description).await
            }
            DaCommands::AppbundleDelete { id } => delete_appbundle(client, &id).await,
            DaCommands::Activities => list_activities(client).await,
            DaCommands::ActivityDelete { id } => delete_activity(client, &id).await,
            DaCommands::Status { workitem_id, wait } => {
                check_status(client, &workitem_id, wait).await
            }
        }
    }
}

async fn list_engines(client: &DesignAutomationClient) -> Result<()> {
    println!("{}", "Fetching engines...".dimmed());

    let engines = client.list_engines().await?;

    if engines.is_empty() {
        println!("{}", "No engines found.".yellow());
        return Ok(());
    }

    println!("\n{}", "Available Engines:".bold());
    println!("{}", "─".repeat(80));

    // Group by product
    let mut autocad_engines = Vec::new();
    let mut revit_engines = Vec::new();
    let mut inventor_engines = Vec::new();
    let mut max_engines = Vec::new();
    let mut other_engines = Vec::new();

    for engine in engines {
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
    Ok(())
}

async fn list_appbundles(client: &DesignAutomationClient) -> Result<()> {
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
        None => {
            Input::new()
                .with_prompt("Enter app bundle ID")
                .interact_text()?
        }
    };

    println!("{}", "Creating app bundle...".dimmed());

    let bundle = client.create_appbundle(&bundle_id, &selected_engine, description.as_deref()).await?;

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

async fn delete_appbundle(client: &DesignAutomationClient, id: &str) -> Result<()> {
    println!("{}", "Deleting app bundle...".dimmed());
    
    client.delete_appbundle(id).await?;

    println!("{} App bundle '{}' deleted!", "✓".green().bold(), id);
    Ok(())
}

async fn list_activities(client: &DesignAutomationClient) -> Result<()> {
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

async fn delete_activity(client: &DesignAutomationClient, id: &str) -> Result<()> {
    println!("{}", "Deleting activity...".dimmed());
    
    client.delete_activity(id).await?;

    println!("{} Activity '{}' deleted!", "✓".green().bold(), id);
    Ok(())
}

async fn check_status(client: &DesignAutomationClient, workitem_id: &str, wait: bool) -> Result<()> {
    if wait {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap()
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


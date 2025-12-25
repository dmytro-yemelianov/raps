//! ACC Extended Commands
//!
//! Commands for ACC modules: Assets, Submittals, Checklists

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use crate::api::AccClient;
use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum AccCommands {
    /// Manage project assets
    #[command(subcommand)]
    Asset(AssetCommands),

    /// Manage project submittals
    #[command(subcommand)]
    Submittal(SubmittalCommands),

    /// Manage project checklists
    #[command(subcommand)]
    Checklist(ChecklistCommands),
}

#[derive(Debug, Subcommand)]
pub enum AssetCommands {
    /// List assets in a project
    List {
        /// Project ID (without "b." prefix)
        project_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum SubmittalCommands {
    /// List submittals in a project
    List {
        /// Project ID (without "b." prefix)
        project_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ChecklistCommands {
    /// List checklists in a project
    List {
        /// Project ID (without "b." prefix)
        project_id: String,
    },

    /// List checklist templates
    Templates {
        /// Project ID (without "b." prefix)
        project_id: String,
    },
}

impl AccCommands {
    pub async fn execute(self, client: &AccClient, output_format: OutputFormat) -> Result<()> {
        match self {
            AccCommands::Asset(cmd) => cmd.execute(client, output_format).await,
            AccCommands::Submittal(cmd) => cmd.execute(client, output_format).await,
            AccCommands::Checklist(cmd) => cmd.execute(client, output_format).await,
        }
    }
}

impl AssetCommands {
    pub async fn execute(self, client: &AccClient, output_format: OutputFormat) -> Result<()> {
        match self {
            AssetCommands::List { project_id } => list_assets(client, &project_id, output_format).await,
        }
    }
}

impl SubmittalCommands {
    pub async fn execute(self, client: &AccClient, output_format: OutputFormat) -> Result<()> {
        match self {
            SubmittalCommands::List { project_id } => list_submittals(client, &project_id, output_format).await,
        }
    }
}

impl ChecklistCommands {
    pub async fn execute(self, client: &AccClient, output_format: OutputFormat) -> Result<()> {
        match self {
            ChecklistCommands::List { project_id } => list_checklists(client, &project_id, output_format).await,
            ChecklistCommands::Templates { project_id } => list_templates(client, &project_id, output_format).await,
        }
    }
}

// ============== ASSETS ==============

#[derive(Serialize)]
struct AssetOutput {
    id: String,
    category_id: Option<String>,
    description: Option<String>,
    barcode: Option<String>,
    updated_at: Option<String>,
}

async fn list_assets(client: &AccClient, project_id: &str, output_format: OutputFormat) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching assets...".dimmed());
    }

    let assets = client.list_assets(project_id).await?;

    let outputs: Vec<AssetOutput> = assets
        .iter()
        .map(|a| AssetOutput {
            id: a.id.clone(),
            category_id: a.category_id.clone(),
            description: a.description.clone(),
            barcode: a.barcode.clone(),
            updated_at: a.updated_at.clone(),
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No assets found.".yellow()),
            _ => output_format.write(&Vec::<AssetOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Assets:".bold());
            println!("{}", "─".repeat(80));
            println!(
                "{:<40} {:<30} {}",
                "ID".bold(),
                "Description".bold(),
                "Barcode".bold()
            );
            println!("{}", "─".repeat(80));

            for asset in &outputs {
                let desc = asset.description.as_deref().unwrap_or("-");
                let barcode = asset.barcode.as_deref().unwrap_or("-");
                println!(
                    "{:<40} {:<30} {}",
                    truncate_str(&asset.id, 40).cyan(),
                    truncate_str(desc, 30),
                    barcode.dimmed()
                );
            }

            println!("{}", "─".repeat(80));
            println!("{} {} asset(s) found", "→".cyan(), outputs.len());
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

// ============== SUBMITTALS ==============

#[derive(Serialize)]
struct SubmittalOutput {
    id: String,
    title: String,
    number: Option<String>,
    status: String,
    due_date: Option<String>,
}

async fn list_submittals(client: &AccClient, project_id: &str, output_format: OutputFormat) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching submittals...".dimmed());
    }

    let submittals = client.list_submittals(project_id).await?;

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

// ============== CHECKLISTS ==============

#[derive(Serialize)]
struct ChecklistOutput {
    id: String,
    title: String,
    status: String,
    location: Option<String>,
    due_date: Option<String>,
}

async fn list_checklists(client: &AccClient, project_id: &str, output_format: OutputFormat) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching checklists...".dimmed());
    }

    let checklists = client.list_checklists(project_id).await?;

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

#[derive(Serialize)]
struct TemplateOutput {
    id: String,
    title: String,
    description: Option<String>,
}

async fn list_templates(client: &AccClient, project_id: &str, output_format: OutputFormat) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching checklist templates...".dimmed());
    }

    let templates = client.list_checklist_templates(project_id).await?;

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
            println!(
                "{:<40} {}",
                "Title".bold(),
                "Description".bold()
            );
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

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}


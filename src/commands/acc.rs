//! ACC Extended Commands
//!
//! Commands for ACC modules: Assets, Submittals, Checklists

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use crate::api::acc::{
    CreateAssetRequest, CreateChecklistRequest, CreateSubmittalRequest, UpdateAssetRequest,
    UpdateChecklistRequest, UpdateSubmittalRequest,
};
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

    /// Get a specific asset
    Get {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Asset ID
        asset_id: String,
    },

    /// Create a new asset
    Create {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Asset description
        #[arg(long)]
        description: Option<String>,
        /// Barcode
        #[arg(long)]
        barcode: Option<String>,
        /// Category ID
        #[arg(long)]
        category_id: Option<String>,
    },

    /// Update an existing asset
    Update {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Asset ID
        asset_id: String,
        /// New description
        #[arg(long)]
        description: Option<String>,
        /// New barcode
        #[arg(long)]
        barcode: Option<String>,
        /// New status ID
        #[arg(long)]
        status_id: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum SubmittalCommands {
    /// List submittals in a project
    List {
        /// Project ID (without "b." prefix)
        project_id: String,
    },

    /// Get a specific submittal
    Get {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Submittal ID
        submittal_id: String,
    },

    /// Create a new submittal
    Create {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Submittal title
        #[arg(long)]
        title: String,
        /// Spec section reference
        #[arg(long)]
        spec_section: Option<String>,
        /// Due date (ISO 8601 format)
        #[arg(long)]
        due_date: Option<String>,
    },

    /// Update an existing submittal
    Update {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Submittal ID
        submittal_id: String,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New status
        #[arg(long)]
        status: Option<String>,
        /// New due date
        #[arg(long)]
        due_date: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum ChecklistCommands {
    /// List checklists in a project
    List {
        /// Project ID (without "b." prefix)
        project_id: String,
    },

    /// Get a specific checklist
    Get {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Checklist ID
        checklist_id: String,
    },

    /// Create a new checklist
    Create {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Checklist title
        #[arg(long)]
        title: String,
        /// Template ID to use
        #[arg(long)]
        template_id: Option<String>,
        /// Location reference
        #[arg(long)]
        location: Option<String>,
        /// Due date (ISO 8601 format)
        #[arg(long)]
        due_date: Option<String>,
        /// Assignee user ID
        #[arg(long)]
        assignee_id: Option<String>,
    },

    /// Update an existing checklist
    Update {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Checklist ID
        checklist_id: String,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New status
        #[arg(long)]
        status: Option<String>,
        /// New location
        #[arg(long)]
        location: Option<String>,
        /// New due date
        #[arg(long)]
        due_date: Option<String>,
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
            AssetCommands::List { project_id } => {
                list_assets(client, &project_id, output_format).await
            }
            AssetCommands::Get {
                project_id,
                asset_id,
            } => get_asset(client, &project_id, &asset_id, output_format).await,
            AssetCommands::Create {
                project_id,
                description,
                barcode,
                category_id,
            } => {
                create_asset(
                    client,
                    &project_id,
                    description,
                    barcode,
                    category_id,
                    output_format,
                )
                .await
            }
            AssetCommands::Update {
                project_id,
                asset_id,
                description,
                barcode,
                status_id,
            } => {
                update_asset(
                    client,
                    &project_id,
                    &asset_id,
                    description,
                    barcode,
                    status_id,
                    output_format,
                )
                .await
            }
        }
    }
}

impl SubmittalCommands {
    pub async fn execute(self, client: &AccClient, output_format: OutputFormat) -> Result<()> {
        match self {
            SubmittalCommands::List { project_id } => {
                list_submittals(client, &project_id, output_format).await
            }
            SubmittalCommands::Get {
                project_id,
                submittal_id,
            } => get_submittal(client, &project_id, &submittal_id, output_format).await,
            SubmittalCommands::Create {
                project_id,
                title,
                spec_section,
                due_date,
            } => {
                create_submittal(
                    client,
                    &project_id,
                    &title,
                    spec_section,
                    due_date,
                    output_format,
                )
                .await
            }
            SubmittalCommands::Update {
                project_id,
                submittal_id,
                title,
                status,
                due_date,
            } => {
                update_submittal(
                    client,
                    &project_id,
                    &submittal_id,
                    title,
                    status,
                    due_date,
                    output_format,
                )
                .await
            }
        }
    }
}

impl ChecklistCommands {
    pub async fn execute(self, client: &AccClient, output_format: OutputFormat) -> Result<()> {
        match self {
            ChecklistCommands::List { project_id } => {
                list_checklists(client, &project_id, output_format).await
            }
            ChecklistCommands::Get {
                project_id,
                checklist_id,
            } => get_checklist(client, &project_id, &checklist_id, output_format).await,
            ChecklistCommands::Create {
                project_id,
                title,
                template_id,
                location,
                due_date,
                assignee_id,
            } => {
                create_checklist(
                    client,
                    &project_id,
                    &title,
                    template_id,
                    location,
                    due_date,
                    assignee_id,
                    output_format,
                )
                .await
            }
            ChecklistCommands::Update {
                project_id,
                checklist_id,
                title,
                status,
                location,
                due_date,
            } => {
                update_checklist(
                    client,
                    &project_id,
                    &checklist_id,
                    title,
                    status,
                    location,
                    due_date,
                    output_format,
                )
                .await
            }
            ChecklistCommands::Templates { project_id } => {
                list_templates(client, &project_id, output_format).await
            }
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

async fn list_assets(
    client: &AccClient,
    project_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
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

async fn list_submittals(
    client: &AccClient,
    project_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
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

async fn list_checklists(
    client: &AccClient,
    project_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
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

async fn list_templates(
    client: &AccClient,
    project_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
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

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

// ============== ASSET CRUD ==============

async fn get_asset(
    client: &AccClient,
    project_id: &str,
    asset_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching asset details...".dimmed());
    }

    let asset = client.get_asset(project_id, asset_id).await?;

    let output = AssetOutput {
        id: asset.id.clone(),
        category_id: asset.category_id.clone(),
        description: asset.description.clone(),
        barcode: asset.barcode.clone(),
        updated_at: asset.updated_at.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Asset Details:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "ID:".bold(), asset.id.cyan());
            println!(
                "{:<15} {}",
                "Category:".bold(),
                asset.category_id.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Description:".bold(),
                asset.description.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Barcode:".bold(),
                asset.barcode.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Updated:".bold(),
                asset.updated_at.as_deref().unwrap_or("-")
            );
            println!("{}", "─".repeat(60));
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

async fn create_asset(
    client: &AccClient,
    project_id: &str,
    description: Option<String>,
    barcode: Option<String>,
    category_id: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Creating asset...".dimmed());
    }

    let request = CreateAssetRequest {
        description,
        barcode,
        category_id,
        client_asset_id: None,
    };

    let asset = client.create_asset(project_id, request).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Asset created successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), asset.id.cyan());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "id": asset.id,
                "created": true
            }))?;
        }
    }

    Ok(())
}

async fn update_asset(
    client: &AccClient,
    project_id: &str,
    asset_id: &str,
    description: Option<String>,
    barcode: Option<String>,
    status_id: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Updating asset...".dimmed());
    }

    let request = UpdateAssetRequest {
        description,
        barcode,
        status_id,
        category_id: None,
    };

    let asset = client.update_asset(project_id, asset_id, request).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Asset updated successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), asset.id.cyan());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "id": asset.id,
                "updated": true
            }))?;
        }
    }

    Ok(())
}

// ============== SUBMITTAL CRUD ==============

async fn get_submittal(
    client: &AccClient,
    project_id: &str,
    submittal_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching submittal details...".dimmed());
    }

    let submittal = client.get_submittal(project_id, submittal_id).await?;

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

async fn create_submittal(
    client: &AccClient,
    project_id: &str,
    title: &str,
    spec_section: Option<String>,
    due_date: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Creating submittal...".dimmed());
    }

    let request = CreateSubmittalRequest {
        title: title.to_string(),
        spec_section,
        due_date,
    };

    let submittal = client.create_submittal(project_id, request).await?;

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

async fn update_submittal(
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
        .await?;

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

// ============== CHECKLIST CRUD ==============

async fn get_checklist(
    client: &AccClient,
    project_id: &str,
    checklist_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching checklist details...".dimmed());
    }

    let checklist = client.get_checklist(project_id, checklist_id).await?;

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
async fn create_checklist(
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

    let checklist = client.create_checklist(project_id, request).await?;

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

async fn update_checklist(
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
        .await?;

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

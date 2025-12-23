//! Folder management commands
//!
//! Commands for listing, creating, and managing folders (requires 3-legged auth).

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::Input;
use serde::Serialize;

use crate::api::DataManagementClient;
use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum FolderCommands {
    /// List folder contents
    List {
        /// Project ID
        project_id: String,
        /// Folder ID
        folder_id: String,
    },

    /// Create a new folder
    Create {
        /// Project ID
        project_id: String,
        /// Parent folder ID
        parent_folder_id: String,
        /// Folder name (interactive if not provided)
        #[arg(short, long)]
        name: Option<String>,
    },
}

impl FolderCommands {
    pub async fn execute(self, client: &DataManagementClient, output_format: OutputFormat) -> Result<()> {
        match self {
            FolderCommands::List {
                project_id,
                folder_id,
            } => list_folder_contents(client, &project_id, &folder_id, output_format).await,
            FolderCommands::Create {
                project_id,
                parent_folder_id,
                name,
            } => create_folder(client, &project_id, &parent_folder_id, name, output_format).await,
        }
    }
}

#[derive(Serialize)]
struct FolderItemOutput {
    id: String,
    name: String,
    item_type: String,
}

async fn list_folder_contents(
    client: &DataManagementClient,
    project_id: &str,
    folder_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching folder contents...".dimmed());
    }

    let contents = client.list_folder_contents(project_id, folder_id).await?;

    let items: Vec<FolderItemOutput> = contents
        .iter()
        .map(|item| {
            let item_type = item
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");
            let id = item.get("id").and_then(|i| i.as_str()).unwrap_or("unknown");
            let name = item
                .get("attributes")
                .and_then(|a| a.get("displayName").or(a.get("name")))
                .and_then(|n| n.as_str())
                .unwrap_or("Unnamed");

            FolderItemOutput {
                id: id.to_string(),
                name: name.to_string(),
                item_type: item_type.to_string(),
            }
        })
        .collect();

    if items.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "Folder is empty.".yellow()),
            _ => {
                output_format.write(&Vec::<FolderItemOutput>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Folder Contents:".bold());
            println!("{}", "─".repeat(80));

            for item in &items {
                let icon = if item.item_type == "folders" {
                    "📁"
                } else {
                    "📄"
                };
                let type_label = if item.item_type == "folders" {
                    "folder"
                } else {
                    "item"
                };

                println!("  {} {} [{}]", icon, item.name.cyan(), type_label.dimmed());
                println!("    {} {}", "ID:".dimmed(), item.id);
            }

            println!("{}", "─".repeat(80));
        }
        _ => {
            output_format.write(&items)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct CreateFolderOutput {
    success: bool,
    id: String,
    name: String,
}

async fn create_folder(
    client: &DataManagementClient,
    project_id: &str,
    parent_folder_id: &str,
    name: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let folder_name = match name {
        Some(n) => n,
        None => Input::new()
            .with_prompt("Enter folder name")
            .interact_text()?,
    };

    if output_format.supports_colors() {
        println!("{}", "Creating folder...".dimmed());
    }

    let folder = client
        .create_folder(project_id, parent_folder_id, &folder_name)
        .await?;

    let output = CreateFolderOutput {
        success: true,
        id: folder.id.clone(),
        name: folder.attributes.name.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Folder created successfully!", "✓".green().bold());
            println!("  {} {}", "Name:".bold(), output.name.cyan());
            println!("  {} {}", "ID:".bold(), output.id);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

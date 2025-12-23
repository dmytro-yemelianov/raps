//! Folder management commands
//! 
//! Commands for listing, creating, and managing folders (requires 3-legged auth).

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::Input;

use crate::api::DataManagementClient;

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
    pub async fn execute(self, client: &DataManagementClient) -> Result<()> {
        match self {
            FolderCommands::List { project_id, folder_id } => {
                list_folder_contents(client, &project_id, &folder_id).await
            }
            FolderCommands::Create { project_id, parent_folder_id, name } => {
                create_folder(client, &project_id, &parent_folder_id, name).await
            }
        }
    }
}

async fn list_folder_contents(client: &DataManagementClient, project_id: &str, folder_id: &str) -> Result<()> {
    println!("{}", "Fetching folder contents...".dimmed());

    let contents = client.list_folder_contents(project_id, folder_id).await?;

    if contents.is_empty() {
        println!("{}", "Folder is empty.".yellow());
        return Ok(());
    }

    println!("\n{}", "Folder Contents:".bold());
    println!("{}", "─".repeat(80));

    for item in contents {
        let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("unknown");
        let id = item.get("id").and_then(|i| i.as_str()).unwrap_or("unknown");
        
        let name = item.get("attributes")
            .and_then(|a| a.get("displayName").or(a.get("name")))
            .and_then(|n| n.as_str())
            .unwrap_or("Unnamed");

        let icon = if item_type == "folders" { "📁" } else { "📄" };
        let type_label = if item_type == "folders" { "folder" } else { "item" };

        println!("  {} {} [{}]", icon, name.cyan(), type_label.dimmed());
        println!("    {} {}", "ID:".dimmed(), id);
    }

    println!("{}", "─".repeat(80));
    Ok(())
}

async fn create_folder(
    client: &DataManagementClient,
    project_id: &str,
    parent_folder_id: &str,
    name: Option<String>,
) -> Result<()> {
    let folder_name = match name {
        Some(n) => n,
        None => {
            Input::new()
                .with_prompt("Enter folder name")
                .interact_text()?
        }
    };

    println!("{}", "Creating folder...".dimmed());

    let folder = client.create_folder(project_id, parent_folder_id, &folder_name).await?;

    println!("{} Folder created successfully!", "✓".green().bold());
    println!("  {} {}", "Name:".bold(), folder.attributes.name.cyan());
    println!("  {} {}", "ID:".bold(), folder.id);

    Ok(())
}


// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Folder management commands
//!
//! Commands for listing, creating, and managing folders (requires 3-legged auth).

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::Input;
#[allow(unused_imports)]
use raps_kernel::prompts;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_acc::permissions::FolderPermissionsClient;
use raps_dm::DataManagementClient;
use raps_kernel::interactive;
// use raps_kernel::output::OutputFormat;

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

    /// Rename a folder
    Rename {
        /// Project ID
        project_id: String,
        /// Folder ID
        folder_id: String,
        /// New folder name
        #[arg(short, long)]
        name: String,
    },

    /// Delete a folder
    Delete {
        /// Project ID
        project_id: String,
        /// Folder ID
        folder_id: String,
    },

    /// Show permissions (rights) for a folder
    Rights {
        /// Project ID
        project_id: String,
        /// Folder ID
        folder_id: String,
    },
}

impl FolderCommands {
    pub async fn execute(
        self,
        client: &DataManagementClient,
        permissions_client: &FolderPermissionsClient,
        output_format: OutputFormat,
    ) -> Result<()> {
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
            FolderCommands::Rename {
                project_id,
                folder_id,
                name,
            } => rename_folder(client, &project_id, &folder_id, &name, output_format).await,
            FolderCommands::Delete {
                project_id,
                folder_id,
            } => delete_folder(client, &project_id, &folder_id, output_format).await,
            FolderCommands::Rights {
                project_id,
                folder_id,
            } => {
                folder_rights(permissions_client, &project_id, &folder_id, output_format).await
            }
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
            println!("{}", "-".repeat(80));

            for item in &items {
                let icon = if item.item_type == "folders" {
                    "[folder]"
                } else {
                    "[file]"
                };
                let type_label = if item.item_type == "folders" {
                    "folder"
                } else {
                    "item"
                };

                println!("  {} {} [{}]", icon, item.name.cyan(), type_label.dimmed());
                println!("    {} {}", "ID:".dimmed(), item.id);
            }

            println!("{}", "-".repeat(80));
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
        None => {
            // In non-interactive mode, require the name
            if interactive::is_non_interactive() {
                anyhow::bail!("Folder name is required in non-interactive mode. Use --name flag.");
            }
            Input::new()
                .with_prompt("Enter folder name")
                .interact_text()?
        }
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

#[derive(Serialize)]
struct RenameFolderOutput {
    success: bool,
    id: String,
    name: String,
}

async fn rename_folder(
    client: &DataManagementClient,
    project_id: &str,
    folder_id: &str,
    new_name: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Renaming folder...".dimmed());
    }

    let folder = client
        .rename_folder(project_id, folder_id, new_name)
        .await?;

    let output = RenameFolderOutput {
        success: true,
        id: folder.id.clone(),
        name: folder.attributes.name.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Folder renamed successfully!", "✓".green().bold());
            println!("  {} {}", "Name:".bold(), output.name.cyan());
            println!("  {} {}", "ID:".bold(), output.id);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct DeleteFolderOutput {
    success: bool,
    folder_id: String,
    message: String,
}

async fn delete_folder(
    client: &DataManagementClient,
    project_id: &str,
    folder_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Deleting folder...".dimmed());
    }

    client.delete_folder(project_id, folder_id).await?;

    let output = DeleteFolderOutput {
        success: true,
        folder_id: folder_id.to_string(),
        message: "Folder deleted successfully!".to_string(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} {}", "✓".green().bold(), output.message);
        }
        _ => {
            output_format.write(&output)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct FolderRightOutput {
    subject_id: String,
    subject_type: String,
    actions: Vec<String>,
    inherited_from: Option<String>,
}

async fn folder_rights(
    client: &FolderPermissionsClient,
    project_id: &str,
    folder_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching folder permissions...".dimmed());
    }

    let permissions = client.get_permissions(project_id, folder_id).await?;

    let items: Vec<FolderRightOutput> = permissions
        .iter()
        .map(|p| FolderRightOutput {
            subject_id: p.subject_id.clone(),
            subject_type: p.subject_type.clone(),
            actions: p.actions.clone(),
            inherited_from: p.inherited_from.clone(),
        })
        .collect();

    if items.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No permissions found for this folder.".yellow()),
            _ => {
                output_format.write(&Vec::<FolderRightOutput>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Folder Permissions:".bold());
            println!("{}", "-".repeat(80));

            for item in &items {
                let inherited = item
                    .inherited_from
                    .as_deref()
                    .unwrap_or("direct");
                println!(
                    "  {} {} [{}]",
                    item.subject_type.cyan(),
                    item.subject_id,
                    inherited.dimmed()
                );
                println!("    {} {}", "Actions:".dimmed(), item.actions.join(", "));
            }

            println!("{}", "-".repeat(80));
        }
        _ => {
            output_format.write(&items)?;
        }
    }
    Ok(())
}

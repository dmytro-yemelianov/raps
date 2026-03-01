// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Folder management commands
//!
//! Commands for listing, creating, and managing folders (requires 3-legged auth).

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use dialoguer::Input;
#[allow(unused_imports)]
use raps_kernel::prompts;
use serde::Serialize;

use crate::commands::interactive;
use crate::commands::tracked::tracked_op;

use crate::output::OutputFormat;
use raps_acc::permissions::FolderPermissionsClient;
use raps_dm::DataManagementClient;
// use raps_kernel::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum FolderCommands {
    /// List folder contents
    List {
        /// Project ID
        project_id: Option<String>,
        /// Folder ID
        folder_id: Option<String>,
        /// Hub ID (for interactive mode)
        #[arg(long, hide = true)]
        hub_id: Option<String>,
    },

    /// Create a new folder
    Create {
        /// Project ID
        project_id: Option<String>,
        /// Parent folder ID
        parent_folder_id: Option<String>,
        /// Folder name (interactive if not provided)
        #[arg(short, long)]
        name: Option<String>,
        /// Hub ID (for interactive mode)
        #[arg(long, hide = true)]
        hub_id: Option<String>,
    },

    /// Rename a folder
    Rename {
        /// Project ID
        project_id: Option<String>,
        /// Folder ID
        folder_id: Option<String>,
        /// New folder name (interactive if not provided)
        #[arg(short, long)]
        name: Option<String>,
        /// Hub ID (for interactive mode)
        #[arg(long, hide = true)]
        hub_id: Option<String>,
    },

    /// Delete a folder
    Delete {
        /// Project ID
        project_id: Option<String>,
        /// Folder ID
        folder_id: Option<String>,
        /// Hub ID (for interactive mode)
        #[arg(long, hide = true)]
        hub_id: Option<String>,
    },

    /// Show permissions (rights) for a folder
    Rights {
        /// Project ID
        project_id: Option<String>,
        /// Folder ID
        folder_id: Option<String>,
        /// Hub ID (for interactive mode)
        #[arg(long, hide = true)]
        hub_id: Option<String>,
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
                hub_id,
            } => {
                let (p_id, f_id) =
                    resolve_folder_args(client, hub_id, project_id, folder_id).await?;
                list_folder_contents(client, &p_id, &f_id, output_format).await
            }
            FolderCommands::Create {
                project_id,
                parent_folder_id,
                name,
                hub_id,
            } => {
                let (p_id, f_id) =
                    resolve_folder_args(client, hub_id, project_id, parent_folder_id).await?;
                create_folder(client, &p_id, &f_id, name, output_format).await
            }
            FolderCommands::Rename {
                project_id,
                folder_id,
                name,
                hub_id,
            } => {
                let (p_id, f_id) =
                    resolve_folder_args(client, hub_id, project_id, folder_id).await?;
                rename_folder(client, &p_id, &f_id, name, output_format).await
            }
            FolderCommands::Delete {
                project_id,
                folder_id,
                hub_id,
            } => {
                let (p_id, f_id) =
                    resolve_folder_args(client, hub_id, project_id, folder_id).await?;
                if !raps_kernel::interactive::should_proceed_destructive("delete this folder") {
                    println!("Operation cancelled.");
                    return Ok(());
                }
                delete_folder(client, &p_id, &f_id, output_format).await
            }
            FolderCommands::Rights {
                project_id,
                folder_id,
                hub_id,
            } => {
                let (p_id, f_id) =
                    resolve_folder_args(client, hub_id, project_id, folder_id).await?;
                folder_rights(permissions_client, &p_id, &f_id, output_format).await
            }
        }
    }
}

async fn resolve_folder_args(
    client: &DataManagementClient,
    opt_hub_id: Option<String>,
    opt_project_id: Option<String>,
    opt_folder_id: Option<String>,
) -> Result<(String, String)> {
    let hub_id = match (&opt_hub_id, &opt_project_id, &opt_folder_id) {
        (Some(h), _, _) => h.clone(),
        (None, Some(_), Some(_)) => String::new(), // Not needed if both P and F are provided
        (None, _, _) => interactive::prompt_for_hub(client).await?,
    };

    let project_id = match opt_project_id {
        Some(p) => p,
        None => interactive::prompt_for_project(client, &hub_id).await?,
    };

    let folder_id = match opt_folder_id {
        Some(f) => f,
        None => interactive::prompt_for_folder(client, &hub_id, &project_id).await?,
    };

    Ok((project_id, folder_id))
}

#[derive(Serialize, schemars::JsonSchema)]
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
    let contents = tracked_op("Fetching folder contents", output_format, || async {
        client
            .list_folder_contents(project_id, folder_id)
            .await
            .context(format!(
                "Failed to list folder '{}' contents. Verify folder ID and permissions",
                folder_id
            ))
    })
    .await?;

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

#[derive(Serialize, schemars::JsonSchema)]
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
            raps_kernel::prompts::spawn_prompt(|| {
                Ok(Input::new()
                    .with_prompt("Enter folder name")
                    .interact_text()?)
            })
            .await?
        }
    };

    if output_format.supports_colors() {
        println!("{}", "Creating folder...".dimmed());
    }

    let folder = client
        .create_folder(project_id, parent_folder_id, &folder_name)
        .await
        .context(format!(
            "Failed to create folder '{}' in project '{}'. Check parent folder permissions",
            folder_name, project_id
        ))?;

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

#[derive(Serialize, schemars::JsonSchema)]
struct RenameFolderOutput {
    success: bool,
    id: String,
    name: String,
}

async fn rename_folder(
    client: &DataManagementClient,
    project_id: &str,
    folder_id: &str,
    new_name: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let name_str = match new_name {
        Some(n) => n,
        None => {
            if interactive::is_non_interactive() {
                anyhow::bail!("Folder name is required in non-interactive mode. Use --name flag.");
            }
            raps_kernel::prompts::spawn_prompt(|| {
                Ok(Input::new()
                    .with_prompt("Enter new folder name")
                    .interact_text()?)
            })
            .await?
        }
    };

    if output_format.supports_colors() {
        println!("{}", "Renaming folder...".dimmed());
    }

    let folder = client
        .rename_folder(project_id, folder_id, &name_str)
        .await
        .context(format!(
            "Failed to rename folder '{}'. Check permissions and that folder exists",
            folder_id
        ))?;

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

#[derive(Serialize, schemars::JsonSchema)]
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

    client
        .delete_folder(project_id, folder_id)
        .await
        .context(format!(
            "Failed to delete folder '{}'. Folder may not be empty or you lack permissions",
            folder_id
        ))?;

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

#[derive(Serialize, schemars::JsonSchema)]
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
    let permissions = tracked_op("Fetching folder permissions", output_format, || async {
        client
            .get_permissions(project_id, folder_id)
            .await
            .context(format!(
                "Failed to get permissions for folder '{}'",
                folder_id
            ))
    })
    .await?;

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
                let inherited = item.inherited_from.as_deref().unwrap_or("direct");
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

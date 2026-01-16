// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Item (file) management commands
//!
//! Commands for listing, viewing, and downloading items (requires 3-legged auth).

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_dm::DataManagementClient;
// use raps_kernel::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum ItemCommands {
    /// Get item details
    Info {
        /// Project ID
        project_id: String,
        /// Item ID
        item_id: String,
    },

    /// List item versions
    Versions {
        /// Project ID
        project_id: String,
        /// Item ID
        item_id: String,
    },

    /// Create an item from an OSS object (bind OSS upload to ACC folder)
    #[command(name = "create-from-oss")]
    CreateFromOss {
        /// Project ID (with "b." prefix)
        project_id: String,
        /// Target folder ID (get from folder list)
        folder_id: String,
        /// Display name for the item
        #[arg(short, long)]
        name: String,
        /// OSS object ID (urn:adsk.objects:os.object:bucket/objectkey)
        #[arg(long)]
        object_id: String,
    },
}

impl ItemCommands {
    pub async fn execute(
        self,
        client: &DataManagementClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            ItemCommands::Info {
                project_id,
                item_id,
            } => item_info(client, &project_id, &item_id, output_format).await,
            ItemCommands::Versions {
                project_id,
                item_id,
            } => list_versions(client, &project_id, &item_id, output_format).await,
            ItemCommands::CreateFromOss {
                project_id,
                folder_id,
                name,
                object_id,
            } => {
                create_from_oss(
                    client,
                    &project_id,
                    &folder_id,
                    &name,
                    &object_id,
                    output_format,
                )
                .await
            }
        }
    }
}

#[derive(Serialize)]
struct ItemInfoOutput {
    id: String,
    name: String,
    item_type: String,
    create_time: Option<String>,
    modified_time: Option<String>,
    extension_type: Option<String>,
    extension_version: Option<String>,
}

async fn item_info(
    client: &DataManagementClient,
    project_id: &str,
    item_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching item details...".dimmed());
    }

    let item = client.get_item(project_id, item_id).await?;

    let extension_type = item
        .attributes
        .extension
        .as_ref()
        .and_then(|e| e.extension_type.clone());
    let extension_version = item
        .attributes
        .extension
        .as_ref()
        .and_then(|e| e.version.clone());

    let output = ItemInfoOutput {
        id: item.id.clone(),
        name: item.attributes.display_name.clone(),
        item_type: item.item_type.clone(),
        create_time: item.attributes.create_time.clone(),
        modified_time: item.attributes.last_modified_time.clone(),
        extension_type,
        extension_version,
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Item Details".bold());
            println!("{}", "-".repeat(60));
            println!("  {} {}", "Name:".bold(), output.name.cyan());
            println!("  {} {}", "ID:".bold(), output.id);
            println!("  {} {}", "Type:".bold(), output.item_type);

            if let Some(ref create_time) = output.create_time {
                println!("  {} {}", "Created:".bold(), create_time);
            }

            if let Some(ref modified_time) = output.modified_time {
                println!("  {} {}", "Modified:".bold(), modified_time);
            }

            if let Some(ref ext_type) = output.extension_type {
                println!("  {} {}", "Extension:".bold(), ext_type);
            }
            if let Some(version) = output.extension_version {
                println!("  {} {}", "Ext Version:".bold(), version);
            }

            println!("{}", "-".repeat(60));
            println!(
                "\n{}",
                "Use 'raps item versions' to see version history".dimmed()
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct VersionOutput {
    version_number: Option<i32>,
    name: String,
    size: Option<u64>,
    size_human: Option<String>,
    create_time: Option<String>,
}

async fn list_versions(
    client: &DataManagementClient,
    project_id: &str,
    item_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching item versions...".dimmed());
    }

    let versions = client.get_item_versions(project_id, item_id).await?;

    let version_outputs: Vec<VersionOutput> = versions
        .iter()
        .map(|v| {
            let name = v
                .attributes
                .display_name
                .as_ref()
                .or(Some(&v.attributes.name))
                .cloned()
                .unwrap_or_default();
            VersionOutput {
                version_number: v.attributes.version_number,
                name,
                size: v.attributes.storage_size.map(|s| s as u64),
                size_human: v.attributes.storage_size.map(|s| format_size(s as u64)),
                create_time: v.attributes.create_time.clone(),
            }
        })
        .collect();

    if version_outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No versions found.".yellow()),
            _ => {
                output_format.write(&Vec::<VersionOutput>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Item Versions:".bold());
            println!("{}", "-".repeat(80));
            println!(
                "{:<6} {:<40} {:>12} {}",
                "Ver".bold(),
                "Name".bold(),
                "Size".bold(),
                "Created".bold()
            );
            println!("{}", "-".repeat(80));

            for version in &version_outputs {
                let ver_num = version
                    .version_number
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let name = truncate_str(&version.name, 40);
                let size = version.size_human.as_deref().unwrap_or("-");
                let created = version.create_time.as_deref().unwrap_or("-");

                println!(
                    "{:<6} {:<40} {:>12} {}",
                    ver_num.cyan(),
                    name,
                    size,
                    created.dimmed()
                );
            }

            println!("{}", "-".repeat(80));
        }
        _ => {
            output_format.write(&version_outputs)?;
        }
    }
    Ok(())
}

/// Format file size in human-readable format
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Truncate string with ellipsis
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[derive(Serialize)]
struct CreateFromOssOutput {
    success: bool,
    item_id: String,
    name: String,
    message: String,
}

async fn create_from_oss(
    client: &DataManagementClient,
    project_id: &str,
    folder_id: &str,
    name: &str,
    object_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Creating item from OSS object...".dimmed());
        println!("  {} {}", "Project:".bold(), project_id);
        println!("  {} {}", "Folder:".bold(), folder_id);
        println!("  {} {}", "Name:".bold(), name.cyan());
        println!("  {} {}", "Object ID:".bold(), object_id.dimmed());
    }

    // Create the item using the Data Management API
    let item = client
        .create_item_from_storage(project_id, folder_id, name, object_id)
        .await?;

    let output = CreateFromOssOutput {
        success: true,
        item_id: item.id.clone(),
        name: item.attributes.display_name.clone(),
        message: format!("Item '{}' created successfully from OSS object", name),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{} {}", "✓".green().bold(), output.message);
            println!("  {} {}", "Item ID:".bold(), output.item_id);
            println!("  {} {}", "Name:".bold(), output.name.cyan());
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

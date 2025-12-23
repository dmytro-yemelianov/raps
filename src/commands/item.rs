//! Item (file) management commands
//! 
//! Commands for listing, viewing, and downloading items (requires 3-legged auth).

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;

use crate::api::DataManagementClient;

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
}

impl ItemCommands {
    pub async fn execute(self, client: &DataManagementClient) -> Result<()> {
        match self {
            ItemCommands::Info { project_id, item_id } => {
                item_info(client, &project_id, &item_id).await
            }
            ItemCommands::Versions { project_id, item_id } => {
                list_versions(client, &project_id, &item_id).await
            }
        }
    }
}

async fn item_info(client: &DataManagementClient, project_id: &str, item_id: &str) -> Result<()> {
    println!("{}", "Fetching item details...".dimmed());

    let item = client.get_item(project_id, item_id).await?;

    println!("\n{}", "Item Details".bold());
    println!("{}", "─".repeat(60));
    println!("  {} {}", "Name:".bold(), item.attributes.display_name.cyan());
    println!("  {} {}", "ID:".bold(), item.id);
    println!("  {} {}", "Type:".bold(), item.item_type);
    
    if let Some(ref create_time) = item.attributes.create_time {
        println!("  {} {}", "Created:".bold(), create_time);
    }
    
    if let Some(ref modified_time) = item.attributes.last_modified_time {
        println!("  {} {}", "Modified:".bold(), modified_time);
    }

    if let Some(ref ext) = item.attributes.extension {
        if let Some(ref ext_type) = ext.extension_type {
            println!("  {} {}", "Extension:".bold(), ext_type);
        }
        if let Some(ref version) = ext.version {
            println!("  {} {}", "Ext Version:".bold(), version);
        }
    }

    println!("{}", "─".repeat(60));
    println!("\n{}", "Use 'raps item versions' to see version history".dimmed());
    Ok(())
}

async fn list_versions(client: &DataManagementClient, project_id: &str, item_id: &str) -> Result<()> {
    println!("{}", "Fetching item versions...".dimmed());

    let versions = client.get_item_versions(project_id, item_id).await?;

    if versions.is_empty() {
        println!("{}", "No versions found.".yellow());
        return Ok(());
    }

    println!("\n{}", "Item Versions:".bold());
    println!("{}", "─".repeat(80));
    println!(
        "{:<6} {:<40} {:>12} {}",
        "Ver".bold(),
        "Name".bold(),
        "Size".bold(),
        "Created".bold()
    );
    println!("{}", "─".repeat(80));

    for version in versions {
        let ver_num = version.attributes.version_number
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        
        let name = version.attributes.display_name
            .as_ref()
            .or(Some(&version.attributes.name))
            .map(|s| truncate_str(s, 40))
            .unwrap_or_default();
        
        let size = version.attributes.storage_size
            .map(|s| format_size(s as u64))
            .unwrap_or_else(|| "-".to_string());
        
        let created = version.attributes.create_time
            .as_deref()
            .unwrap_or("-");

        println!(
            "{:<6} {:<40} {:>12} {}",
            ver_num.cyan(),
            name,
            size,
            created.dimmed()
        );
    }

    println!("{}", "─".repeat(80));
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


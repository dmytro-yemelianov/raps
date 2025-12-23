//! Hub management commands
//!
//! Commands for listing and viewing hubs (requires 3-legged auth).

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;

use crate::api::DataManagementClient;

#[derive(Debug, Subcommand)]
pub enum HubCommands {
    /// List all accessible hubs
    List,

    /// Get hub details
    Info {
        /// Hub ID
        hub_id: String,
    },
}

impl HubCommands {
    pub async fn execute(self, client: &DataManagementClient) -> Result<()> {
        match self {
            HubCommands::List => list_hubs(client).await,
            HubCommands::Info { hub_id } => hub_info(client, &hub_id).await,
        }
    }
}

async fn list_hubs(client: &DataManagementClient) -> Result<()> {
    println!("{}", "Fetching hubs (requires 3-legged auth)...".dimmed());

    let hubs = client.list_hubs().await?;

    if hubs.is_empty() {
        println!("{}", "No hubs found.".yellow());
        return Ok(());
    }

    println!("\n{}", "Hubs:".bold());
    println!("{}", "─".repeat(80));
    println!(
        "{:<45} {:<15} {}",
        "Hub Name".bold(),
        "Type".bold(),
        "Region".bold()
    );
    println!("{}", "─".repeat(80));

    for hub in hubs {
        let hub_type = hub
            .attributes
            .extension
            .and_then(|e| e.extension_type)
            .map(|t| extract_hub_type(&t))
            .unwrap_or_else(|| "Unknown".to_string());

        let region = hub.attributes.region.as_deref().unwrap_or("US");

        println!(
            "{:<45} {:<15} {}",
            hub.attributes.name.cyan(),
            hub_type,
            region.dimmed()
        );
        println!("  {} {}", "ID:".dimmed(), hub.id);
    }

    println!("{}", "─".repeat(80));
    Ok(())
}

async fn hub_info(client: &DataManagementClient, hub_id: &str) -> Result<()> {
    println!("{}", "Fetching hub details...".dimmed());

    let hub = client.get_hub(hub_id).await?;

    println!("\n{}", "Hub Details".bold());
    println!("{}", "─".repeat(60));
    println!("  {} {}", "Name:".bold(), hub.attributes.name.cyan());
    println!("  {} {}", "ID:".bold(), hub.id);
    println!("  {} {}", "Type:".bold(), hub.hub_type);

    if let Some(ref region) = hub.attributes.region {
        println!("  {} {}", "Region:".bold(), region);
    }

    if let Some(ref ext) = hub.attributes.extension {
        if let Some(ref ext_type) = ext.extension_type {
            println!("  {} {}", "Extension:".bold(), extract_hub_type(ext_type));
        }
    }

    println!("{}", "─".repeat(60));
    println!(
        "\n{}",
        "Use 'raps project list <hub-id>' to see projects".dimmed()
    );
    Ok(())
}

/// Extract friendly hub type from extension type
fn extract_hub_type(ext_type: &str) -> String {
    if ext_type.contains("bim360") {
        "BIM 360".to_string()
    } else if ext_type.contains("accproject") {
        "ACC".to_string()
    } else if ext_type.contains("a360") {
        "A360".to_string()
    } else if ext_type.contains("fusion") {
        "Fusion".to_string()
    } else {
        ext_type
            .split(':')
            .next_back()
            .unwrap_or("Unknown")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_hub_type_bim360() {
        assert_eq!(extract_hub_type("autodesk.bim360:Account"), "BIM 360");
        assert_eq!(extract_hub_type("something.bim360.else"), "BIM 360");
    }

    #[test]
    fn test_extract_hub_type_acc() {
        assert_eq!(extract_hub_type("autodesk.accproject:Project"), "ACC");
        assert_eq!(extract_hub_type("prefix.accproject.suffix"), "ACC");
    }

    #[test]
    fn test_extract_hub_type_a360() {
        assert_eq!(extract_hub_type("autodesk.a360:Account"), "A360");
        assert_eq!(extract_hub_type("something.a360.else"), "A360");
    }

    #[test]
    fn test_extract_hub_type_fusion() {
        assert_eq!(extract_hub_type("autodesk.fusion:Account"), "Fusion");
        assert_eq!(extract_hub_type("something.fusion.else"), "Fusion");
    }

    #[test]
    fn test_extract_hub_type_with_colon() {
        assert_eq!(extract_hub_type("autodesk:something:Project"), "Project");
        assert_eq!(extract_hub_type("namespace:type:subtype"), "subtype");
    }

    #[test]
    fn test_extract_hub_type_unknown() {
        assert_eq!(extract_hub_type("unknown"), "unknown");
        // Empty string splits to [""], next_back() returns Some(""), so result is ""
        assert_eq!(extract_hub_type(""), "");
        // But if there's no colon, it returns the whole string
        assert_eq!(extract_hub_type("simple"), "simple");
    }
}

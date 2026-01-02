// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Hub management commands
//!
//! Commands for listing and viewing hubs (requires 3-legged auth).

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use crate::api::DataManagementClient;
use crate::output::OutputFormat;

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
    pub async fn execute(
        self,
        client: &DataManagementClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            HubCommands::List => list_hubs(client, output_format).await,
            HubCommands::Info { hub_id } => hub_info(client, &hub_id, output_format).await,
        }
    }
}

#[derive(Serialize)]
struct HubListOutput {
    id: String,
    name: String,
    hub_type: String,
    extension_type: Option<String>,
    region: Option<String>,
}

async fn list_hubs(client: &DataManagementClient, output_format: OutputFormat) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching hubs (requires 3-legged auth)...".dimmed());
    }

    let hubs = client.list_hubs().await?;

    let hub_outputs: Vec<HubListOutput> = hubs
        .iter()
        .map(|h| {
            let extension_type = h
                .attributes
                .extension
                .as_ref()
                .and_then(|e| e.extension_type.as_ref())
                .map(|t| extract_hub_type(t));
            HubListOutput {
                id: h.id.clone(),
                name: h.attributes.name.clone(),
                hub_type: h.hub_type.clone(),
                extension_type,
                region: h.attributes.region.clone(),
            }
        })
        .collect();

    if hub_outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No hubs found.".yellow()),
            _ => {
                output_format.write(&Vec::<HubListOutput>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Hubs:".bold());
            println!("{}", "─".repeat(80));
            println!(
                "{:<45} {:<15} {}",
                "Hub Name".bold(),
                "Type".bold(),
                "Region".bold()
            );
            println!("{}", "─".repeat(80));

            for hub in &hub_outputs {
                let hub_type = hub.extension_type.as_deref().unwrap_or("Unknown");
                let region = hub.region.as_deref().unwrap_or("US");

                println!(
                    "{:<45} {:<15} {}",
                    hub.name.cyan(),
                    hub_type,
                    region.dimmed()
                );
                println!("  {} {}", "ID:".dimmed(), hub.id);
            }

            println!("{}", "─".repeat(80));
        }
        _ => {
            output_format.write(&hub_outputs)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct HubInfoOutput {
    id: String,
    name: String,
    hub_type: String,
    region: Option<String>,
    extension_type: Option<String>,
}

async fn hub_info(
    client: &DataManagementClient,
    hub_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching hub details...".dimmed());
    }

    let hub = client.get_hub(hub_id).await?;

    let extension_type = hub
        .attributes
        .extension
        .as_ref()
        .and_then(|e| e.extension_type.as_ref())
        .map(|t| extract_hub_type(t));

    let output = HubInfoOutput {
        id: hub.id.clone(),
        name: hub.attributes.name.clone(),
        hub_type: hub.hub_type.clone(),
        region: hub.attributes.region.clone(),
        extension_type,
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Hub Details".bold());
            println!("{}", "─".repeat(60));
            println!("  {} {}", "Name:".bold(), output.name.cyan());
            println!("  {} {}", "ID:".bold(), output.id);
            println!("  {} {}", "Type:".bold(), output.hub_type);

            if let Some(ref region) = output.region {
                println!("  {} {}", "Region:".bold(), region);
            }

            if let Some(ref ext_type) = output.extension_type {
                println!("  {} {}", "Extension:".bold(), ext_type);
            }

            println!("{}", "─".repeat(60));
            println!(
                "\n{}",
                "Use 'raps project list <hub-id>' to see projects".dimmed()
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }
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

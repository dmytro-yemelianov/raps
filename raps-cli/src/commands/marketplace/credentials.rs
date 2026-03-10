// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Credential management command handlers: set, list, delete

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;

use crate::marketplace::client::MarketplaceClient;
use crate::marketplace::MarketplaceAuth;
use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum CredentialCommands {
    /// Store APS credentials for hosted MCP
    Set {
        /// APS Client ID
        #[arg(long)]
        client_id: String,
        /// APS Client Secret
        #[arg(long)]
        client_secret: String,
        /// Credential label (default: "default")
        #[arg(long, default_value = "default")]
        label: String,
    },
    /// List stored credential labels
    List,
    /// Delete stored credentials
    Delete {
        /// Label to delete
        label: String,
    },
}

pub(super) async fn execute(cmd: CredentialCommands, output_format: OutputFormat) -> Result<()> {
    let key = MarketplaceAuth::get_license_key()?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No license key found. Run `raps marketplace license <key>` first."
            )
        })?;

    let client = MarketplaceClient::new()?;
    match cmd {
        CredentialCommands::Set {
            client_id,
            client_secret,
            label,
        } => {
            client
                .store_credentials(&key, &client_id, &client_secret, &label)
                .await?;
            match output_format {
                OutputFormat::Table => {
                    println!(
                        "{} Credentials stored with label '{}'",
                        "✓".green().bold(),
                        label.cyan()
                    );
                }
                _ => {
                    output_format.write(&serde_json::json!({
                        "stored": true,
                        "label": label,
                    }))?;
                }
            }
        }
        CredentialCommands::List => {
            let creds = client.list_credentials(&key).await?;
            match output_format {
                OutputFormat::Table => {
                    if let Some(arr) = creds.as_array() {
                        if arr.is_empty() {
                            println!("{}", "No credentials stored.".yellow());
                        } else {
                            println!("\n{}", "Stored Credentials:".bold());
                            println!("{}", "─".repeat(40));
                            for item in arr {
                                if let Some(label) = item.get("label").and_then(|v| v.as_str()) {
                                    println!("  • {}", label.cyan());
                                }
                            }
                            println!("{}", "─".repeat(40));
                        }
                    } else {
                        output_format.write(&creds)?;
                    }
                }
                _ => {
                    output_format.write(&creds)?;
                }
            }
        }
        CredentialCommands::Delete { label } => {
            client.delete_credentials(&key, &label).await?;
            match output_format {
                OutputFormat::Table => {
                    println!(
                        "{} Credentials '{}' deleted",
                        "✓".green().bold(),
                        label.cyan()
                    );
                }
                _ => {
                    output_format.write(&serde_json::json!({
                        "deleted": true,
                        "label": label,
                    }))?;
                }
            }
        }
    }
    Ok(())
}

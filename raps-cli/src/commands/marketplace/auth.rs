// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Auth command handlers: login, logout, status, license

use anyhow::Result;
use colored::Colorize;

use crate::marketplace::{MarketplaceAuth, SubscriptionManager};
use crate::output::OutputFormat;

use super::LicenseArgs;

pub(super) async fn login(_output_format: OutputFormat) -> Result<()> {
    anyhow::bail!(
        "Marketplace authentication uses a license key.\n\
         Run `raps marketplace license <key>` to store your license key."
    )
}

pub(super) async fn logout(output_format: OutputFormat) -> Result<()> {
    MarketplaceAuth::clear_tokens()?;
    SubscriptionManager::clear_cache();

    match output_format {
        OutputFormat::Table => {
            println!("{} Logged out successfully", "✓".green().bold());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "logged_out": true
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn status(output_format: OutputFormat) -> Result<()> {
    if !MarketplaceAuth::is_authenticated() {
        match output_format {
            OutputFormat::Table => {
                println!("{}", "Not logged in.".yellow());
                println!(
                    "Run {} to authenticate.",
                    "raps marketplace license <key>".cyan()
                );
            }
            _ => {
                output_format.write(&serde_json::json!({
                    "authenticated": false
                }))?;
            }
        }
        return Ok(());
    }

    match SubscriptionManager::get_subscription() {
        Some(subscription) => {
            match output_format {
                OutputFormat::Table => {
                    println!("\n{}", "Subscription Status:".bold());
                    println!("{}", "─".repeat(40));
                    println!(
                        "{}",
                        SubscriptionManager::format_subscription_status(&subscription)
                    );
                    println!("{}", "─".repeat(40));
                }
                _ => {
                    output_format.write(&subscription)?;
                }
            }
        }
        None => {
            match output_format {
                OutputFormat::Table => {
                    println!("{}", "License key stored but not yet validated.".yellow());
                    println!(
                        "Run {} to validate.",
                        "raps marketplace license <key>".cyan()
                    );
                }
                _ => {
                    output_format.write(&serde_json::json!({
                        "authenticated": true,
                        "validated": false
                    }))?;
                }
            }
        }
    }

    Ok(())
}

pub(super) async fn license(args: LicenseArgs, output_format: OutputFormat) -> Result<()> {
    // Validate first (before storing), then store on success
    let subscription = SubscriptionManager::register_license(&args.key).await?;

    match output_format {
        OutputFormat::Table => {
            println!("{} License registered!", "✓".green().bold());
            println!(
                "{}",
                SubscriptionManager::format_subscription_status(&subscription)
            );
            if !subscription.plugins.is_empty() {
                println!("\n{}", "Entitled plugins:".bold());
                for plugin in &subscription.plugins {
                    println!("  • {}", plugin.as_str().cyan());
                }
            }
        }
        _ => {
            output_format.write(&subscription)?;
        }
    }

    Ok(())
}

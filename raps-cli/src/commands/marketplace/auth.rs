// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Auth command handlers: login, logout, status, license

use anyhow::{Context, Result};
use colored::Colorize;

use crate::marketplace::{MarketplaceAuth, SubscriptionManager};
use crate::output::OutputFormat;

use super::LicenseArgs;

pub(super) async fn login(output_format: OutputFormat) -> Result<()> {
    let auth = MarketplaceAuth::new();
    let token_response = auth.login().await?;

    match output_format {
        OutputFormat::Table => {
            println!("{} Logged in successfully!", "✓".green().bold());
            println!(
                "  {} expires in {} seconds",
                "Token".dimmed(),
                token_response.expires_in
            );
        }
        _ => {
            output_format.write(&serde_json::json!({
                "logged_in": true,
                "expires_in": token_response.expires_in
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn logout(output_format: OutputFormat) -> Result<()> {
    let auth = MarketplaceAuth::new();
    auth.clear_tokens().await?;

    let sub_manager = SubscriptionManager::new()?;
    sub_manager.clear_cache().await?;

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
    let auth = MarketplaceAuth::new();
    auth.load_tokens().await?;

    if !auth.is_authenticated().await {
        match output_format {
            OutputFormat::Table => {
                println!("{}", "Not logged in.".yellow());
                println!("Run {} to authenticate.", "raps marketplace login".cyan());
            }
            _ => {
                output_format.write(&serde_json::json!({
                    "authenticated": false
                }))?;
            }
        }
        return Ok(());
    }

    let sub_manager = SubscriptionManager::new()?;
    let token: String = auth.get_access_token().await.context("Not authenticated. Run 'raps auth login' first.")?;
    let subscription = sub_manager.get_subscription(&token).await?;

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

    Ok(())
}

pub(super) async fn license(args: LicenseArgs, output_format: OutputFormat) -> Result<()> {
    let auth = MarketplaceAuth::new();
    auth.load_tokens().await?;

    if !auth.is_authenticated().await {
        anyhow::bail!("Please login first: raps marketplace login");
    }

    let sub_manager = SubscriptionManager::new()?;
    let token: String = auth.get_access_token().await.context("Not authenticated. Run 'raps auth login' first.")?;

    let subscription = sub_manager.register_license(&token, &args.key).await?;

    match output_format {
        OutputFormat::Table => {
            println!("{} License registered!", "✓".green().bold());
            println!(
                "{}",
                SubscriptionManager::format_subscription_status(&subscription)
            );
        }
        _ => {
            output_format.write(&subscription)?;
        }
    }

    Ok(())
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Install command handlers: install, uninstall, update

use anyhow::{Context, Result};
use colored::Colorize;

use crate::marketplace::{MarketplaceAuth, MarketplaceClient, PluginInstaller, SubscriptionManager};
use crate::output::OutputFormat;
use raps_kernel::marketplace::{Installation, Plugin, PluginTier};

use super::{InstallArgs, UninstallArgs, UpdateArgs};

pub(super) async fn install(args: InstallArgs, output_format: OutputFormat) -> Result<()> {
    let client = MarketplaceClient::new();

    // Check if Pro plugin requires authentication
    let plugin: Plugin = client.get_plugin(&args.name).await?;
    if plugin.tier == PluginTier::Pro {
        let auth = MarketplaceAuth::new();
        auth.load_tokens().await?;

        if !auth.is_authenticated().await {
            anyhow::bail!(
                "Plugin '{}' requires a Pro subscription. Run 'raps marketplace login' first.",
                args.name
            );
        }

        let sub_manager = SubscriptionManager::new()?;
        let token: String = auth.get_access_token().await.context("Not authenticated. Run 'raps auth login' first.")?;

        if !sub_manager.can_use_pro(&token).await? {
            anyhow::bail!(
                "Plugin '{}' requires a Pro subscription.\n\
                 Subscribe at: https://marketplace.rapscli.xyz/subscribe",
                args.name
            );
        }
    }

    let installer = PluginInstaller::new(client)?;
    let result = installer
        .install(&args.name, args.version.as_deref())
        .await?;

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Installed {} v{}",
                "✓".green().bold(),
                result.name.cyan(),
                result.version
            );
            println!("  {} {}", "Binary:".dimmed(), result.binary_path.display());

            if result.suggest_path {
                println!();
                println!("{}", "Note:".yellow().bold());
                println!("{}", installer.path_suggestion());
            }
        }
        _ => {
            output_format.write(&serde_json::json!({
                "name": result.name,
                "version": result.version,
                "path": result.binary_path.to_string_lossy(),
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn uninstall(args: UninstallArgs, output_format: OutputFormat) -> Result<()> {
    let client = MarketplaceClient::new();
    let installer = PluginInstaller::new(client)?;

    installer.uninstall(&args.name).await?;

    match output_format {
        OutputFormat::Table => {
            println!("{} Uninstalled {}", "✓".green().bold(), args.name.cyan());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "name": args.name,
                "uninstalled": true
            }))?;
        }
    }

    Ok(())
}

/// Update info with changelog
struct UpdateInfo {
    name: String,
    current_version: String,
    available_version: String,
    changelog: Option<String>,
    is_pro: bool,
    raps_compatible: bool,
}

pub(super) async fn update(args: UpdateArgs, output_format: OutputFormat) -> Result<()> {
    let client = MarketplaceClient::new();
    let installer = PluginInstaller::new(client.clone())?;
    let installations: Vec<Installation> = installer.load_registry().await?;

    if installations.is_empty() {
        match output_format {
            OutputFormat::Table => {
                println!("{}", "No marketplace plugins installed.".yellow());
            }
            _ => {
                output_format.write(&serde_json::json!({
                    "updates": []
                }))?;
            }
        }
        return Ok(());
    }

    // Filter installations based on args
    let to_check: Vec<Installation> = if let Some(ref name) = args.name {
        installations
            .into_iter()
            .filter(|i| &i.name == name)
            .collect()
    } else {
        // For --all, --check, or default: check all
        installations
    };

    let mut updates_available: Vec<UpdateInfo> = Vec::new();

    for install in &to_check {
        let Ok(versions) = client.get_versions(&install.name).await else {
            continue;
        };

        let Some(latest) = versions.iter().filter(|v| !v.yanked).max_by(|a, b| {
            semver::Version::parse(&a.version)
                .unwrap_or_else(|_| semver::Version::new(0, 0, 0))
                .cmp(
                    &semver::Version::parse(&b.version)
                        .unwrap_or_else(|_| semver::Version::new(0, 0, 0)),
                )
        }) else {
            continue;
        };

        let current = semver::Version::parse(&install.version)
            .unwrap_or_else(|_| semver::Version::new(0, 0, 0));
        let available = semver::Version::parse(&latest.version)
            .unwrap_or_else(|_| semver::Version::new(0, 0, 0));

        if available > current {
            // Check RAPS version compatibility
            let raps_compatible =
                PluginInstaller::check_raps_compatibility(&latest.raps_compatibility)
                    .unwrap_or(false);

            // Check if it's a Pro plugin
            let is_pro = client
                .get_plugin(&install.name)
                .await
                .map(|p| p.tier == PluginTier::Pro)
                .unwrap_or(false);

            updates_available.push(UpdateInfo {
                name: install.name.clone(),
                current_version: install.version.clone(),
                available_version: latest.version.clone(),
                changelog: latest.changelog.clone(),
                is_pro,
                raps_compatible,
            });
        }
    }

    if args.check || (!args.all && args.name.is_none()) {
        display_available_updates(&updates_available, output_format)?;
    } else {
        perform_updates(updates_available, &installer, &args, output_format).await?;
    }

    Ok(())
}

fn display_available_updates(
    updates_available: &[UpdateInfo],
    output_format: OutputFormat,
) -> Result<()> {
    match output_format {
        OutputFormat::Table => {
            if updates_available.is_empty() {
                println!("{}", "All plugins are up to date.".green());
            } else {
                println!("\n{}", "Updates Available:".bold());
                println!("{}", "─".repeat(80));
                for update in updates_available {
                    let tier_badge = if update.is_pro {
                        " [PRO]".magenta().to_string()
                    } else {
                        String::new()
                    };
                    let compat_warning = if !update.raps_compatible {
                        " ⚠ incompatible".yellow().to_string()
                    } else {
                        String::new()
                    };
                    println!(
                        "  {}{} {} → {}{}",
                        update.name.cyan(),
                        tier_badge,
                        update.current_version.dimmed(),
                        update.available_version.green(),
                        compat_warning
                    );
                    if let Some(ref changelog) = update.changelog {
                        // Show first line of changelog
                        let first_line = changelog.lines().next().unwrap_or("");
                        if !first_line.is_empty() {
                            println!("    {}", first_line.dimmed());
                        }
                    }
                }
                println!("{}", "─".repeat(80));
                println!(
                    "\nRun {} to update all, or {} to update one.",
                    "raps marketplace update --all".cyan(),
                    "raps marketplace update <name>".cyan()
                );
            }
        }
        _ => {
            output_format.write(&serde_json::json!({
                "updates": updates_available.iter().map(|u| {
                    serde_json::json!({
                        "name": u.name,
                        "current": u.current_version,
                        "available": u.available_version,
                        "changelog": u.changelog,
                        "is_pro": u.is_pro,
                        "raps_compatible": u.raps_compatible
                    })
                }).collect::<Vec<_>>()
            }))?;
        }
    }

    Ok(())
}

async fn perform_updates(
    updates_available: Vec<UpdateInfo>,
    installer: &PluginInstaller,
    args: &UpdateArgs,
    output_format: OutputFormat,
) -> Result<()> {
    // Check Pro subscription if any Pro plugins need updating
    let has_pro_updates = updates_available.iter().any(|u| u.is_pro);
    let mut can_update_pro = false;

    if has_pro_updates {
        let auth = MarketplaceAuth::new();
        auth.load_tokens().await?;

        if auth.is_authenticated().await {
            let sub_manager = SubscriptionManager::new()?;
            let token = auth.get_access_token().await.context("Not authenticated. Run 'raps auth login' first.")?;
            can_update_pro = sub_manager.can_update_pro(&token).await.unwrap_or(false);
        }
    }

    // Perform updates with rollback support
    let mut success_count = 0;
    let mut fail_count = 0;
    let mut skipped_count = 0;

    for update in updates_available {
        // Check RAPS compatibility
        if !update.raps_compatible {
            if let OutputFormat::Table = output_format {
                println!(
                    "{} Skipping {} - incompatible with current RAPS version",
                    "⚠".yellow().bold(),
                    update.name.cyan()
                );
            }
            skipped_count += 1;
            continue;
        }

        // Check Pro subscription for Pro plugins
        if update.is_pro && !can_update_pro {
            if let OutputFormat::Table = output_format {
                println!(
                    "{} Skipping {} - Pro subscription required for updates",
                    "⚠".yellow().bold(),
                    update.name.cyan()
                );
                println!(
                    "    Subscribe at: {}",
                    "https://marketplace.rapscli.xyz/subscribe".cyan()
                );
            }
            skipped_count += 1;
            continue;
        }

        // Perform update with rollback support
        match installer.update_with_rollback(&update.name, None).await {
            Ok(result) => {
                if let OutputFormat::Table = output_format {
                    println!(
                        "{} Updated {} to v{}",
                        "✓".green().bold(),
                        result.name.cyan(),
                        result.version
                    );
                }
                success_count += 1;
            }
            Err(e) => {
                if let OutputFormat::Table = output_format {
                    println!(
                        "{} Failed to update {}: {}",
                        "✗".red().bold(),
                        update.name,
                        e
                    );
                }
                fail_count += 1;
            }
        }
    }

    // Summary for bulk updates
    if args.all && matches!(output_format, OutputFormat::Table) {
        println!("{}", "─".repeat(60));
        println!(
            "{} {} updated, {} failed, {} skipped",
            "Summary:".bold(),
            success_count.to_string().green(),
            fail_count.to_string().red(),
            skipped_count.to_string().yellow()
        );
    }

    Ok(())
}

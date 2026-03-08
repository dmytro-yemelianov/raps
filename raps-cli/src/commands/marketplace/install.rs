// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Install command handlers: install, uninstall, update

use anyhow::Result;
use colored::Colorize;

use crate::marketplace::PluginInstaller;
use crate::output::OutputFormat;

use super::{InstallArgs, UninstallArgs, UpdateArgs};

pub(super) async fn install(args: InstallArgs, output_format: OutputFormat) -> Result<()> {
    let installer = PluginInstaller::new();
    let result = installer.install(&args.name).await?;

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Installed {} v{}",
                "✓".green().bold(),
                result.slug.cyan(),
                result.version
            );
            println!("  {} {}", "Path:".dimmed(), result.install_path);
        }
        _ => {
            output_format.write(&serde_json::json!({
                "slug": result.slug,
                "version": result.version,
                "path": result.install_path,
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn uninstall(args: UninstallArgs, output_format: OutputFormat) -> Result<()> {
    let installer = PluginInstaller::new();
    installer.uninstall(&args.name)?;

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

pub(super) async fn update(args: UpdateArgs, output_format: OutputFormat) -> Result<()> {
    let installer = PluginInstaller::new();
    let slugs = installer.load_registry();

    if slugs.is_empty() {
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

    // Filter based on args
    let to_update: Vec<String> = if let Some(ref name) = args.name {
        slugs.into_iter().filter(|s| s == name).collect()
    } else if args.all || (!args.check && args.name.is_none()) {
        slugs
    } else {
        // --check mode: just list installed plugins
        match output_format {
            OutputFormat::Table => {
                println!("{}", "Installed marketplace plugins:".bold());
                for slug in &slugs {
                    println!("  • {}", slug.as_str().cyan());
                }
                println!(
                    "\nRun {} to update all.",
                    "raps marketplace update --all".cyan()
                );
            }
            _ => {
                output_format.write(&serde_json::json!({
                    "installed": slugs
                }))?;
            }
        }
        return Ok(());
    };

    let mut success_count = 0;
    let mut fail_count = 0;

    for slug in &to_update {
        match installer.update_with_rollback(slug).await {
            Ok(result) => {
                if let OutputFormat::Table = output_format {
                    println!(
                        "{} Updated {} to v{}",
                        "✓".green().bold(),
                        result.slug.cyan(),
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
                        slug,
                        e
                    );
                }
                fail_count += 1;
            }
        }
    }

    if args.all && matches!(output_format, OutputFormat::Table) {
        println!("{}", "─".repeat(60));
        println!(
            "{} {} updated, {} failed",
            "Summary:".bold(),
            success_count.to_string().green(),
            fail_count.to_string().red(),
        );
    }

    Ok(())
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Discovery command handlers: search, clear-cache

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use super::SearchArgs;
use crate::output::OutputFormat;

#[derive(Serialize, schemars::JsonSchema)]
struct PluginDiscovery {
    name: String,
    slug: String,
    tier: String,
    category: String,
    description: String,
    price: String,
}

pub(super) async fn search(args: SearchArgs, output_format: OutputFormat) -> Result<()> {
    // In a real implementation, this would call the Marketplace API
    // For now, we return a static list including the newly moved Dashboard
    let mut plugins = vec![
        PluginDiscovery {
            name: "TUI Dashboard".into(),
            slug: "dashboard".into(),
            tier: "pro".into(),
            category: "Observability".into(),
            description: "Full-featured keyboard-driven terminal dashboard for APS resources."
                .into(),
            price: "$10/mo".into(),
        },
        PluginDiscovery {
            name: "ACC Bulk Manager".into(),
            slug: "acc-bulk".into(),
            tier: "pro".into(),
            category: "Automation".into(),
            description:
                "Bulk operations for Autodesk Construction Cloud (users, projects, assets).".into(),
            price: "$15/mo".into(),
        },
    ];

    // Simple filtering for demonstration
    if !args.query.is_empty() {
        let q = args.query.to_lowercase();
        plugins.retain(|p| {
            p.name.to_lowercase().contains(&q) || p.description.to_lowercase().contains(&q)
        });
    }
    if let Some(ref tier) = args.tier {
        plugins.retain(|p| p.tier == *tier);
    }

    match output_format {
        OutputFormat::Table => {
            if plugins.is_empty() {
                println!(
                    "{}",
                    "No plugins matching your criteria were found.".yellow()
                );
            } else {
                println!("\n{}", "Marketplace Search Results:".bold());
                println!("{}", "─".repeat(90));
                println!(
                    "  {:<20} {:<15} {:<10} {:<10} {}",
                    "Name".bold(),
                    "Slug".bold(),
                    "Tier".bold(),
                    "Price".bold(),
                    "Description".bold()
                );
                println!("{}", "─".repeat(90));

                for p in &plugins {
                    let tier_color = if p.tier == "pro" {
                        p.tier.magenta()
                    } else {
                        p.tier.green()
                    };
                    println!(
                        "  {:<20} {:<15} {:<10} {:<10} {}",
                        p.name.cyan(),
                        p.slug.dimmed(),
                        tier_color,
                        p.price.yellow(),
                        super::truncate_str(&p.description, 35)
                    );
                }
                println!("{}", "─".repeat(90));
                println!(
                    "Run {} to install a plugin.",
                    "raps marketplace install <slug>".cyan()
                );
            }
        }
        _ => {
            output_format.write(&plugins)?;
        }
    }

    Ok(())
}

pub(super) async fn clear_cache(output_format: OutputFormat) -> Result<()> {
    crate::marketplace::SubscriptionManager::clear_cache();

    match output_format {
        OutputFormat::Table => {
            println!("{} Marketplace cache cleared.", "✓".green().bold());
        }
        _ => {
            output_format.write(&serde_json::json!({ "cleared": true }))?;
        }
    }
    Ok(())
}

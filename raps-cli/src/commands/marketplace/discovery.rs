// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Discovery command handlers: search, clear_cache

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::marketplace::MarketplaceClient;
use crate::output::OutputFormat;

use super::SearchArgs;
use super::truncate_str;

#[derive(Serialize, schemars::JsonSchema)]
struct PluginSearchOutput {
    slug: String,
    name: String,
    description: String,
    price_monthly_cents: u32,
    price_yearly_cents: u32,
    latest_version: Option<String>,
}

pub(super) async fn search(args: SearchArgs, output_format: OutputFormat) -> Result<()> {
    let client = MarketplaceClient::new()?;
    let mut plugins: Vec<raps_kernel::marketplace::Plugin> = client.list_plugins().await?;

    // Filter by query string (name/description match)
    if !args.query.is_empty() {
        let q = args.query.to_lowercase();
        plugins.retain(|p| {
            p.name.to_lowercase().contains(&q)
                || p.description.to_lowercase().contains(&q)
                || p.slug.to_lowercase().contains(&q)
        });
    }

    // Only show published plugins
    plugins.retain(|p| p.published);

    let outputs: Vec<PluginSearchOutput> = plugins
        .iter()
        .map(|p| PluginSearchOutput {
            slug: p.slug.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            price_monthly_cents: p.price_monthly_cents,
            price_yearly_cents: p.price_yearly_cents,
            latest_version: p.latest_version.clone(),
        })
        .collect();

    match output_format {
        OutputFormat::Table => {
            if outputs.is_empty() {
                println!("{}", "No plugins found.".yellow());
            } else {
                println!("\n{}", "Marketplace Plugins:".bold());
                println!("{}", "─".repeat(100));
                println!(
                    "  {:<25} {:<15} {:<10} {}",
                    "Slug".bold(),
                    "Version".bold(),
                    "Price/mo".bold(),
                    "Description".bold()
                );
                println!("{}", "─".repeat(100));

                for plugin in &outputs {
                    let version_display = plugin
                        .latest_version
                        .as_deref()
                        .unwrap_or("-");
                    let price_display = if plugin.price_monthly_cents == 0 {
                        "free".green().to_string()
                    } else {
                        format!("${:.2}", plugin.price_monthly_cents as f32 / 100.0)
                            .magenta()
                            .to_string()
                    };

                    println!(
                        "  {:<25} {:<15} {:<10} {}",
                        plugin.slug.cyan(),
                        version_display,
                        price_display,
                        truncate_str(&plugin.description, 45)
                    );
                }

                println!("{}", "─".repeat(100));
                println!("{} {} plugin(s) found", "→".cyan(), outputs.len());
            }
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

pub(super) async fn clear_cache(output_format: OutputFormat) -> Result<()> {
    crate::marketplace::SubscriptionManager::clear_cache();

    match output_format {
        OutputFormat::Table => {
            println!("{} Marketplace cache cleared", "✓".green().bold());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "cache_cleared": true
            }))?;
        }
    }

    Ok(())
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Discovery command handlers: search, clear_cache

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::marketplace::{CacheManager, MarketplaceClient};
use crate::output::OutputFormat;
use raps_kernel::marketplace::Plugin;

use super::SearchArgs;
use super::truncate_str;

#[derive(Serialize)]
struct PluginSearchOutput {
    name: String,
    version: String,
    description: String,
    tier: String,
    rating: Option<f32>,
    downloads: u64,
}

pub(super) async fn search(args: SearchArgs, output_format: OutputFormat) -> Result<()> {
    let client = MarketplaceClient::new();
    let cache = CacheManager::new(client)?;

    let query = if args.query.is_empty() {
        None
    } else {
        Some(args.query.as_str())
    };

    let plugins: Vec<Plugin> = cache
        .search_cached(query, args.tier.as_deref(), args.category.as_deref())
        .await?;

    let outputs: Vec<PluginSearchOutput> = plugins
        .iter()
        .map(|p| PluginSearchOutput {
            name: p.name.clone(),
            version: p.version.clone(),
            description: p.description.clone(),
            tier: p.tier.to_string(),
            rating: p.rating,
            downloads: p.install_count,
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
                    "  {:<25} {:<10} {:<8} {:<6} {:<8} {}",
                    "Name".bold(),
                    "Version".bold(),
                    "Tier".bold(),
                    "Rating".bold(),
                    "Downloads".bold(),
                    "Description".bold()
                );
                println!("{}", "─".repeat(100));

                for plugin in &outputs {
                    let tier_display = match plugin.tier.as_str() {
                        "pro" => "pro".magenta().to_string(),
                        _ => "basic".green().to_string(),
                    };
                    let rating_display = plugin
                        .rating
                        .map(|r| format!("{:.1}★", r))
                        .unwrap_or_else(|| "-".to_string());

                    println!(
                        "  {:<25} {:<10} {:<8} {:<6} {:<8} {}",
                        plugin.name.cyan(),
                        plugin.version,
                        tier_display,
                        rating_display,
                        plugin.downloads,
                        truncate_str(&plugin.description, 35)
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
    let client = MarketplaceClient::new();
    let cache = CacheManager::new(client)?;
    cache.clear().await?;

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

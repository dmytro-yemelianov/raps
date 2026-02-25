// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Asset CRUD command handlers

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_acc::{AccClient, CreateAssetRequest, UpdateAssetRequest};

use super::truncate_str;

#[derive(Serialize)]
struct AssetOutput {
    id: String,
    category_id: Option<String>,
    description: Option<String>,
    barcode: Option<String>,
    updated_at: Option<String>,
}

pub(super) async fn list_assets(
    client: &AccClient,
    project_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching assets...".dimmed());
    }

    let assets = client.list_assets(project_id).await.context(format!(
        "Failed to list assets for project '{}'",
        project_id
    ))?;

    let outputs: Vec<AssetOutput> = assets
        .iter()
        .map(|a| AssetOutput {
            id: a.id.clone(),
            category_id: a.category_id.clone(),
            description: a.description.clone(),
            barcode: a.barcode.clone(),
            updated_at: a.updated_at.clone(),
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No assets found.".yellow()),
            _ => output_format.write(&Vec::<AssetOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Assets:".bold());
            println!("{}", "─".repeat(80));
            println!(
                "{:<40} {:<30} {}",
                "ID".bold(),
                "Description".bold(),
                "Barcode".bold()
            );
            println!("{}", "─".repeat(80));

            for asset in &outputs {
                let desc = asset.description.as_deref().unwrap_or("-");
                let barcode = asset.barcode.as_deref().unwrap_or("-");
                println!(
                    "{:<40} {:<30} {}",
                    truncate_str(&asset.id, 40).cyan(),
                    truncate_str(desc, 30),
                    barcode.dimmed()
                );
            }

            println!("{}", "─".repeat(80));
            println!("{} {} asset(s) found", "→".cyan(), outputs.len());
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

pub(super) async fn get_asset(
    client: &AccClient,
    project_id: &str,
    asset_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching asset details...".dimmed());
    }

    let asset = client
        .get_asset(project_id, asset_id)
        .await
        .context(format!(
            "Failed to get asset '{}'. Verify the asset ID exists",
            asset_id
        ))?;

    let output = AssetOutput {
        id: asset.id.clone(),
        category_id: asset.category_id.clone(),
        description: asset.description.clone(),
        barcode: asset.barcode.clone(),
        updated_at: asset.updated_at.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Asset Details:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "ID:".bold(), asset.id.cyan());
            println!(
                "{:<15} {}",
                "Category:".bold(),
                asset.category_id.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Description:".bold(),
                asset.description.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Barcode:".bold(),
                asset.barcode.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Updated:".bold(),
                asset.updated_at.as_deref().unwrap_or("-")
            );
            println!("{}", "─".repeat(60));
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

pub(super) async fn create_asset(
    client: &AccClient,
    project_id: &str,
    description: Option<String>,
    barcode: Option<String>,
    category_id: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Creating asset...".dimmed());
    }

    let request = CreateAssetRequest {
        description,
        barcode,
        category_id,
        client_asset_id: None,
    };

    let asset = client
        .create_asset(project_id, request)
        .await
        .context("Failed to create asset. Verify your permissions on this project")?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Asset created successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), asset.id.cyan());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "id": asset.id,
                "created": true
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn update_asset(
    client: &AccClient,
    project_id: &str,
    asset_id: &str,
    description: Option<String>,
    barcode: Option<String>,
    status_id: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Updating asset...".dimmed());
    }

    let request = UpdateAssetRequest {
        description,
        barcode,
        status_id,
        category_id: None,
    };

    let asset = client
        .update_asset(project_id, asset_id, request)
        .await
        .context(format!(
            "Failed to update asset '{}'. Check permissions",
            asset_id
        ))?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Asset updated successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), asset.id.cyan());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "id": asset.id,
                "updated": true
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn delete_asset(
    client: &AccClient,
    project_id: &str,
    asset_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Deleting asset...".dimmed());
    }

    client
        .delete_asset(project_id, asset_id)
        .await
        .context(format!(
            "Failed to delete asset '{}'. Verify the asset ID and your permissions",
            asset_id
        ))?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Asset deleted successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), asset_id.cyan());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "id": asset_id,
                "deleted": true
            }))?;
        }
    }

    Ok(())
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! AppBundle CRUD handlers for Design Automation.

use anyhow::Result;
use colored::Colorize;

use crate::output::OutputFormat;
use raps_da::DesignAutomationClient;
use raps_kernel::prompts;

pub(super) async fn list_appbundles(
    client: &DesignAutomationClient,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching app bundles...".dimmed());
    }

    let appbundles = client.list_appbundles().await?;

    if appbundles.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No app bundles found.".yellow()),
            _ => {
                output_format.write(&Vec::<String>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "App Bundles:".bold());
            println!("{}", "-".repeat(60));

            for bundle in &appbundles {
                println!("  {} {}", "-".cyan(), bundle);
            }

            println!("{}", "-".repeat(60));
        }
        _ => {
            output_format.write(&appbundles)?;
        }
    }
    Ok(())
}

pub(super) async fn create_appbundle(
    client: &DesignAutomationClient,
    id: Option<String>,
    engine: Option<String>,
    description: Option<String>,
    _output_format: OutputFormat,
) -> Result<()> {
    // Get engine first to help with ID suggestion
    let selected_engine = match engine {
        Some(e) => e,
        None => {
            println!("{}", "Fetching engines...".dimmed());
            let engines = client.list_engines().await?;

            let selection = prompts::select("Select engine", &engines)?;
            engines[selection].clone()
        }
    };

    // Get bundle ID
    let bundle_id = match id {
        Some(i) => i,
        None => prompts::input("Enter app bundle ID", None)?,
    };

    println!("{}", "Creating app bundle...".dimmed());

    let bundle = client
        .create_appbundle(&bundle_id, &selected_engine, description.as_deref())
        .await?;

    println!("{} App bundle created!", "\u{2713}".green().bold());
    println!("  {} {}", "ID:".bold(), bundle.id);
    println!("  {} {}", "Engine:".bold(), bundle.engine.cyan());
    println!("  {} {}", "Version:".bold(), bundle.version);

    // Auto-create a "default" alias so the bundle can be referenced
    match client
        .create_appbundle_alias(&bundle_id, "default", bundle.version)
        .await
    {
        Ok(()) => {
            println!(
                "  {} Alias '{}' created",
                "\u{2713}".green(),
                "default".cyan()
            );
        }
        Err(e) => {
            println!("  {} Could not create alias: {}", "!".yellow(), e);
        }
    }

    if let Some(upload) = bundle.upload_parameters
        && let Some(ref url) = upload.endpoint_url
    {
        println!("\n{}", "Upload your bundle ZIP to:".yellow());
        println!("  {}", url);
    }

    Ok(())
}

pub(super) async fn delete_appbundle(
    client: &DesignAutomationClient,
    id: &str,
    _output_format: OutputFormat,
) -> Result<()> {
    println!("{}", "Deleting app bundle...".dimmed());

    client.delete_appbundle(id).await?;

    println!("{} App bundle '{}' deleted!", "\u{2713}".green().bold(), id);
    Ok(())
}

pub(super) async fn upload_appbundle(
    client: &DesignAutomationClient,
    id: &str,
    file: &std::path::Path,
    engine: &str,
    description: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    // Step 1: Create a new version of the app bundle to get upload parameters
    if output_format.supports_colors() {
        println!(
            "{} Creating new version of app bundle '{}'...",
            "\u{2192}".cyan(),
            id
        );
    }

    let bundle_details = client
        .create_appbundle(id, engine, description.as_deref())
        .await?;

    let upload_params = bundle_details
        .upload_parameters
        .ok_or_else(|| anyhow::anyhow!("No upload parameters returned for app bundle '{}'", id))?;

    if output_format.supports_colors() {
        println!(
            "{} Version {} created. Uploading archive...",
            "\u{2713}".green().bold(),
            bundle_details.version
        );
    }

    // Step 2: Upload the archive
    client.upload_appbundle(&upload_params, file).await?;

    if output_format.supports_colors() {
        println!(
            "{} Archive '{}' uploaded to app bundle '{}' (version {})",
            "\u{2713}".green().bold(),
            file.display(),
            id,
            bundle_details.version
        );
    } else {
        #[derive(serde::Serialize)]
        struct UploadResult {
            id: String,
            version: i32,
            file: String,
        }
        output_format.write(&UploadResult {
            id: id.to_string(),
            version: bundle_details.version,
            file: file.display().to_string(),
        })?;
    }

    Ok(())
}

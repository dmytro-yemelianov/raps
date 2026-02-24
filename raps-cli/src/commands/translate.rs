// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Translation commands for Model Derivative API
//!
//! Commands for starting translations, checking status, viewing manifests, and downloading derivatives.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;
use std::time::Duration;
use std::{path::PathBuf, str::FromStr};

use crate::output::OutputFormat;
use raps_derivative::{
    DerivativeClient, MdRegion, OutputFormat as DerivativeOutputFormat, PropertyQuery,
};
// use raps_kernel::output::OutputFormat;
use raps_kernel::{progress, prompts};

/// Client-side polling timeout for translations (2 hours).
/// Translations typically complete within minutes, but complex models can take longer.
const TRANSLATE_POLL_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);

#[derive(Debug, Subcommand)]
pub enum TranslateCommands {
    /// Start a translation job
    Start {
        /// Base64-encoded URN of the source file
        urn: Option<String>,

        /// Output format (svf2, svf, obj, stl, step, iges, ifc)
        #[arg(short, long)]
        format: Option<String>,

        /// Root filename (for ZIP files with multiple design files)
        #[arg(short, long)]
        root_filename: Option<String>,

        /// Wait for translation to complete (polls until done)
        #[arg(short, long)]
        wait: bool,

        /// APS data center region (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR)
        #[arg(long, default_value = "US")]
        region: String,

        /// Force re-translation by deleting existing manifest.
        /// Note: Prior versions always forced re-translation. The default is now
        /// to preserve existing manifests.
        #[arg(long)]
        force: bool,
    },

    /// Check translation status
    Status {
        /// Base64-encoded URN of the source file
        urn: String,

        /// Wait for translation to complete
        #[arg(short, long)]
        wait: bool,
    },

    /// Get translation manifest (available derivatives)
    Manifest {
        /// Base64-encoded URN of the source file
        urn: String,
    },

    /// List downloadable derivatives
    Derivatives {
        /// Base64-encoded URN of the source file
        urn: String,

        /// Filter by format (obj, stl, step, etc.)
        #[arg(short, long)]
        format: Option<String>,
    },

    /// Download translated derivatives
    Download {
        /// Base64-encoded URN of the source file
        urn: String,

        /// Download by format (obj, stl, step, etc.) - downloads all matching
        #[arg(short, long)]
        format: Option<String>,

        /// Download specific derivative by GUID
        #[arg(short, long)]
        guid: Option<String>,

        /// Output directory (defaults to current directory)
        #[arg(long = "out-dir")]
        out_dir: Option<PathBuf>,

        /// Download all available derivatives
        #[arg(short, long)]
        all: bool,
    },

    /// List model views/viewables (metadata)
    Metadata {
        /// Base64-encoded URN of the source file
        urn: String,

        /// APS data center region (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR)
        #[arg(long)]
        region: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Get object tree hierarchy for a model view
    Tree {
        /// Base64-encoded URN of the source file
        urn: String,

        /// Model view GUID (from metadata command)
        guid: String,

        /// APS data center region (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR)
        #[arg(long)]
        region: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Get properties for a model view
    Properties {
        /// Base64-encoded URN of the source file
        urn: String,

        /// Model view GUID (from metadata command)
        guid: String,

        /// Filter by object ID
        #[arg(long = "object-id")]
        object_id: Option<i64>,

        /// APS data center region (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR)
        #[arg(long)]
        region: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Query specific properties by object IDs
    #[command(name = "query-properties")]
    QueryProperties {
        /// Base64-encoded URN of the source file
        urn: String,

        /// Model view GUID (from metadata command)
        guid: String,

        /// Comma-separated object IDs to filter (e.g., "1,2,3")
        #[arg(short, long)]
        filter: String,

        /// Comma-separated field names to return
        #[arg(long)]
        fields: Option<String>,

        /// APS data center region (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR)
        #[arg(long)]
        region: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Manage translation presets
    #[command(subcommand)]
    Preset(PresetCommands),
}

#[derive(Debug, Subcommand)]
pub enum PresetCommands {
    /// List available translation presets
    List,

    /// Show preset details
    Show {
        /// Preset name
        name: String,
    },

    /// Create a new preset
    Create {
        /// Preset name
        name: String,
        /// Output format (svf2, obj, stl, step, etc.)
        #[arg(short, long)]
        format: String,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Delete a preset
    Delete {
        /// Preset name
        name: String,
    },

    /// Use a preset to start translation
    Use {
        /// Base64-encoded URN of the source file
        urn: String,
        /// Preset name
        preset: String,
    },
}

impl TranslateCommands {
    pub async fn execute(
        self,
        client: &DerivativeClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            TranslateCommands::Start {
                urn,
                format,
                root_filename,
                wait,
                region,
                force,
            } => {
                start_translation(
                    client,
                    urn,
                    format,
                    root_filename,
                    wait,
                    output_format,
                    region,
                    force,
                )
                .await
            }
            TranslateCommands::Status { urn, wait } => {
                check_status(client, &urn, wait, output_format).await
            }
            TranslateCommands::Manifest { urn } => show_manifest(client, &urn, output_format).await,
            TranslateCommands::Derivatives { urn, format } => {
                list_derivatives(client, &urn, format, output_format).await
            }
            TranslateCommands::Download {
                urn,
                format,
                guid,
                out_dir,
                all,
            } => {
                download_derivatives(client, &urn, format, guid, out_dir, all, output_format).await
            }
            TranslateCommands::Metadata {
                urn,
                region,
                output,
            } => show_metadata(client, &urn, region, &output).await,
            TranslateCommands::Tree {
                urn,
                guid,
                region,
                output,
            } => show_object_tree(client, &urn, &guid, region, &output).await,
            TranslateCommands::Properties {
                urn,
                guid,
                object_id,
                region,
                output,
            } => show_properties(client, &urn, &guid, object_id, region, &output).await,
            TranslateCommands::QueryProperties {
                urn,
                guid,
                filter,
                fields,
                region,
                output,
            } => query_properties(client, &urn, &guid, &filter, fields, region, &output).await,
            TranslateCommands::Preset(cmd) => cmd.execute(client, output_format).await,
        }
    }
}

impl PresetCommands {
    pub async fn execute(
        self,
        client: &DerivativeClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            PresetCommands::List => list_presets(output_format),
            PresetCommands::Show { name } => show_preset(&name, output_format),
            PresetCommands::Create {
                name,
                format,
                description,
            } => create_preset(&name, &format, description, output_format),
            PresetCommands::Delete { name } => delete_preset(&name, output_format),
            PresetCommands::Use { urn, preset } => {
                use_preset(client, &urn, &preset, output_format).await
            }
        }
    }
}

#[derive(Serialize)]
struct TranslationStartOutput {
    success: bool,
    result: String,
    urn: String,
    accepted_formats: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
async fn start_translation(
    client: &DerivativeClient,
    urn: Option<String>,
    format: Option<String>,
    root_filename: Option<String>,
    wait: bool,
    output_format: OutputFormat,
    region_str: String,
    force: bool,
) -> Result<()> {
    let region: raps_derivative::MdRegion = region_str.parse().context("Invalid --region value")?;
    // Get URN interactively if not provided
    let source_urn = match urn {
        Some(u) => u,
        None => {
            prompts::input_validated("Enter the base64-encoded URN", None, |input: &String| {
                if input.is_empty() {
                    Err("URN cannot be empty")
                } else {
                    Ok(())
                }
            })?
        }
    };

    // Select output format interactively if not provided
    let derivative_format = match format {
        Some(f) => match f.to_lowercase().as_str() {
            "svf2" => DerivativeOutputFormat::Svf2,
            "svf" => DerivativeOutputFormat::Svf,
            "thumbnail" => DerivativeOutputFormat::Thumbnail,
            "obj" => DerivativeOutputFormat::Obj,
            "stl" => DerivativeOutputFormat::Stl,
            "step" => DerivativeOutputFormat::Step,
            "iges" => DerivativeOutputFormat::Iges,
            "ifc" => DerivativeOutputFormat::Ifc,
            _ => anyhow::bail!(
                "Invalid format. Use: svf2, svf, thumbnail, obj, stl, step, iges, ifc"
            ),
        },
        None => {
            // Interactive mode: prompt for format
            let formats = DerivativeOutputFormat::all();
            let format_labels: Vec<String> = formats.iter().map(|f| f.to_string()).collect();

            let selection = prompts::select("Select output format", &format_labels)?;
            formats[selection]
        }
    };

    if output_format.supports_colors() {
        println!(
            "{} {} {} {}",
            "Starting translation".dimmed(),
            "->".dimmed(),
            derivative_format.to_string().cyan(),
            "format".dimmed()
        );
    }

    let response = client
        .translate(
            &source_urn,
            derivative_format,
            root_filename.as_deref(),
            region,
            force,
        )
        .await?;

    let accepted_formats: Vec<String> = response
        .accepted_jobs
        .as_ref()
        .map(|jobs| {
            jobs.output
                .formats
                .iter()
                .map(|f| f.format_type.clone())
                .collect()
        })
        .unwrap_or_default();

    let output = TranslationStartOutput {
        success: true,
        result: response.result.clone(),
        urn: response.urn.clone(),
        accepted_formats,
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Translation job started!", "✓".green().bold());
            println!("  {} {}", "Result:".bold(), output.result);
            println!("  {} {}", "URN:".bold(), output.urn);

            if !output.accepted_formats.is_empty() {
                println!("  {} ", "Accepted formats:".bold());
                for format in &output.accepted_formats {
                    println!("    {} {}", "-".dimmed(), format.cyan());
                }
            }

            if !wait {
                println!(
                    "\n{}",
                    "Tip: Use 'raps translate status <urn> --wait' to monitor progress".dimmed()
                );
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    // If --wait flag is set, poll until translation completes
    if wait {
        let urn_for_status = response.urn.clone();
        check_status(client, &urn_for_status, true, output_format).await?;
    }

    Ok(())
}

#[derive(Serialize)]
struct StatusOutput {
    status: String,
    progress: String,
}

async fn check_status(
    client: &DerivativeClient,
    urn: &str,
    wait: bool,
    output_format: OutputFormat,
) -> Result<()> {
    if wait {
        // Poll until complete (spinner hidden in non-interactive mode)
        let spinner = progress::spinner("Checking translation status...");

        let timeout = TRANSLATE_POLL_TIMEOUT;
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                spinner.finish_with_message(format!(
                    "{} Timed out after {} hours. Check status: raps translate status {}",
                    "⏱".yellow().bold(),
                    timeout.as_secs() / 3600,
                    urn
                ));
                break;
            }

            let (status, progress) = client.get_status(urn).await?;
            spinner.set_message(format!("Translation: {} ({})", status, progress));

            match status.as_str() {
                "success" => {
                    spinner.finish_with_message(format!(
                        "{} Translation complete! (100%)",
                        "✓".green().bold()
                    ));
                    break;
                }
                "failed" => {
                    spinner
                        .finish_with_message(format!("{} Translation failed!", "X".red().bold()));
                    anyhow::bail!("Translation failed");
                }
                "timeout" => {
                    spinner.finish_with_message(format!(
                        "{} Translation timed out!",
                        "X".red().bold()
                    ));
                    anyhow::bail!("Translation timed out");
                }
                _ => {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    } else {
        // Just show current status
        let (status, progress) = client.get_status(urn).await?;

        let output = StatusOutput {
            status: status.clone(),
            progress: progress.clone(),
        };

        match output_format {
            OutputFormat::Table => {
                let status_icon = match status.as_str() {
                    "success" => "✓".green().bold(),
                    "failed" | "timeout" => "X".red().bold(),
                    "inprogress" | "pending" => "...".yellow().bold(),
                    _ => "?".dimmed(),
                };
                println!("{} {} ({})", status_icon, status, progress);
            }
            _ => {
                output_format.write(&output)?;
            }
        }
    }

    Ok(())
}

// ============== METADATA HANDLERS ==============

fn parse_region(region: Option<String>) -> Result<Option<MdRegion>> {
    match region {
        Some(r) => Ok(Some(r.parse::<MdRegion>()?)),
        None => Ok(None),
    }
}

async fn show_metadata(
    client: &DerivativeClient,
    urn: &str,
    region: Option<String>,
    output: &str,
) -> Result<()> {
    let region = parse_region(region)?;
    let output_format: OutputFormat = output.parse().unwrap_or(OutputFormat::Table);

    println!("{}", "Fetching model metadata...".dimmed());
    let response = client.get_metadata(urn, region).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Model Views / Viewables".bold());
            println!("{}", "─".repeat(70));
            for view in &response.data.metadata {
                let role_icon = match view.role.as_str() {
                    "3d" => "🧊",
                    "2d" => "📐",
                    _ => "📄",
                };
                println!(
                    "  {} {} {}",
                    role_icon,
                    view.name.bold(),
                    format!("({})", view.role).dimmed()
                );
                println!("    {} {}", "GUID:".dimmed(), view.guid);
                if let Some(ref mime) = view.mime_type {
                    println!("    {} {}", "MIME:".dimmed(), mime);
                }
                if let Some(ref progress) = view.progress {
                    println!("    {} {}", "Progress:".dimmed(), progress);
                }
            }
            if response.data.metadata.is_empty() {
                println!("  {}", "No views found.".dimmed());
            }
        }
        _ => {
            output_format.write(&response)?;
        }
    }

    Ok(())
}

async fn show_object_tree(
    client: &DerivativeClient,
    urn: &str,
    guid: &str,
    region: Option<String>,
    output: &str,
) -> Result<()> {
    let region = parse_region(region)?;
    let output_format: OutputFormat = output.parse().unwrap_or(OutputFormat::Table);

    println!("{}", "Fetching object tree...".dimmed());
    let response = client.get_object_tree(urn, guid, region).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Object Tree".bold());
            println!("{}", "─".repeat(70));
            for node in &response.data.objects {
                print_tree_node(node, 0);
            }
        }
        _ => {
            output_format.write(&response)?;
        }
    }

    Ok(())
}

fn print_tree_node(node: &raps_derivative::ObjectTreeNode, depth: usize) {
    let indent = "  ".repeat(depth + 1);
    let connector = if depth > 0 { "├─ " } else { "" };
    println!(
        "{}{}[{}] {}",
        indent,
        connector,
        node.object_id.to_string().dimmed(),
        node.name
    );
    for child in &node.objects {
        print_tree_node(child, depth + 1);
    }
}

async fn show_properties(
    client: &DerivativeClient,
    urn: &str,
    guid: &str,
    object_id: Option<i64>,
    region: Option<String>,
    output: &str,
) -> Result<()> {
    let region = parse_region(region)?;
    let output_format: OutputFormat = output.parse().unwrap_or(OutputFormat::Table);

    println!("{}", "Fetching properties...".dimmed());
    let response = client.get_properties(urn, guid, object_id, region).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Properties".bold());
            println!("{}", "─".repeat(70));
            for obj in &response.data.collection {
                println!(
                    "\n  {} [{}]",
                    obj.name.bold(),
                    obj.object_id.to_string().dimmed()
                );
                for (category, props) in &obj.properties {
                    println!("    {}:", category.cyan());
                    if let Some(map) = props.as_object() {
                        for (key, value) in map {
                            println!("      {} {}", format!("{}:", key).dimmed(), value);
                        }
                    }
                }
            }
            if response.data.collection.is_empty() {
                println!("  {}", "No properties found.".dimmed());
            }
        }
        _ => {
            output_format.write(&response)?;
        }
    }

    Ok(())
}

async fn query_properties(
    client: &DerivativeClient,
    urn: &str,
    guid: &str,
    filter: &str,
    fields: Option<String>,
    region: Option<String>,
    output: &str,
) -> Result<()> {
    let region = parse_region(region)?;
    let output_format: OutputFormat = output.parse().unwrap_or(OutputFormat::Table);

    // Parse comma-separated object IDs
    let ids: Vec<i64> = filter
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<i64>()
                .context(format!("Invalid object ID: '{}'", s.trim()))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut query = PropertyQuery::by_object_ids(ids);
    if let Some(ref f) = fields {
        query.fields = Some(f.split(',').map(|s| s.trim().to_string()).collect());
    }

    println!("{}", "Querying properties...".dimmed());
    let response = client.query_properties(urn, guid, query, region).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Query Results".bold());
            println!("{}", "─".repeat(70));
            for obj in &response.data.collection {
                println!(
                    "\n  {} [{}]",
                    obj.name.bold(),
                    obj.object_id.to_string().dimmed()
                );
                for (category, props) in &obj.properties {
                    println!("    {}:", category.cyan());
                    if let Some(map) = props.as_object() {
                        for (key, value) in map {
                            println!("      {} {}", format!("{}:", key).dimmed(), value);
                        }
                    }
                }
            }
            if response.data.collection.is_empty() {
                println!("  {}", "No matching properties found.".dimmed());
            }
        }
        _ => {
            output_format.write(&response)?;
        }
    }

    Ok(())
}

async fn show_manifest(
    client: &DerivativeClient,
    urn: &str,
    output_format: OutputFormat,
) -> Result<()> {
    println!("{}", "Fetching manifest...".dimmed());

    let manifest = client.get_manifest(urn).await?;

    match output_format {
        OutputFormat::Table => {
            let status_icon = match manifest.status.as_str() {
                "success" => "✓".green().bold(),
                "failed" | "timeout" => "X".red().bold(),
                "inprogress" | "pending" => "...".yellow().bold(),
                _ => "?".dimmed(),
            };

            println!("\n{}", "Manifest".bold());
            println!("{}", "-".repeat(60));
            println!("  {} {} {}", "Status:".bold(), status_icon, manifest.status);
            println!("  {} {}", "Progress:".bold(), manifest.progress);
            println!("  {} {}", "Region:".bold(), manifest.region);
            println!("  {} {}", "Has Thumbnail:".bold(), manifest.has_thumbnail);

            if let Some(version) = &manifest.version {
                println!("  {} {}", "Version:".bold(), version);
            }

            if !manifest.derivatives.is_empty() {
                println!("\n{}", "Derivatives:".bold());
                println!("{}", "-".repeat(60));

                for derivative in &manifest.derivatives {
                    let status_icon = match derivative.status.as_str() {
                        "success" => "✓".green(),
                        "failed" | "timeout" => "X".red(),
                        "inprogress" | "pending" => "...".yellow(),
                        _ => "?".dimmed(),
                    };

                    println!(
                        "  {} {} {}",
                        status_icon,
                        derivative.output_type.cyan().bold(),
                        derivative.progress.as_deref().unwrap_or("").dimmed()
                    );

                    if let Some(name) = &derivative.name {
                        println!("      {} {}", "Name:".dimmed(), name);
                    }

                    // Show children (viewables)
                    for child in &derivative.children {
                        println!(
                            "      {} {} ({})",
                            "L".dimmed(),
                            child.name.as_deref().unwrap_or(&child.guid),
                            child.role.dimmed()
                        );
                    }
                }
            }

            println!("{}", "-".repeat(60));
        }
        _ => {
            // For non-table formats, serialize the manifest directly
            output_format.write(&manifest)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct DerivativeListOutput {
    guid: String,
    name: String,
    output_type: String,
    role: String,
    size: Option<u64>,
    size_human: Option<String>,
}

async fn list_derivatives(
    client: &DerivativeClient,
    urn: &str,
    format_filter: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching downloadable derivatives...".dimmed());
    }

    let mut derivatives = client.list_downloadable_derivatives(urn).await?;

    // Apply format filter if specified
    if let Some(ref format) = format_filter {
        derivatives = DerivativeClient::filter_by_format(&derivatives, format);
    }

    let outputs: Vec<DerivativeListOutput> = derivatives
        .iter()
        .map(|d| DerivativeListOutput {
            guid: d.guid.clone(),
            name: d.name.clone(),
            output_type: d.output_type.clone(),
            role: d.role.clone(),
            size: d.size,
            size_human: d.size.map(format_size),
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => {
                if format_filter.is_some() {
                    println!(
                        "{}",
                        "No derivatives found matching the specified format.".yellow()
                    );
                } else {
                    println!("{}", "No downloadable derivatives found.".yellow());
                }
            }
            _ => {
                output_format.write(&Vec::<DerivativeListOutput>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Downloadable Derivatives:".bold());
            println!("{}", "-".repeat(90));
            println!(
                "{:<40} {:<12} {:<15} {:>10}",
                "Name".bold(),
                "Format".bold(),
                "Role".bold(),
                "Size".bold()
            );
            println!("{}", "-".repeat(90));

            for d in &outputs {
                let size_str = d.size_human.as_deref().unwrap_or("-");
                println!(
                    "{:<40} {:<12} {:<15} {:>10}",
                    truncate_str(&d.name, 40).cyan(),
                    d.output_type,
                    d.role.dimmed(),
                    size_str
                );
            }

            println!("{}", "-".repeat(90));
            println!(
                "\n{}",
                "Tip: Use 'raps translate download <urn> --format <format>' to download".dimmed()
            );
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct DownloadResultOutput {
    success: bool,
    downloaded: Vec<DownloadedFile>,
    total_size: u64,
    total_size_human: String,
}

#[derive(Serialize)]
struct DownloadedFile {
    name: String,
    size: u64,
    size_human: String,
    path: String,
}

async fn download_derivatives(
    client: &DerivativeClient,
    urn: &str,
    format_filter: Option<String>,
    guid_filter: Option<String>,
    output_dir: Option<PathBuf>,
    all: bool,
    output_format: OutputFormat,
) -> Result<()> {
    // Validate that at least one filter is specified
    if format_filter.is_none() && guid_filter.is_none() && !all {
        anyhow::bail!(
            "Please specify --format, --guid, or --all to select derivatives to download"
        );
    }

    if output_format.supports_colors() {
        println!("{}", "Fetching downloadable derivatives...".dimmed());
    }

    let derivatives = client.list_downloadable_derivatives(urn).await?;

    if derivatives.is_empty() {
        anyhow::bail!("No downloadable derivatives found in manifest");
    }

    // Filter derivatives based on criteria
    let to_download: Vec<_> = if let Some(ref guid) = guid_filter {
        match DerivativeClient::filter_by_guid(&derivatives, guid) {
            Some(d) => vec![d],
            None => anyhow::bail!("No derivative found with GUID '{guid}'"),
        }
    } else if let Some(ref format) = format_filter {
        let filtered = DerivativeClient::filter_by_format(&derivatives, format);
        if filtered.is_empty() {
            anyhow::bail!("No derivatives found with format '{format}'");
        }
        filtered
    } else {
        // --all
        derivatives
    };

    // Determine output directory
    let output_path = output_dir.unwrap_or_else(|| PathBuf::from("."));

    // Create output directory if needed
    if !output_path.exists() {
        tokio::fs::create_dir_all(&output_path).await?;
    }

    if output_format.supports_colors() {
        println!(
            "{} {} derivatives to {}",
            "Downloading".dimmed(),
            to_download.len().to_string().cyan(),
            output_path.display().to_string().cyan()
        );
    }

    let mut downloaded_files = Vec::new();
    let mut total_size: u64 = 0;

    for derivative in to_download {
        let file_path = output_path.join(&derivative.name);

        match client
            .download_derivative(urn, &derivative.urn, &file_path)
            .await
        {
            Ok(size) => {
                total_size += size;
                downloaded_files.push(DownloadedFile {
                    name: derivative.name.clone(),
                    size,
                    size_human: format_size(size),
                    path: file_path.display().to_string(),
                });
            }
            Err(e) => {
                eprintln!(
                    "{} Failed to download {}: {}",
                    "X".red().bold(),
                    derivative.name,
                    e
                );
            }
        }
    }

    let output = DownloadResultOutput {
        success: !downloaded_files.is_empty(),
        downloaded: downloaded_files,
        total_size,
        total_size_human: format_size(total_size),
    };

    match output_format {
        OutputFormat::Table => {
            if output.downloaded.is_empty() {
                println!("{} No files were downloaded.", "X".red().bold());
            } else {
                println!("\n{} Download complete!", "✓".green().bold());
                println!(
                    "  {} {} files",
                    "Downloaded:".bold(),
                    output.downloaded.len()
                );
                println!("  {} {}", "Total size:".bold(), output.total_size_human);

                if output.downloaded.len() <= 10 {
                    println!("\n  {}:", "Files".bold());
                    for file in &output.downloaded {
                        println!(
                            "    {} {} ({})",
                            "-".cyan(),
                            file.name,
                            file.size_human.dimmed()
                        );
                    }
                }
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

/// Format file size in human-readable format
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Truncate string with ellipsis
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

// ============== TRANSLATION PRESETS ==============

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
struct TranslationPreset {
    name: String,
    format: String,
    description: Option<String>,
}

#[derive(Debug, Serialize, serde::Deserialize, Default)]
struct PresetStore {
    presets: Vec<TranslationPreset>,
}

impl PresetStore {
    fn file_path() -> Result<std::path::PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
            .context("Failed to get project directories")?;
        let config_dir = proj_dirs.config_dir();
        std::fs::create_dir_all(config_dir)?;
        Ok(config_dir.join("presets.json"))
    }

    fn load() -> Result<Self> {
        let path = Self::file_path()?;
        if !path.exists() {
            return Ok(Self::default_presets());
        }
        let content = std::fs::read_to_string(&path)?;
        let store: Self = serde_json::from_str(&content)?;
        Ok(store)
    }

    fn save(&self) -> Result<()> {
        let path = Self::file_path()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn default_presets() -> Self {
        Self {
            presets: vec![
                TranslationPreset {
                    name: "viewer".to_string(),
                    format: "svf2".to_string(),
                    description: Some("Optimized for web viewer (SVF2)".to_string()),
                },
                TranslationPreset {
                    name: "export-3d".to_string(),
                    format: "obj".to_string(),
                    description: Some("Export as OBJ mesh for external tools".to_string()),
                },
                TranslationPreset {
                    name: "3d-print".to_string(),
                    format: "stl".to_string(),
                    description: Some("Export for 3D printing (STL)".to_string()),
                },
                TranslationPreset {
                    name: "cad-exchange".to_string(),
                    format: "step".to_string(),
                    description: Some("CAD interchange format (STEP)".to_string()),
                },
                TranslationPreset {
                    name: "bim".to_string(),
                    format: "ifc".to_string(),
                    description: Some("BIM format (IFC)".to_string()),
                },
            ],
        }
    }
}

fn list_presets(output_format: OutputFormat) -> Result<()> {
    let store = PresetStore::load()?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Translation Presets:".bold());
            println!("{}", "-".repeat(70));
            println!(
                "{:<20} {:<12} {}",
                "Name".bold(),
                "Format".bold(),
                "Description".bold()
            );
            println!("{}", "-".repeat(70));

            for preset in &store.presets {
                let desc = preset.description.as_deref().unwrap_or("-");
                println!(
                    "{:<20} {:<12} {}",
                    preset.name.cyan(),
                    preset.format,
                    desc.dimmed()
                );
            }

            println!("{}", "-".repeat(70));
            println!(
                "\n{}",
                "Use 'raps translate preset use <urn> <preset>' to translate".dimmed()
            );
        }
        _ => {
            output_format.write(&store.presets)?;
        }
    }

    Ok(())
}

fn show_preset(name: &str, output_format: OutputFormat) -> Result<()> {
    let store = PresetStore::load()?;

    let preset = store
        .presets
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", name))?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Preset Details:".bold());
            println!("{}", "-".repeat(50));
            println!("  {} {}", "Name:".bold(), preset.name.cyan());
            println!("  {} {}", "Format:".bold(), preset.format);
            if let Some(ref desc) = preset.description {
                println!("  {} {}", "Description:".bold(), desc);
            }
            println!("{}", "-".repeat(50));
        }
        _ => {
            output_format.write(preset)?;
        }
    }

    Ok(())
}

fn create_preset(
    name: &str,
    format: &str,
    description: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    // Validate format
    if DerivativeOutputFormat::from_str(format).is_err() {
        anyhow::bail!(
            "Invalid format '{}'. Valid formats: svf2, svf, obj, stl, step, iges, ifc, thumbnail",
            format
        );
    }

    let mut store = PresetStore::load()?;

    // Check for duplicate
    if store
        .presets
        .iter()
        .any(|p| p.name.eq_ignore_ascii_case(name))
    {
        anyhow::bail!("Preset '{name}' already exists");
    }

    let preset = TranslationPreset {
        name: name.to_string(),
        format: format.to_string(),
        description,
    };

    store.presets.push(preset.clone());
    store.save()?;

    match output_format {
        OutputFormat::Table => {
            println!("{} Preset '{}' created!", "✓".green().bold(), name.cyan());
        }
        _ => {
            output_format.write(&preset)?;
        }
    }

    Ok(())
}

fn delete_preset(name: &str, output_format: OutputFormat) -> Result<()> {
    let mut store = PresetStore::load()?;

    let initial_len = store.presets.len();
    store.presets.retain(|p| !p.name.eq_ignore_ascii_case(name));

    if store.presets.len() == initial_len {
        anyhow::bail!("Preset '{name}' not found");
    }

    store.save()?;

    #[derive(Serialize)]
    struct DeleteOutput {
        success: bool,
        name: String,
    }

    let output = DeleteOutput {
        success: true,
        name: name.to_string(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Preset '{}' deleted!", "✓".green().bold(), name);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

async fn use_preset(
    client: &DerivativeClient,
    urn: &str,
    preset_name: &str,
    output_format: OutputFormat,
) -> Result<()> {
    let store = PresetStore::load()?;

    let preset = store
        .presets
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(preset_name))
        .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", preset_name))?;

    let format = DerivativeOutputFormat::from_str(&preset.format)
        .map_err(|_| anyhow::anyhow!("Invalid format in preset: {}", preset.format))?;

    if output_format.supports_colors() {
        println!(
            "{} Using preset: {} ({})",
            "->".cyan(),
            preset.name.bold(),
            preset.format
        );
    }

    // Start translation using the preset format
    let response = client
        .translate(
            urn,
            format,
            None,
            raps_derivative::MdRegion::default(),
            false,
        )
        .await?;

    #[derive(Serialize)]
    struct UsePresetOutput {
        success: bool,
        preset: String,
        format: String,
        urn: String,
        result: String,
    }

    let output = UsePresetOutput {
        success: response.result == "created" || response.result == "success",
        preset: preset.name.clone(),
        format: preset.format.clone(),
        urn: response.urn.clone(),
        result: response.result.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Translation started with preset '{}'!",
                "✓".green().bold(),
                preset.name
            );
            println!("  {} {}", "Format:".bold(), output.format.cyan());
            println!("  {} {}", "URN:".bold(), output.urn.dimmed());
            println!(
                "\n{}",
                "Use 'raps translate status <urn> --wait' to monitor progress".dimmed()
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_poll_timeout_is_two_hours() {
        assert_eq!(TRANSLATE_POLL_TIMEOUT, Duration::from_secs(7200));
        assert_eq!(TRANSLATE_POLL_TIMEOUT.as_secs() / 3600, 2);
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Translation job handlers: start, status, manifest, list derivatives, download.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::commands::cost::CostEstimate;
use crate::output::OutputFormat;
use base64::Engine as _;
use raps_derivative::{DerivativeClient, OutputFormat as DerivativeOutputFormat, TranslationCache};
use raps_kernel::{progress, prompts};

/// Client-side polling timeout for translations (2 hours).
/// Translations typically complete within minutes, but complex models can take longer.
pub(super) const TRANSLATE_POLL_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);

/// Detect the best output format from a file extension.
/// Returns the format name (e.g. "svf2") and the matched extension for display.
fn detect_output_format(file_path: &str) -> (&'static str, &'static str) {
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        // Autodesk native formats → SVF2 (best for viewing)
        "rvt" => ("svf2", "rvt"),
        "rfa" => ("svf2", "rfa"),
        "rte" => ("svf2", "rte"),
        "rft" => ("svf2", "rft"),
        "dwg" => ("svf2", "dwg"),
        "dxf" => ("svf2", "dxf"),
        "dwf" => ("svf2", "dwf"),
        "dwfx" => ("svf2", "dwfx"),
        "ipt" => ("svf2", "ipt"),
        "iam" => ("svf2", "iam"),
        "ipn" => ("svf2", "ipn"),
        "ide" => ("svf2", "ide"),
        "nwd" => ("svf2", "nwd"),
        "nwc" => ("svf2", "nwc"),
        "nwf" => ("svf2", "nwf"),
        "max" => ("svf2", "max"),
        "3ds" => ("svf2", "3ds"),
        // Open/exchange formats → SVF2 (most useful for viewing)
        "ifc" => ("svf2", "ifc"),
        "ifczip" => ("svf2", "ifczip"),
        "stp" => ("svf2", "stp"),
        "step" => ("svf2", "step"),
        "ste" => ("svf2", "ste"),
        "sat" => ("svf2", "sat"),
        "sab" => ("svf2", "sab"),
        "obj" => ("svf2", "obj"),
        "stl" => ("svf2", "stl"),
        "fbx" => ("svf2", "fbx"),
        "gltf" => ("svf2", "gltf"),
        "glb" => ("svf2", "glb"),
        // Fallback
        _ => ("svf2", ""),
    }
}

/// Try to extract the file extension from a base64-encoded URN.
/// APS URNs are base64url-encoded strings of the form "urn:adsk.objects:os.object:<bucket>/<key>".
/// Returns the decoded path/filename portion if decoding succeeds.
fn filename_from_urn(urn: &str) -> Option<String> {
    use base64::Engine as _;

    // Try base64url (no pad) first, then standard variants
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(urn)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(urn))
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(urn))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(urn))
        .ok()?;

    let s = std::str::from_utf8(&decoded).ok()?;
    // The URN looks like "urn:adsk.objects:os.object:bucket/path/to/file.rvt"
    // Extract the last path component as the filename
    s.split('/').last().map(|f| f.to_string())
}

#[derive(Serialize, schemars::JsonSchema)]
struct TranslationStartOutput {
    success: bool,
    result: String,
    urn: String,
    accepted_formats: Vec<String>,
}

#[derive(Serialize, schemars::JsonSchema)]
struct StatusOutput {
    status: String,
    progress: String,
}

#[derive(Serialize, schemars::JsonSchema)]
struct DerivativeListOutput {
    guid: String,
    name: String,
    output_type: String,
    role: String,
    size: Option<u64>,
    size_human: Option<String>,
}

#[derive(Serialize, schemars::JsonSchema)]
struct DownloadResultOutput {
    success: bool,
    downloaded: Vec<DownloadedFile>,
    total_size: u64,
    total_size_human: String,
}

#[derive(Serialize, schemars::JsonSchema)]
struct DownloadedFile {
    name: String,
    size: u64,
    size_human: String,
    path: String,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn start_translation(
    client: &DerivativeClient,
    urn: Option<String>,
    format: Option<String>,
    root_filename: Option<String>,
    wait: bool,
    watch: bool,
    poll_interval: u64,
    watch_timeout: u64,
    output_format: OutputFormat,
    region_str: String,
    force: bool,
    cost_estimate: bool,
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
            // Try to auto-detect from URN (base64-decode to get filename/extension)
            let auto_detected =
                filename_from_urn(&source_urn).map(|filename| detect_output_format(&filename));

            if let Some((fmt, ext)) = auto_detected.filter(|(_, ext)| !ext.is_empty()) {
                if output_format.supports_colors() {
                    println!(
                        "{} Auto-detected output format: {} (from .{} extension)",
                        "->".dimmed(),
                        fmt.cyan(),
                        ext
                    );
                } else {
                    println!(
                        "Auto-detected output format: {} (from .{} extension)",
                        fmt, ext
                    );
                }
                match fmt {
                    "svf2" => DerivativeOutputFormat::Svf2,
                    "svf" => DerivativeOutputFormat::Svf,
                    "obj" => DerivativeOutputFormat::Obj,
                    "stl" => DerivativeOutputFormat::Stl,
                    "step" => DerivativeOutputFormat::Step,
                    "iges" => DerivativeOutputFormat::Iges,
                    "ifc" => DerivativeOutputFormat::Ifc,
                    _ => DerivativeOutputFormat::Svf2,
                }
            } else {
                // Interactive mode: prompt for format
                let formats = DerivativeOutputFormat::all();
                let format_labels: Vec<String> = formats.iter().map(|f| f.to_string()).collect();

                let selection = prompts::select("Select output format", &format_labels)?;
                formats[selection]
            }
        }
    };

    // ------------------------------------------------------------------
    // Translation deduplication cache (issue #205)
    // ------------------------------------------------------------------
    let format_key = derivative_format.type_name().to_string();
    let mut cache = TranslationCache::load();

    if force {
        // Invalidate any stale cache entry so we re-translate unconditionally.
        cache.invalidate(&source_urn, &format_key);
    } else {
        // 1. Check local disk cache first (fast, no network).
        if let Some(entry) = cache.get(&source_urn, &format_key) {
            if entry.status == "success" {
                if output_format.supports_colors() {
                    println!(
                        "{} Translation already complete (cached). Use --force to re-translate.",
                        "\u{2713}".green().bold()
                    );
                } else {
                    println!("Translation already complete (cached). Use --force to re-translate.");
                }
                return Ok(());
            }
        } else {
            // 2. No local cache entry — check the live manifest to avoid
            //    re-submitting a job that succeeded in a previous session.
            match client.get_manifest(&source_urn).await {
                Ok(manifest) if manifest.status == "success" => {
                    // Cache it for next time and skip submission.
                    cache.insert(
                        &source_urn,
                        &format_key,
                        manifest.urn.clone(),
                        "success".to_string(),
                    );
                    cache.save();
                    if output_format.supports_colors() {
                        println!(
                            "{} Translation already complete (manifest exists). Use --force to re-translate.",
                            "\u{2713}".green().bold()
                        );
                    } else {
                        println!(
                            "Translation already complete (manifest exists). Use --force to re-translate."
                        );
                    }
                    return Ok(());
                }
                // 404, inprogress, pending, or any error → fall through and submit.
                _ => {}
            }
        }
    }

    // Show cost estimate if requested, using file extension from URN
    if cost_estimate {
        // Decode the URN to extract the file extension heuristic
        let ext = {
            let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(source_urn.trim_end_matches('='))
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or_default();
            // URN typically ends with the object key, e.g. ".../<filename.rvt>"
            std::path::Path::new(&decoded)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase()
        };
        // Use 0 as file size since we don't have it at this point
        let estimate = CostEstimate::for_translation(0, &ext);
        estimate.print(&output_format);
    }

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
            println!("{} Translation job started!", "\u{2713}".green().bold());
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

    // If --watch flag is set, use the dedicated watch loop with configurable interval/timeout
    if watch && !wait {
        super::watch::watch_translation(
            client,
            &response.urn,
            poll_interval,
            watch_timeout,
            &output_format,
        )
        .await?;

        // Persist success to the deduplication cache.
        cache.insert(
            &source_urn,
            &format_key,
            response.urn.clone(),
            "success".to_string(),
        );
        cache.save();
    }

    Ok(())
}

pub(super) async fn check_status(
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
                    "\u{23F1}".yellow().bold(),
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
                        "\u{2713}".green().bold()
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
                    "success" => "\u{2713}".green().bold(),
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

pub(super) async fn show_manifest(
    client: &DerivativeClient,
    urn: &str,
    output_format: OutputFormat,
) -> Result<()> {
    println!("{}", "Fetching manifest...".dimmed());

    let manifest = client.get_manifest(urn).await?;

    match output_format {
        OutputFormat::Table => {
            let status_icon = match manifest.status.as_str() {
                "success" => "\u{2713}".green().bold(),
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
                        "success" => "\u{2713}".green(),
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

pub(super) async fn list_derivatives(
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

pub(super) async fn download_derivatives(
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
        let file_path = raps_kernel::security::safe_join(&output_path, &derivative.name)?;

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
                println!("\n{} Download complete!", "\u{2713}".green().bold());
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

// ---------------------------------------------------------------------------
// Serverless dispatch
// ---------------------------------------------------------------------------

/// Dispatch a translation job to a serverless Fly.io machine.
#[allow(clippy::too_many_arguments)]
pub(super) async fn start_serverless(
    urn: Option<String>,
    format: Option<String>,
    root_filename: Option<String>,
    region: String,
    force: bool,
    wait: bool,
    notify: bool,
    output_format: OutputFormat,
) -> Result<()> {
    use raps_kernel::serverless::{ServerlessDispatchAgent, TranslateJobRequest};

    let urn = match urn {
        Some(u) => u,
        None => prompts::input("URN", None)?,
    };
    let format = match format {
        Some(f) => f,
        None => {
            // Try to auto-detect from URN
            let auto = filename_from_urn(&urn)
                .map(|filename| detect_output_format(&filename))
                .filter(|(_, ext)| !ext.is_empty());

            if let Some((fmt, ext)) = auto {
                println!(
                    "Auto-detected output format: {} (from .{} extension)",
                    fmt, ext
                );
                fmt.to_string()
            } else {
                prompts::input("Output format", Some("svf2"))?
            }
        }
    };

    let job = TranslateJobRequest {
        urn: urn.clone(),
        output_format: format.clone(),
        root_filename,
        region: Some(region),
        force,
    };

    let agent = ServerlessDispatchAgent::from_config()
        .context("Failed to load serverless config from swarm.toml")?;

    println!(
        "{} Dispatching to Fly.io: {} -> {}",
        "▶".cyan(),
        truncate_str(&urn, 30),
        format,
    );

    let receipt = agent.dispatch_translate(&job).await?;

    #[derive(Serialize, schemars::JsonSchema)]
    struct ServerlessDispatchOutput {
        machine_id: String,
        region: String,
        state: String,
        app: String,
    }

    let out = ServerlessDispatchOutput {
        machine_id: receipt.machine_id.clone(),
        region: receipt.region.clone(),
        state: receipt.state.clone(),
        app: receipt.app.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Dispatched to Fly.io", "✓".green());
            println!("  Machine: {}", receipt.machine_id);
            println!("  Region:  {}", receipt.region);
            println!("  State:   {}", receipt.state);
            println!("  App:     {}", receipt.app);
        }
        _ => {
            output_format.write(&out)?;
        }
    }

    if notify {
        let msg = format!(
            "RAPS: Translation dispatched — URN: {}, Machine: {}, Region: {}",
            truncate_str(&urn, 40),
            receipt.machine_id,
            receipt.region,
        );
        if let Err(e) = agent.notify_slack(&msg).await {
            eprintln!("{} Slack notification failed: {}", "!".yellow(), e);
        }
    }

    if wait {
        println!("{} Polling machine status...", "⏳".dimmed());
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            match agent.machine_status(&receipt.machine_id).await {
                Ok(status) => match status.state.as_str() {
                    "stopped" | "destroyed" => {
                        println!("{} Machine finished (state: {})", "✓".green(), status.state);
                        break;
                    }
                    "failed" => {
                        anyhow::bail!("Machine failed — check Fly.io dashboard");
                    }
                    _ => {
                        println!("  State: {} ...", status.state);
                    }
                },
                Err(e) => {
                    eprintln!("{} Status check failed: {}", "!".yellow(), e);
                }
            }
        }
    }

    Ok(())
}

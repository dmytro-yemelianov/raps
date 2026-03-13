// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Workflow command — compose upload → translate → download in one step.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::output::OutputFormat;
use raps_derivative::DerivativeClient;
use raps_kernel::config::Config;
use raps_kernel::progress;
use raps_oss::OssClient;

#[derive(Debug, Subcommand)]
pub enum WorkflowCommands {
    /// Run the full pipeline: upload → translate → (optionally) download
    Run {
        /// Local file to process
        #[arg(long, short)]
        file: PathBuf,

        /// Target OSS bucket key
        #[arg(long, short)]
        bucket: Option<String>,

        /// Output format for translation (auto-detected from extension if omitted)
        #[arg(long)]
        output_format: Option<String>,

        /// Directory to download translated derivatives into
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Skip upload if identical object already exists
        #[arg(long, default_value = "true")]
        skip_existing: bool,

        /// Translation polling interval in seconds
        #[arg(long, default_value = "5")]
        poll_interval: u64,

        /// Translation timeout in seconds (0 = no limit)
        #[arg(long, default_value = "1800")]
        watch_timeout: u64,

        /// Show what would happen without executing
        #[arg(long)]
        dry_run: bool,
    },
}

impl WorkflowCommands {
    pub async fn execute(self, config: &Config, output_format: OutputFormat) -> Result<()> {
        match self {
            WorkflowCommands::Run {
                file,
                bucket,
                output_format: translation_format,
                output_dir,
                skip_existing,
                poll_interval,
                watch_timeout,
                dry_run,
            } => {
                run_workflow(
                    config,
                    file,
                    bucket,
                    translation_format,
                    output_dir,
                    skip_existing,
                    poll_interval,
                    watch_timeout,
                    dry_run,
                    output_format,
                )
                .await
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_workflow(
    config: &Config,
    file: PathBuf,
    bucket: Option<String>,
    translation_format: Option<String>,
    output_dir: Option<PathBuf>,
    skip_existing: bool,
    poll_interval: u64,
    watch_timeout: u64,
    dry_run: bool,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!(
            "{} {}",
            "Running workflow for".bold(),
            file.display().to_string().cyan()
        );
    } else {
        println!("Running workflow for {}", file.display());
    }

    if dry_run {
        println!();
        println!("{}", "[dry-run] Steps that would execute:".yellow().bold());
        println!(
            "  {} Upload {} to OSS bucket{}",
            "1.".bold(),
            file.display().to_string().cyan(),
            bucket
                .as_deref()
                .map(|b| format!(" '{}'", b))
                .unwrap_or_default()
        );
        if skip_existing {
            println!(
                "     {} skip if identical object already exists",
                "->".dimmed()
            );
        }
        println!(
            "  {} Start translation (format: {})",
            "2.".bold(),
            translation_format
                .as_deref()
                .unwrap_or("auto-detect")
                .cyan()
        );
        println!(
            "     {} poll every {}s, timeout {}s",
            "->".dimmed(),
            poll_interval,
            watch_timeout
        );
        if let Some(ref dir) = output_dir {
            println!(
                "  {} Download derivatives to {}",
                "3.".bold(),
                dir.display().to_string().cyan()
            );
        } else {
            println!("  {} Download: skipped (no --output-dir)", "3.".bold());
        }
        return Ok(());
    }

    // ── Step 1: Upload ────────────────────────────────────────────────────────

    if output_format.supports_colors() {
        println!("\n{} {}", "Step 1/3:".bold(), "Upload".cyan());
    } else {
        println!("\nStep 1/3: Upload");
    }

    if !file.exists() {
        anyhow::bail!("File not found: {}", file.display());
    }

    let auth = raps_kernel::auth::AuthClient::new(config.clone());
    let oss_client = OssClient::new(config.clone(), auth);

    // Resolve bucket key
    let bucket_key = match bucket {
        Some(b) => b,
        None => {
            let buckets = oss_client.list_buckets().await?;
            if buckets.is_empty() {
                anyhow::bail!("No buckets found. Create a bucket first using 'raps bucket create'");
            }
            // Use first bucket when non-interactive; raps_kernel::prompts would block in CI
            let keys: Vec<String> = buckets.iter().map(|b| b.bucket_key.clone()).collect();
            let selection = raps_kernel::prompts::select("Select bucket", &keys)?;
            keys[selection].clone()
        }
    };

    let object_key = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unnamed")
        .to_string();

    // Duplicate detection
    let urn = if skip_existing {
        match oss_client
            .check_duplicate(&bucket_key, &object_key, &file)
            .await?
        {
            Some(existing) => {
                if output_format.supports_colors() {
                    println!(
                        "  {} Identical object already exists — skipping upload",
                        "=".cyan().bold()
                    );
                } else {
                    println!("  Identical object already exists — skipping upload");
                }
                oss_client.get_urn(&bucket_key, &existing.object_key)
            }
            None => {
                let info = oss_client
                    .upload_object(&bucket_key, &object_key, &file)
                    .await?;
                let urn = oss_client.get_urn(&bucket_key, &info.object_key);
                if output_format.supports_colors() {
                    println!("  {} Upload complete", "\u{2713}".green().bold());
                    println!("  {} {}", "URN:".bold(), urn.dimmed());
                } else {
                    println!("  Upload complete. URN: {}", urn);
                }
                urn
            }
        }
    } else {
        let info = oss_client
            .upload_object(&bucket_key, &object_key, &file)
            .await?;
        let urn = oss_client.get_urn(&bucket_key, &info.object_key);
        if output_format.supports_colors() {
            println!("  {} Upload complete", "\u{2713}".green().bold());
            println!("  {} {}", "URN:".bold(), urn.dimmed());
        } else {
            println!("  Upload complete. URN: {}", urn);
        }
        urn
    };

    // ── Step 2: Translate ─────────────────────────────────────────────────────

    if output_format.supports_colors() {
        println!("\n{} {}", "Step 2/3:".bold(), "Translate".cyan());
    } else {
        println!("\nStep 2/3: Translate");
    }

    let auth2 = raps_kernel::auth::AuthClient::new(config.clone());
    let derivative_client = DerivativeClient::new(config.clone(), auth2);

    // Resolve translation output format
    let derivative_format = match translation_format.as_deref() {
        Some(f) => match f.to_lowercase().as_str() {
            "svf2" => raps_derivative::OutputFormat::Svf2,
            "svf" => raps_derivative::OutputFormat::Svf,
            "thumbnail" => raps_derivative::OutputFormat::Thumbnail,
            "obj" => raps_derivative::OutputFormat::Obj,
            "stl" => raps_derivative::OutputFormat::Stl,
            "step" => raps_derivative::OutputFormat::Step,
            "iges" => raps_derivative::OutputFormat::Iges,
            "ifc" => raps_derivative::OutputFormat::Ifc,
            other => anyhow::bail!(
                "Unknown translation format '{}'. Valid: svf2, svf, thumbnail, obj, stl, step, iges, ifc",
                other
            ),
        },
        None => {
            // Auto-detect from file extension
            let ext = file
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let fmt = match ext.as_str() {
                "rvt" | "rfa" | "rte" | "rft" | "dwg" | "dxf" | "dwf" | "dwfx" | "ipt" | "iam"
                | "ipn" | "ide" | "nwd" | "nwc" | "nwf" | "max" | "3ds" | "ifc" | "ifczip"
                | "stp" | "step" | "ste" | "sat" | "sab" | "obj" | "stl" | "fbx" | "gltf"
                | "glb" => "svf2",
                _ => "svf2",
            };
            if output_format.supports_colors() {
                println!(
                    "  {} Auto-detected translation format: {}",
                    "->".dimmed(),
                    fmt.cyan()
                );
            }
            raps_derivative::OutputFormat::Svf2
        }
    };

    let region: raps_derivative::MdRegion = "US".parse().expect("US is always valid");

    let response = derivative_client
        .translate(&urn, derivative_format, None, region, false)
        .await?;

    if output_format.supports_colors() {
        println!(
            "  {} Translation job submitted (result: {})",
            "\u{2713}".green().bold(),
            response.result.cyan()
        );
    } else {
        println!("  Translation job submitted (result: {})", response.result);
    }

    // Poll until complete
    let deadline = if watch_timeout > 0 {
        Some(Instant::now() + Duration::from_secs(watch_timeout))
    } else {
        None
    };

    let spinner = progress::spinner("Waiting for translation...");

    loop {
        if let Some(dl) = deadline {
            if Instant::now() > dl {
                spinner.finish_with_message(format!(
                    "{} Timed out after {}s",
                    "\u{23F1}".yellow().bold(),
                    watch_timeout
                ));
                anyhow::bail!("Translation watch timed out after {}s", watch_timeout);
            }
        }

        let (status, progress_msg) = derivative_client.get_status(&response.urn).await?;

        spinner.set_message(format!(
            "Translating... status={} progress={}",
            status, progress_msg
        ));

        match status.as_str() {
            "success" => {
                spinner.finish_with_message(format!(
                    "{} Translation complete! ({})",
                    "\u{2713}".green().bold(),
                    progress_msg
                ));
                break;
            }
            "failed" | "timeout" => {
                spinner.finish_with_message(format!("{} Translation {}", "X".red().bold(), status));
                anyhow::bail!("Translation failed with status: {}", status);
            }
            _ => {
                tokio::time::sleep(Duration::from_secs(poll_interval)).await;
            }
        }
    }

    // ── Step 3: Download ──────────────────────────────────────────────────────

    if let Some(ref out_dir) = output_dir {
        if output_format.supports_colors() {
            println!("\n{} {}", "Step 3/3:".bold(), "Download".cyan());
        } else {
            println!("\nStep 3/3: Download");
        }

        if !out_dir.exists() {
            tokio::fs::create_dir_all(out_dir).await?;
        }

        let derivatives = derivative_client
            .list_downloadable_derivatives(&response.urn)
            .await?;

        if derivatives.is_empty() {
            if output_format.supports_colors() {
                println!("  {} No downloadable derivatives found.", "!".yellow());
            } else {
                println!("  No downloadable derivatives found.");
            }
        } else {
            if output_format.supports_colors() {
                println!(
                    "  {} Downloading {} derivatives to {}",
                    "->".dimmed(),
                    derivatives.len().to_string().cyan(),
                    out_dir.display().to_string().cyan()
                );
            }

            let mut total_size: u64 = 0;
            let mut downloaded_count = 0usize;

            for derivative in &derivatives {
                let file_path = raps_kernel::security::safe_join(out_dir, &derivative.name)?;
                match derivative_client
                    .download_derivative(&response.urn, &derivative.urn, &file_path)
                    .await
                {
                    Ok(size) => {
                        total_size += size;
                        downloaded_count += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "  {} Failed to download {}: {}",
                            "X".red().bold(),
                            derivative.name,
                            e
                        );
                    }
                }
            }

            if output_format.supports_colors() {
                println!(
                    "  {} Downloaded {} files ({} bytes total)",
                    "\u{2713}".green().bold(),
                    downloaded_count,
                    total_size
                );
            } else {
                println!(
                    "  Downloaded {} files ({} bytes total)",
                    downloaded_count, total_size
                );
            }
        }
    } else {
        if output_format.supports_colors() {
            println!(
                "\n{} {} {}",
                "Step 3/3:".bold(),
                "Download".dimmed(),
                "(skipped — use --output-dir to download)".dimmed()
            );
        } else {
            println!("\nStep 3/3: Download (skipped — use --output-dir to download)");
        }
    }

    if output_format.supports_colors() {
        println!("\n{} Workflow complete!", "\u{2713}".green().bold());
    } else {
        println!("\nWorkflow complete!");
    }

    Ok(())
}

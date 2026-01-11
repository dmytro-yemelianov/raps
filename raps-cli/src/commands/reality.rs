// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Reality Capture commands
//!
//! Commands for photogrammetry processing.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;

use raps_kernel::interactive;
use raps_kernel::output::OutputFormat;
use raps_reality::{OutputFormat as RealityOutputFormat, RealityCaptureClient, SceneType};

#[derive(Debug, Subcommand)]
pub enum RealityCommands {
    /// Create a new photoscene
    Create {
        /// Photoscene name
        #[arg(short, long)]
        name: Option<String>,

        /// Scene type (aerial or object)
        #[arg(short, long)]
        scene_type: Option<String>,

        /// Output format (rcm, rcs, obj, fbx, ortho)
        #[arg(short, long)]
        format: Option<String>,
    },

    /// Upload photos to a photoscene
    Upload {
        /// Photoscene ID
        photoscene_id: String,

        /// Photo files to upload
        #[arg(required = true)]
        photos: Vec<PathBuf>,
    },

    /// Start processing a photoscene
    Process {
        /// Photoscene ID
        photoscene_id: String,
    },

    /// Check photoscene progress
    Status {
        /// Photoscene ID
        photoscene_id: String,

        /// Wait for completion
        #[arg(short, long)]
        wait: bool,
    },

    /// Get result (download link)
    Result {
        /// Photoscene ID
        photoscene_id: String,

        /// Output format
        #[arg(short, long, default_value = "obj")]
        format: String,
    },

    /// List available output formats
    Formats,

    /// Delete a photoscene
    Delete {
        /// Photoscene ID
        photoscene_id: String,
    },
}

impl RealityCommands {
    pub async fn execute(
        self,
        client: &RealityCaptureClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            RealityCommands::Create {
                name,
                scene_type,
                format,
            } => create_photoscene(client, name, scene_type, format, output_format).await,
            RealityCommands::Upload {
                photoscene_id,
                photos,
            } => upload_photos(client, &photoscene_id, photos, output_format).await,
            RealityCommands::Process { photoscene_id } => {
                start_processing(client, &photoscene_id, output_format).await
            }
            RealityCommands::Status {
                photoscene_id,
                wait,
            } => check_status(client, &photoscene_id, wait, output_format).await,
            RealityCommands::Result {
                photoscene_id,
                format,
            } => get_result(client, &photoscene_id, &format, output_format).await,
            RealityCommands::Formats => list_formats(client, output_format),
            RealityCommands::Delete { photoscene_id } => {
                delete_photoscene(client, &photoscene_id, output_format).await
            }
        }
    }
}

async fn create_photoscene(
    client: &RealityCaptureClient,
    name: Option<String>,
    scene_type: Option<String>,
    format: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    // Get name
    let scene_name = match name {
        Some(n) => n,
        None => {
            // In non-interactive mode, require the name
            if interactive::is_non_interactive() {
                anyhow::bail!(
                    "Photoscene name is required in non-interactive mode. Use --name flag."
                );
            }
            Input::new()
                .with_prompt("Enter photoscene name")
                .interact_text()?
        }
    };

    // Get scene type
    let selected_scene_type = match scene_type {
        Some(t) => match t.to_lowercase().as_str() {
            "aerial" => SceneType::Aerial,
            "object" => SceneType::Object,
            _ => anyhow::bail!("Invalid scene type. Use 'aerial' or 'object'"),
        },
        None => {
            // In non-interactive mode, default to object
            if interactive::is_non_interactive() {
                SceneType::Object
            } else {
                let types = vec!["aerial (drone/outdoor)", "object (turntable/indoor)"];
                let selection = Select::new()
                    .with_prompt("Select scene type")
                    .items(&types)
                    .interact()?;
                if selection == 0 {
                    SceneType::Aerial
                } else {
                    SceneType::Object
                }
            }
        }
    };

    // Get output format
    let selected_format = match format {
        Some(f) => parse_format(&f)?,
        None => {
            // In non-interactive mode, default to OBJ
            if interactive::is_non_interactive() {
                RealityOutputFormat::Obj
            } else {
                let formats = RealityOutputFormat::all();
                let format_labels: Vec<String> = formats
                    .iter()
                    .map(|f| format!("{} - {}", f, f.description()))
                    .collect();

                let selection = Select::new()
                    .with_prompt("Select output format")
                    .items(&format_labels)
                    .default(2) // OBJ is usually a good default
                    .interact()?;

                formats[selection]
            }
        }
    };

    if output_format.supports_colors() {
        println!("{}", "Creating photoscene...".dimmed());
    }

    let photoscene = client
        .create_photoscene(&scene_name, selected_scene_type, selected_format)
        .await?;

    #[derive(Serialize)]
    struct CreatePhotosceneOutput {
        success: bool,
        photoscene_id: String,
        name: String,
    }

    let output = CreatePhotosceneOutput {
        success: true,
        photoscene_id: photoscene.photoscene_id.clone(),
        name: scene_name.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Photoscene created!", "✓".green().bold());
            println!("  {} {}", "ID:".bold(), output.photoscene_id.cyan());
            println!("  {} {}", "Name:".bold(), output.name);

            println!("\n{}", "Next steps:".yellow());
            println!(
                "  1. Upload photos: raps reality upload {} <photo1.jpg> <photo2.jpg> ...",
                output.photoscene_id
            );
            println!(
                "  2. Start processing: raps reality process {}",
                output.photoscene_id
            );
            println!(
                "  3. Check status: raps reality status {} --wait",
                output.photoscene_id
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

async fn upload_photos(
    client: &RealityCaptureClient,
    photoscene_id: &str,
    photos: Vec<PathBuf>,
    _output_format: OutputFormat,
) -> Result<()> {
    // Validate files exist
    for photo in &photos {
        if !photo.exists() {
            anyhow::bail!("File not found: {}", photo.display());
        }
    }

    let pb = ProgressBar::new(photos.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}/{len}")
            .unwrap()
            .progress_chars("█▓░"),
    );
    pb.set_message("Uploading photos");

    // Upload in batches of 5
    let photo_refs: Vec<&std::path::Path> = photos.iter().map(|p| p.as_path()).collect();

    for chunk in photo_refs.chunks(5) {
        client.upload_photos(photoscene_id, chunk).await?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("Upload complete");

    println!("{} Uploaded {} photos!", "✓".green().bold(), photos.len());
    Ok(())
}

async fn start_processing(
    client: &RealityCaptureClient,
    photoscene_id: &str,
    _output_format: OutputFormat,
) -> Result<()> {
    println!("{}", "Starting processing...".dimmed());

    client.start_processing(photoscene_id).await?;

    println!("{} Processing started!", "✓".green().bold());
    println!(
        "  {}",
        "Use 'raps reality status <id> --wait' to monitor progress".dimmed()
    );
    Ok(())
}

async fn check_status(
    client: &RealityCaptureClient,
    photoscene_id: &str,
    wait: bool,
    _output_format: OutputFormat,
) -> Result<()> {
    if wait {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.enable_steady_tick(Duration::from_millis(100));

        loop {
            let progress = client.get_progress(photoscene_id).await?;
            let msg = progress.progress_msg.as_deref().unwrap_or("");
            spinner.set_message(format!("Progress: {}% - {}", progress.progress, msg));

            if progress.progress == "100" || progress.status.as_deref() == Some("Done") {
                spinner.finish_with_message(format!("{} Processing complete!", "✓".green().bold()));
                break;
            }

            if progress.status.as_deref() == Some("Error") {
                spinner.finish_with_message(format!(
                    "{} Processing failed: {}",
                    "✗".red().bold(),
                    msg
                ));
                break;
            }

            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    } else {
        let progress = client.get_progress(photoscene_id).await?;

        println!("{}", "Photoscene Status:".bold());
        println!("  {} {}%", "Progress:".bold(), progress.progress.cyan());

        if let Some(ref status) = progress.status {
            println!("  {} {}", "Status:".bold(), status);
        }

        if let Some(ref msg) = progress.progress_msg {
            println!("  {} {}", "Message:".bold(), msg.dimmed());
        }
    }

    Ok(())
}

async fn get_result(
    client: &RealityCaptureClient,
    photoscene_id: &str,
    format: &str,
    _output_format: OutputFormat,
) -> Result<()> {
    let output_format = parse_format(format)?;

    println!("{}", "Fetching result...".dimmed());

    let result = client.get_result(photoscene_id, output_format).await?;

    println!("{}", "Photoscene Result:".bold());
    println!("  {} {}", "ID:".bold(), result.photoscene_id);
    println!("  {} {}%", "Progress:".bold(), result.progress);

    if let Some(ref link) = result.scene_link {
        println!("\n{}", "Download Link:".green().bold());
        println!("  {}", link);
    } else {
        println!(
            "{}",
            "No download link available yet. Processing may still be in progress.".yellow()
        );
    }

    if let Some(ref size) = result.file_size {
        println!("  {} {}", "File Size:".bold(), size);
    }

    Ok(())
}

fn list_formats(client: &RealityCaptureClient, _output_format: OutputFormat) -> Result<()> {
    let formats = client.available_formats();

    println!("\n{}", "Available Output Formats:".bold());
    println!("{}", "─".repeat(60));

    for format in formats {
        println!(
            "  {} {} - {}",
            "•".cyan(),
            format,
            format.description().dimmed()
        );
    }

    println!("{}", "─".repeat(60));
    Ok(())
}

async fn delete_photoscene(
    client: &RealityCaptureClient,
    photoscene_id: &str,
    _output_format: OutputFormat,
) -> Result<()> {
    println!("{}", "Deleting photoscene...".dimmed());

    client.delete_photoscene(photoscene_id).await?;

    println!(
        "{} Photoscene '{}' deleted!",
        "✓".green().bold(),
        photoscene_id
    );
    Ok(())
}

fn parse_format(s: &str) -> Result<RealityOutputFormat> {
    match s.to_lowercase().as_str() {
        "rcm" => Ok(RealityOutputFormat::Rcm),
        "rcs" => Ok(RealityOutputFormat::Rcs),
        "obj" => Ok(RealityOutputFormat::Obj),
        "fbx" => Ok(RealityOutputFormat::Fbx),
        "ortho" => Ok(RealityOutputFormat::Ortho),
        _ => anyhow::bail!("Invalid format. Use: rcm, rcs, obj, fbx, ortho"),
    }
}

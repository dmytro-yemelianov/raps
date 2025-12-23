//! Translation commands for Model Derivative API
//!
//! Commands for starting translations, checking status, and viewing manifests.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::time::Duration;

use crate::api::{derivative::OutputFormat as DerivativeOutputFormat, DerivativeClient};
use crate::interactive;
use crate::output::OutputFormat;

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
}

impl TranslateCommands {
    pub async fn execute(self, client: &DerivativeClient, output_format: OutputFormat) -> Result<()> {
        match self {
            TranslateCommands::Start {
                urn,
                format,
                root_filename,
            } => start_translation(client, urn, format, root_filename, output_format).await,
            TranslateCommands::Status { urn, wait } => check_status(client, &urn, wait, output_format).await,
            TranslateCommands::Manifest { urn } => show_manifest(client, &urn, output_format).await,
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

async fn start_translation(
    client: &DerivativeClient,
    urn: Option<String>,
    format: Option<String>,
    root_filename: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    // Get URN interactively if not provided
    let source_urn = match urn {
        Some(u) => u,
        None => {
            // In non-interactive mode, require the URN
            if interactive::is_non_interactive() {
                anyhow::bail!("URN is required in non-interactive mode. Use --urn flag or provide as argument.");
            }
            
            // Interactive mode: prompt for URN
            Input::new()
                .with_prompt("Enter the base64-encoded URN")
                .validate_with(|input: &String| -> Result<(), &str> {
                    if input.is_empty() {
                        Err("URN cannot be empty")
                    } else {
                        Ok(())
                    }
                })
                .interact_text()?
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
            // In non-interactive mode, require the format
            if interactive::is_non_interactive() {
                anyhow::bail!("--format is required in non-interactive mode. Use: svf2, svf, thumbnail, obj, stl, step, iges, ifc");
            }
            
            // Interactive mode: prompt for format
            let formats = DerivativeOutputFormat::all();
            let format_labels: Vec<String> = formats.iter().map(|f| f.to_string()).collect();

            let selection = Select::new()
                .with_prompt("Select output format")
                .items(&format_labels)
                .default(0)
                .interact()?;

            formats[selection]
        }
    };

    if output_format.supports_colors() {
        println!(
            "{} {} {} {}",
            "Starting translation".dimmed(),
            "→".dimmed(),
            derivative_format.to_string().cyan(),
            "format".dimmed()
        );
    }

    let response = client
        .translate(&source_urn, derivative_format, root_filename.as_deref())
        .await?;

    let accepted_formats: Vec<String> = response.accepted_jobs.as_ref()
        .map(|jobs| jobs.output.formats.iter().map(|f| f.format_type.clone()).collect())
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
                    println!("    {} {}", "•".dimmed(), format.cyan());
                }
            }

            println!(
                "\n{}",
                "Tip: Use 'raps translate status <urn> --wait' to monitor progress".dimmed()
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct StatusOutput {
    status: String,
    progress: String,
}

async fn check_status(client: &DerivativeClient, urn: &str, wait: bool, output_format: OutputFormat) -> Result<()> {
    if wait {
        // Poll until complete
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.enable_steady_tick(Duration::from_millis(100));

        loop {
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
                        .finish_with_message(format!("{} Translation failed!", "✗".red().bold()));
                    anyhow::bail!("Translation failed");
                }
                "timeout" => {
                    spinner.finish_with_message(format!(
                        "{} Translation timed out!",
                        "✗".red().bold()
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
                    "failed" | "timeout" => "✗".red().bold(),
                    "inprogress" | "pending" => "⋯".yellow().bold(),
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

async fn show_manifest(client: &DerivativeClient, urn: &str, output_format: OutputFormat) -> Result<()> {
    println!("{}", "Fetching manifest...".dimmed());

    let manifest = client.get_manifest(urn).await?;

    match output_format {
        OutputFormat::Table => {
            let status_icon = match manifest.status.as_str() {
                "success" => "✓".green().bold(),
                "failed" | "timeout" => "✗".red().bold(),
                "inprogress" | "pending" => "⋯".yellow().bold(),
                _ => "?".dimmed(),
            };

            println!("\n{}", "Manifest".bold());
            println!("{}", "─".repeat(60));
            println!("  {} {} {}", "Status:".bold(), status_icon, manifest.status);
            println!("  {} {}", "Progress:".bold(), manifest.progress);
            println!("  {} {}", "Region:".bold(), manifest.region);
            println!("  {} {}", "Has Thumbnail:".bold(), manifest.has_thumbnail);

            if let Some(version) = &manifest.version {
                println!("  {} {}", "Version:".bold(), version);
            }

            if !manifest.derivatives.is_empty() {
                println!("\n{}", "Derivatives:".bold());
                println!("{}", "─".repeat(60));

                for derivative in &manifest.derivatives {
                    let status_icon = match derivative.status.as_str() {
                        "success" => "✓".green(),
                        "failed" | "timeout" => "✗".red(),
                        "inprogress" | "pending" => "⋯".yellow(),
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
                            "└".dimmed(),
                            child.name.as_deref().unwrap_or(&child.guid),
                            child.role.dimmed()
                        );
                    }
                }
            }

            println!("{}", "─".repeat(60));
        }
        _ => {
            // For non-table formats, serialize the manifest directly
            output_format.write(&manifest)?;
        }
    }
    Ok(())
}

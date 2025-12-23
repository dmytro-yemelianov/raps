//! Translation commands for Model Derivative API
//!
//! Commands for starting translations, checking status, and viewing manifests.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

use crate::api::{derivative::OutputFormat, DerivativeClient};

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
    pub async fn execute(self, client: &DerivativeClient) -> Result<()> {
        match self {
            TranslateCommands::Start {
                urn,
                format,
                root_filename,
            } => start_translation(client, urn, format, root_filename).await,
            TranslateCommands::Status { urn, wait } => check_status(client, &urn, wait).await,
            TranslateCommands::Manifest { urn } => show_manifest(client, &urn).await,
        }
    }
}

async fn start_translation(
    client: &DerivativeClient,
    urn: Option<String>,
    format: Option<String>,
    root_filename: Option<String>,
) -> Result<()> {
    // Get URN interactively if not provided
    let source_urn = match urn {
        Some(u) => u,
        None => Input::new()
            .with_prompt("Enter the base64-encoded URN")
            .validate_with(|input: &String| -> Result<(), &str> {
                if input.is_empty() {
                    Err("URN cannot be empty")
                } else {
                    Ok(())
                }
            })
            .interact_text()?,
    };

    // Select output format interactively if not provided
    let output_format = match format {
        Some(f) => match f.to_lowercase().as_str() {
            "svf2" => OutputFormat::Svf2,
            "svf" => OutputFormat::Svf,
            "thumbnail" => OutputFormat::Thumbnail,
            "obj" => OutputFormat::Obj,
            "stl" => OutputFormat::Stl,
            "step" => OutputFormat::Step,
            "iges" => OutputFormat::Iges,
            "ifc" => OutputFormat::Ifc,
            _ => anyhow::bail!(
                "Invalid format. Use: svf2, svf, thumbnail, obj, stl, step, iges, ifc"
            ),
        },
        None => {
            let formats = OutputFormat::all();
            let format_labels: Vec<String> = formats.iter().map(|f| f.to_string()).collect();

            let selection = Select::new()
                .with_prompt("Select output format")
                .items(&format_labels)
                .default(0)
                .interact()?;

            formats[selection]
        }
    };

    println!(
        "{} {} {} {}",
        "Starting translation".dimmed(),
        "→".dimmed(),
        output_format.to_string().cyan(),
        "format".dimmed()
    );

    let response = client
        .translate(&source_urn, output_format, root_filename.as_deref())
        .await?;

    println!("{} Translation job started!", "✓".green().bold());
    println!("  {} {}", "Result:".bold(), response.result);
    println!("  {} {}", "URN:".bold(), response.urn);

    if let Some(jobs) = response.accepted_jobs {
        println!("  {} ", "Accepted formats:".bold());
        for format in jobs.output.formats {
            println!("    {} {}", "•".dimmed(), format.format_type.cyan());
        }
    }

    println!(
        "\n{}",
        "Tip: Use 'raps translate status <urn> --wait' to monitor progress".dimmed()
    );

    Ok(())
}

async fn check_status(client: &DerivativeClient, urn: &str, wait: bool) -> Result<()> {
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

        let status_icon = match status.as_str() {
            "success" => "✓".green().bold(),
            "failed" | "timeout" => "✗".red().bold(),
            "inprogress" | "pending" => "⋯".yellow().bold(),
            _ => "?".dimmed(),
        };

        println!("{} {} ({})", status_icon, status, progress);
    }

    Ok(())
}

async fn show_manifest(client: &DerivativeClient, urn: &str) -> Result<()> {
    println!("{}", "Fetching manifest...".dimmed());

    let manifest = client.get_manifest(urn).await?;

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
    Ok(())
}

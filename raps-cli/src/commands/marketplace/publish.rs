// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Publish command handlers: init, package, publish, review

use anyhow::{Context, Result};
use colored::Colorize;

use crate::marketplace::{MarketplaceAuth, MarketplaceClient, PluginInstaller, PluginPublisher};
use crate::output::OutputFormat;
use raps_kernel::marketplace::Installation;

use super::{InitArgs, PackageArgs, PublishArgs, ReviewArgs};
use super::format_status;

pub(super) async fn init(args: InitArgs, output_format: OutputFormat) -> Result<()> {
    use dialoguer::{Input, theme::ColorfulTheme};

    let name = if let Some(n) = args.name {
        n
    } else {
        if raps_kernel::interactive::is_non_interactive() {
            anyhow::bail!("Plugin name is required in non-interactive mode. Use --name flag.");
        }
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Plugin name")
            .interact_text()?
    };

    let author = if let Some(a) = args.author {
        a
    } else {
        if raps_kernel::interactive::is_non_interactive() {
            anyhow::bail!("Author is required in non-interactive mode. Use --author flag.");
        }
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Author")
            .interact_text()?
    };

    let publisher = PluginPublisher::new();
    let dir = std::path::Path::new(&args.dir);
    let manifest_path = publisher.init(dir, &name, &author)?;

    match output_format {
        OutputFormat::Table => {
            println!("{} Created {}", "✓".green().bold(), manifest_path.display());
            println!("\n{}", "Next steps:".bold());
            println!(
                "  1. Edit {} to configure your plugin",
                "raps-plugin.toml".cyan()
            );
            println!("  2. Build your plugin binary for each platform");
            println!(
                "  3. Run {} to create a package",
                "raps marketplace package".cyan()
            );
            println!(
                "  4. Run {} to submit to the marketplace",
                "raps marketplace publish".cyan()
            );
        }
        _ => {
            output_format.write(&serde_json::json!({
                "path": manifest_path.to_string_lossy(),
                "name": name,
                "author": author
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn package(args: PackageArgs, output_format: OutputFormat) -> Result<()> {
    let publisher = PluginPublisher::new();
    let dir = std::path::Path::new(&args.dir);

    let result = publisher.package(dir)?;

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Package created: {}",
                "✓".green().bold(),
                result.path.display()
            );
            println!("  {} {} bytes", "Size:".dimmed(), result.size);
            println!("  {} {}", "Checksum:".dimmed(), result.checksum);
        }
        _ => {
            output_format.write(&serde_json::json!({
                "path": result.path.to_string_lossy(),
                "size": result.size,
                "checksum": result.checksum
            }))?;
        }
    }

    Ok(())
}

pub(super) async fn publish(args: PublishArgs, output_format: OutputFormat) -> Result<()> {
    // Check submission status
    if let Some(submission_id) = args.status {
        return check_submission_status(&submission_id, output_format).await;
    }

    // Publish package
    let auth = MarketplaceAuth::new();
    auth.load_tokens().await?;

    if !auth.is_authenticated().await {
        anyhow::bail!("Please login first: raps marketplace login");
    }

    let publisher = PluginPublisher::new();
    let token: String = auth.get_access_token().await.context("Not authenticated. Run 'raps auth login' first.")?;

    let path = std::path::Path::new(&args.path);
    let package_path = if path.is_dir() {
        // Package first
        let result = publisher.package(path)?;
        result.path
    } else {
        path.to_path_buf()
    };

    let result = publisher.publish(&package_path, &token).await?;

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Submitted {} v{} for review",
                "✓".green().bold(),
                result.plugin_name.cyan(),
                result.version
            );
            println!("  {} {}", "Submission ID:".dimmed(), result.submission_id);
            println!("  {} {}", "Status:".dimmed(), result.status);
            if let Some(ref time) = result.estimated_review_time {
                println!("  {} {}", "Estimated review time:".dimmed(), time);
            }
            println!(
                "\nCheck status with: {}",
                format!("raps marketplace publish --status {}", result.submission_id).cyan()
            );
        }
        _ => {
            output_format.write(&result)?;
        }
    }

    Ok(())
}

async fn check_submission_status(
    submission_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    let auth = MarketplaceAuth::new();
    auth.load_tokens().await?;

    if !auth.is_authenticated().await {
        anyhow::bail!("Please login first: raps marketplace login");
    }

    let publisher = PluginPublisher::new();
    let token: String = auth.get_access_token().await.context("Not authenticated. Run 'raps auth login' first.")?;
    let status = publisher
        .get_submission_status(submission_id, &token)
        .await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Submission Status:".bold());
            println!("{}", "─".repeat(50));
            println!("  {} {}", "ID:".bold(), status.submission_id);
            println!("  {} {}", "Status:".bold(), format_status(&status.status));
            if let Some(ref msg) = status.message {
                println!("  {} {}", "Message:".bold(), msg);
            }
            if let Some(ref feedback) = status.feedback {
                println!("  {} {}", "Feedback:".bold(), feedback);
            }
            if let Some(ref url) = status.plugin_url {
                println!("  {} {}", "URL:".bold(), url);
            }
            println!("{}", "─".repeat(50));
        }
        _ => {
            output_format.write(&status)?;
        }
    }

    Ok(())
}

pub(super) async fn review(args: ReviewArgs, output_format: OutputFormat) -> Result<()> {
    use dialoguer::{Input, Select, theme::ColorfulTheme};

    let auth = MarketplaceAuth::new();
    auth.load_tokens().await?;

    if !auth.is_authenticated().await {
        anyhow::bail!("Please login first: raps marketplace login");
    }

    // Verify plugin is installed
    let client = MarketplaceClient::new();
    let installer = PluginInstaller::new(client.clone())?;

    let installation: Option<Installation> = installer.get_installation(&args.name).await?;
    if installation.is_none() {
        anyhow::bail!("You must have '{}' installed to review it.", args.name);
    }

    let rating = if let Some(r) = args.rating {
        r
    } else {
        if raps_kernel::interactive::is_non_interactive() {
            anyhow::bail!(
                "Rating and Comment are required in non-interactive mode. Use --rating and --comment flags."
            );
        }
        let selections = &[
            "★☆☆☆☆ (1)",
            "★★☆☆☆ (2)",
            "★★★☆☆ (3)",
            "★★★★☆ (4)",
            "★★★★★ (5)",
        ];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Rating")
            .items(selections)
            .default(4)
            .interact()?;
        (selection + 1) as u8
    };

    let comment = if let Some(c) = args.comment {
        Some(c)
    } else {
        if raps_kernel::interactive::is_non_interactive() {
            None
        } else {
            let input: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Comment (optional)")
                .allow_empty(true)
                .interact_text()?;
            if input.is_empty() { None } else { Some(input) }
        }
    };

    let token: String = auth.get_access_token().await.context("Not authenticated. Run 'raps auth login' first.")?;
    let mut client_with_token = client;
    client_with_token.set_token(token);

    let review = client_with_token
        .submit_review(&args.name, rating, comment.as_deref())
        .await?;

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Review submitted for {}",
                "✓".green().bold(),
                args.name.cyan()
            );
            println!("  {} {}", "Rating:".dimmed(), "★".repeat(rating as usize));
            if let Some(c) = comment {
                println!("  {} {}", "Comment:".dimmed(), c);
            }
        }
        _ => {
            output_format.write(&review)?;
        }
    }

    Ok(())
}

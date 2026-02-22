// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Marketplace Commands
//!
//! Commands for interacting with the RAPS Plugin Marketplace.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use serde::Serialize;

use crate::marketplace::{
    CacheManager, MarketplaceAuth, MarketplaceClient, PluginInstaller, PluginPublisher,
    SubscriptionManager,
};
use crate::output::OutputFormat;
use raps_kernel::marketplace::{Installation, Plugin, PluginTier};

/// Marketplace commands
#[derive(Debug, Subcommand)]
pub enum MarketplaceCommands {
    /// Search for plugins in the marketplace
    Search(SearchArgs),

    /// Install a plugin from the marketplace
    Install(InstallArgs),

    /// Uninstall a marketplace plugin
    Uninstall(UninstallArgs),

    /// Check for plugin updates
    Update(UpdateArgs),

    /// Log in to the marketplace
    Login,

    /// Log out from the marketplace
    Logout,

    /// Show subscription status
    Status,

    /// Register an organization license key
    License(LicenseArgs),

    /// Initialize a new plugin project
    Init(InitArgs),

    /// Package a plugin for publication
    Package(PackageArgs),

    /// Publish a plugin to the marketplace
    Publish(PublishArgs),

    /// Submit or view a plugin review
    Review(ReviewArgs),

    /// Clear the local marketplace cache
    ClearCache,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search query
    #[arg(default_value = "")]
    pub query: String,

    /// Filter by tier (basic, pro)
    #[arg(short, long)]
    pub tier: Option<String>,

    /// Filter by category
    #[arg(short, long)]
    pub category: Option<String>,

    /// Sort by (name, downloads, rating, updated)
    #[arg(short, long, default_value = "name")]
    pub sort: String,
}

#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Plugin name
    pub name: String,

    /// Specific version to install
    #[arg(short, long)]
    pub version: Option<String>,
}

#[derive(Debug, Args)]
pub struct UninstallArgs {
    /// Plugin name
    pub name: String,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Plugin name (omit for all plugins)
    pub name: Option<String>,

    /// Check for updates without installing
    #[arg(long)]
    pub check: bool,

    /// Update all plugins
    #[arg(long)]
    pub all: bool,
}

#[derive(Debug, Args)]
pub struct LicenseArgs {
    /// License key to register
    pub key: String,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Plugin name
    #[arg(short, long)]
    pub name: Option<String>,

    /// Author name
    #[arg(short, long)]
    pub author: Option<String>,

    /// Directory to create the manifest in
    #[arg(default_value = ".")]
    pub dir: String,
}

#[derive(Debug, Args)]
pub struct PackageArgs {
    /// Directory containing the plugin
    #[arg(default_value = ".")]
    pub dir: String,
}

#[derive(Debug, Args)]
pub struct PublishArgs {
    /// Package file to publish (or directory to package and publish)
    #[arg(default_value = ".")]
    pub path: String,

    /// Check submission status
    #[arg(long)]
    pub status: Option<String>,
}

#[derive(Debug, Args)]
pub struct ReviewArgs {
    /// Plugin name
    pub name: String,

    /// Rating (1-5 stars)
    #[arg(short, long)]
    pub rating: Option<u8>,

    /// Comment
    #[arg(short, long)]
    pub comment: Option<String>,
}

impl MarketplaceCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            MarketplaceCommands::Search(args) => search(args, output_format).await,
            MarketplaceCommands::Install(args) => install(args, output_format).await,
            MarketplaceCommands::Uninstall(args) => uninstall(args, output_format).await,
            MarketplaceCommands::Update(args) => update(args, output_format).await,
            MarketplaceCommands::Login => login(output_format).await,
            MarketplaceCommands::Logout => logout(output_format).await,
            MarketplaceCommands::Status => status(output_format).await,
            MarketplaceCommands::License(args) => license(args, output_format).await,
            MarketplaceCommands::Init(args) => init(args, output_format).await,
            MarketplaceCommands::Package(args) => package(args, output_format).await,
            MarketplaceCommands::Publish(args) => publish(args, output_format).await,
            MarketplaceCommands::Review(args) => review(args, output_format).await,
            MarketplaceCommands::ClearCache => clear_cache(output_format).await,
        }
    }
}

#[derive(Serialize)]
struct PluginSearchOutput {
    name: String,
    version: String,
    description: String,
    tier: String,
    rating: Option<f32>,
    downloads: u64,
}

async fn search(args: SearchArgs, output_format: OutputFormat) -> Result<()> {
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

async fn install(args: InstallArgs, output_format: OutputFormat) -> Result<()> {
    let client = MarketplaceClient::new();

    // Check if Pro plugin requires authentication
    let plugin: Plugin = client.get_plugin(&args.name).await?;
    if plugin.tier == PluginTier::Pro {
        let auth = MarketplaceAuth::new();
        auth.load_tokens().await?;

        if !auth.is_authenticated().await {
            anyhow::bail!(
                "Plugin '{}' requires a Pro subscription. Run 'raps marketplace login' first.",
                args.name
            );
        }

        let sub_manager = SubscriptionManager::new()?;
        let token: String = auth.get_access_token().await.context("Not authenticated. Run 'raps auth login' first.")?;

        if !sub_manager.can_use_pro(&token).await? {
            anyhow::bail!(
                "Plugin '{}' requires a Pro subscription.\n\
                 Subscribe at: https://marketplace.rapscli.xyz/subscribe",
                args.name
            );
        }
    }

    let installer = PluginInstaller::new(client)?;
    let result = installer
        .install(&args.name, args.version.as_deref())
        .await?;

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Installed {} v{}",
                "✓".green().bold(),
                result.name.cyan(),
                result.version
            );
            println!("  {} {}", "Binary:".dimmed(), result.binary_path.display());

            if result.suggest_path {
                println!();
                println!("{}", "Note:".yellow().bold());
                println!("{}", installer.path_suggestion());
            }
        }
        _ => {
            output_format.write(&serde_json::json!({
                "name": result.name,
                "version": result.version,
                "path": result.binary_path.to_string_lossy(),
            }))?;
        }
    }

    Ok(())
}

async fn uninstall(args: UninstallArgs, output_format: OutputFormat) -> Result<()> {
    let client = MarketplaceClient::new();
    let installer = PluginInstaller::new(client)?;

    installer.uninstall(&args.name).await?;

    match output_format {
        OutputFormat::Table => {
            println!("{} Uninstalled {}", "✓".green().bold(), args.name.cyan());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "name": args.name,
                "uninstalled": true
            }))?;
        }
    }

    Ok(())
}

/// Update info with changelog
struct UpdateInfo {
    name: String,
    current_version: String,
    available_version: String,
    changelog: Option<String>,
    is_pro: bool,
    raps_compatible: bool,
}

async fn update(args: UpdateArgs, output_format: OutputFormat) -> Result<()> {
    let client = MarketplaceClient::new();
    let installer = PluginInstaller::new(client.clone())?;
    let installations: Vec<Installation> = installer.load_registry().await?;

    if installations.is_empty() {
        match output_format {
            OutputFormat::Table => {
                println!("{}", "No marketplace plugins installed.".yellow());
            }
            _ => {
                output_format.write(&serde_json::json!({
                    "updates": []
                }))?;
            }
        }
        return Ok(());
    }

    // Filter installations based on args
    let to_check: Vec<Installation> = if let Some(ref name) = args.name {
        installations
            .into_iter()
            .filter(|i| &i.name == name)
            .collect()
    } else {
        // For --all, --check, or default: check all
        installations
    };

    let mut updates_available: Vec<UpdateInfo> = Vec::new();

    for install in &to_check {
        let Ok(versions) = client.get_versions(&install.name).await else {
            continue;
        };

        let Some(latest) = versions.iter().filter(|v| !v.yanked).max_by(|a, b| {
            semver::Version::parse(&a.version)
                .unwrap_or_else(|_| semver::Version::new(0, 0, 0))
                .cmp(
                    &semver::Version::parse(&b.version)
                        .unwrap_or_else(|_| semver::Version::new(0, 0, 0)),
                )
        }) else {
            continue;
        };

        let current = semver::Version::parse(&install.version)
            .unwrap_or_else(|_| semver::Version::new(0, 0, 0));
        let available = semver::Version::parse(&latest.version)
            .unwrap_or_else(|_| semver::Version::new(0, 0, 0));

        if available > current {
            // Check RAPS version compatibility
            let raps_compatible =
                PluginInstaller::check_raps_compatibility(&latest.raps_compatibility)
                    .unwrap_or(false);

            // Check if it's a Pro plugin
            let is_pro = client
                .get_plugin(&install.name)
                .await
                .map(|p| p.tier == PluginTier::Pro)
                .unwrap_or(false);

            updates_available.push(UpdateInfo {
                name: install.name.clone(),
                current_version: install.version.clone(),
                available_version: latest.version.clone(),
                changelog: latest.changelog.clone(),
                is_pro,
                raps_compatible,
            });
        }
    }

    if args.check || (!args.all && args.name.is_none()) {
        // Just show available updates
        match output_format {
            OutputFormat::Table => {
                if updates_available.is_empty() {
                    println!("{}", "All plugins are up to date.".green());
                } else {
                    println!("\n{}", "Updates Available:".bold());
                    println!("{}", "─".repeat(80));
                    for update in &updates_available {
                        let tier_badge = if update.is_pro {
                            " [PRO]".magenta().to_string()
                        } else {
                            String::new()
                        };
                        let compat_warning = if !update.raps_compatible {
                            " ⚠ incompatible".yellow().to_string()
                        } else {
                            String::new()
                        };
                        println!(
                            "  {}{} {} → {}{}",
                            update.name.cyan(),
                            tier_badge,
                            update.current_version.dimmed(),
                            update.available_version.green(),
                            compat_warning
                        );
                        if let Some(ref changelog) = update.changelog {
                            // Show first line of changelog
                            let first_line = changelog.lines().next().unwrap_or("");
                            if !first_line.is_empty() {
                                println!("    {}", first_line.dimmed());
                            }
                        }
                    }
                    println!("{}", "─".repeat(80));
                    println!(
                        "\nRun {} to update all, or {} to update one.",
                        "raps marketplace update --all".cyan(),
                        "raps marketplace update <name>".cyan()
                    );
                }
            }
            _ => {
                output_format.write(&serde_json::json!({
                    "updates": updates_available.iter().map(|u| {
                        serde_json::json!({
                            "name": u.name,
                            "current": u.current_version,
                            "available": u.available_version,
                            "changelog": u.changelog,
                            "is_pro": u.is_pro,
                            "raps_compatible": u.raps_compatible
                        })
                    }).collect::<Vec<_>>()
                }))?;
            }
        }
    } else {
        // Check Pro subscription if any Pro plugins need updating
        let has_pro_updates = updates_available.iter().any(|u| u.is_pro);
        let mut can_update_pro = false;

        if has_pro_updates {
            let auth = MarketplaceAuth::new();
            auth.load_tokens().await?;

            if auth.is_authenticated().await {
                let sub_manager = SubscriptionManager::new()?;
                let token = auth.get_access_token().await.context("Not authenticated. Run 'raps auth login' first.")?;
                can_update_pro = sub_manager.can_update_pro(&token).await.unwrap_or(false);
            }
        }

        // Perform updates with rollback support
        let mut success_count = 0;
        let mut fail_count = 0;
        let mut skipped_count = 0;

        for update in updates_available {
            // Check RAPS compatibility
            if !update.raps_compatible {
                if let OutputFormat::Table = output_format {
                    println!(
                        "{} Skipping {} - incompatible with current RAPS version",
                        "⚠".yellow().bold(),
                        update.name.cyan()
                    );
                }
                skipped_count += 1;
                continue;
            }

            // Check Pro subscription for Pro plugins
            if update.is_pro && !can_update_pro {
                if let OutputFormat::Table = output_format {
                    println!(
                        "{} Skipping {} - Pro subscription required for updates",
                        "⚠".yellow().bold(),
                        update.name.cyan()
                    );
                    println!(
                        "    Subscribe at: {}",
                        "https://marketplace.rapscli.xyz/subscribe".cyan()
                    );
                }
                skipped_count += 1;
                continue;
            }

            // Perform update with rollback support
            match installer.update_with_rollback(&update.name, None).await {
                Ok(result) => {
                    if let OutputFormat::Table = output_format {
                        println!(
                            "{} Updated {} to v{}",
                            "✓".green().bold(),
                            result.name.cyan(),
                            result.version
                        );
                    }
                    success_count += 1;
                }
                Err(e) => {
                    if let OutputFormat::Table = output_format {
                        println!(
                            "{} Failed to update {}: {}",
                            "✗".red().bold(),
                            update.name,
                            e
                        );
                    }
                    fail_count += 1;
                }
            }
        }

        // Summary for bulk updates
        if args.all && matches!(output_format, OutputFormat::Table) {
            println!("{}", "─".repeat(60));
            println!(
                "{} {} updated, {} failed, {} skipped",
                "Summary:".bold(),
                success_count.to_string().green(),
                fail_count.to_string().red(),
                skipped_count.to_string().yellow()
            );
        }
    }

    Ok(())
}

async fn login(output_format: OutputFormat) -> Result<()> {
    let auth = MarketplaceAuth::new();
    let token_response = auth.login().await?;

    match output_format {
        OutputFormat::Table => {
            println!("{} Logged in successfully!", "✓".green().bold());
            println!(
                "  {} expires in {} seconds",
                "Token".dimmed(),
                token_response.expires_in
            );
        }
        _ => {
            output_format.write(&serde_json::json!({
                "logged_in": true,
                "expires_in": token_response.expires_in
            }))?;
        }
    }

    Ok(())
}

async fn logout(output_format: OutputFormat) -> Result<()> {
    let auth = MarketplaceAuth::new();
    auth.clear_tokens().await?;

    let sub_manager = SubscriptionManager::new()?;
    sub_manager.clear_cache().await?;

    match output_format {
        OutputFormat::Table => {
            println!("{} Logged out successfully", "✓".green().bold());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "logged_out": true
            }))?;
        }
    }

    Ok(())
}

async fn status(output_format: OutputFormat) -> Result<()> {
    let auth = MarketplaceAuth::new();
    auth.load_tokens().await?;

    if !auth.is_authenticated().await {
        match output_format {
            OutputFormat::Table => {
                println!("{}", "Not logged in.".yellow());
                println!("Run {} to authenticate.", "raps marketplace login".cyan());
            }
            _ => {
                output_format.write(&serde_json::json!({
                    "authenticated": false
                }))?;
            }
        }
        return Ok(());
    }

    let sub_manager = SubscriptionManager::new()?;
    let token: String = auth.get_access_token().await.context("Not authenticated. Run 'raps auth login' first.")?;
    let subscription = sub_manager.get_subscription(&token).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Subscription Status:".bold());
            println!("{}", "─".repeat(40));
            println!(
                "{}",
                SubscriptionManager::format_subscription_status(&subscription)
            );
            println!("{}", "─".repeat(40));
        }
        _ => {
            output_format.write(&subscription)?;
        }
    }

    Ok(())
}

async fn license(args: LicenseArgs, output_format: OutputFormat) -> Result<()> {
    let auth = MarketplaceAuth::new();
    auth.load_tokens().await?;

    if !auth.is_authenticated().await {
        anyhow::bail!("Please login first: raps marketplace login");
    }

    let sub_manager = SubscriptionManager::new()?;
    let token: String = auth.get_access_token().await.context("Not authenticated. Run 'raps auth login' first.")?;

    let subscription = sub_manager.register_license(&token, &args.key).await?;

    match output_format {
        OutputFormat::Table => {
            println!("{} License registered!", "✓".green().bold());
            println!(
                "{}",
                SubscriptionManager::format_subscription_status(&subscription)
            );
        }
        _ => {
            output_format.write(&subscription)?;
        }
    }

    Ok(())
}

async fn init(args: InitArgs, output_format: OutputFormat) -> Result<()> {
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

async fn package(args: PackageArgs, output_format: OutputFormat) -> Result<()> {
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

async fn publish(args: PublishArgs, output_format: OutputFormat) -> Result<()> {
    // Check submission status
    if let Some(submission_id) = args.status {
        let auth = MarketplaceAuth::new();
        auth.load_tokens().await?;

        if !auth.is_authenticated().await {
            anyhow::bail!("Please login first: raps marketplace login");
        }

        let publisher = PluginPublisher::new();
        let token: String = auth.get_access_token().await.context("Not authenticated. Run 'raps auth login' first.")?;
        let status = publisher
            .get_submission_status(&submission_id, &token)
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
        return Ok(());
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

async fn review(args: ReviewArgs, output_format: OutputFormat) -> Result<()> {
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

async fn clear_cache(output_format: OutputFormat) -> Result<()> {
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

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn format_status(status: &str) -> String {
    match status {
        "pending" => "Pending".yellow().to_string(),
        "reviewing" => "Under Review".cyan().to_string(),
        "approved" => "Approved".green().to_string(),
        "rejected" => "Rejected".red().to_string(),
        _ => status.to_string(),
    }
}

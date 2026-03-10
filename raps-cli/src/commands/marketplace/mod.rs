// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Marketplace Commands
//!
//! Commands for interacting with the RAPS Plugin Marketplace.

mod auth;
mod discovery;
mod install;
mod publish;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::output::OutputFormat;

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
            MarketplaceCommands::Search(args) => discovery::search(args, output_format).await,
            MarketplaceCommands::Install(args) => install::install(args, output_format).await,
            MarketplaceCommands::Uninstall(args) => install::uninstall(args, output_format).await,
            MarketplaceCommands::Update(args) => install::update(args, output_format).await,
            MarketplaceCommands::Login => auth::login(output_format).await,
            MarketplaceCommands::Logout => auth::logout(output_format).await,
            MarketplaceCommands::Status => auth::status(output_format).await,
            MarketplaceCommands::License(args) => auth::license(args, output_format).await,
            MarketplaceCommands::Init(args) => publish::init(args, output_format).await,
            MarketplaceCommands::Package(args) => publish::package(args, output_format).await,
            MarketplaceCommands::Publish(args) => publish::publish(args, output_format).await,
            MarketplaceCommands::Review(args) => publish::review(args, output_format).await,
            MarketplaceCommands::ClearCache => discovery::clear_cache(output_format).await,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}


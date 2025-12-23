//! APS CLI - Command-line interface for Autodesk Platform Services
//!
//! This CLI tool provides comprehensive access to:
//! - Authentication (2-legged and 3-legged OAuth)
//! - Object Storage Service (OSS): Bucket and object management
//! - Model Derivative: File translation for viewing
//! - Data Management: Hubs, Projects, Folders, Items
//! - Webhooks: Event subscriptions
//! - Design Automation: CAD processing automation
//! - ACC/BIM 360: Issues and RFIs
//! - Reality Capture: Photogrammetry processing

mod api;
mod commands;
mod config;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use colored::Colorize;
use std::io;

use api::{
    AuthClient, DataManagementClient, DerivativeClient, DesignAutomationClient, IssuesClient,
    OssClient, RealityCaptureClient, WebhooksClient,
};
use commands::{
    AuthCommands, BucketCommands, DaCommands, DemoCommands, FolderCommands, GenerateArgs,
    HubCommands, IssueCommands, ItemCommands, ObjectCommands, ProjectCommands, RealityCommands,
    TranslateCommands, WebhookCommands,
};
use config::Config;

/// RAPS - Rust APS CLI - Command-line interface for Autodesk Platform Services
#[derive(Parser)]
#[command(name = "raps")]
#[command(author = "APS Developer")]
#[command(version = "0.3.0")]
#[command(about = "Command-line interface for Autodesk Platform Services (APS)", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authentication management (login, logout, test)
    #[command(subcommand)]
    Auth(AuthCommands),

    /// Manage OSS buckets
    #[command(subcommand)]
    Bucket(BucketCommands),

    /// Manage objects in OSS buckets
    #[command(subcommand)]
    Object(ObjectCommands),

    /// Translate files using Model Derivative API
    #[command(subcommand)]
    Translate(TranslateCommands),

    /// List and manage hubs (requires 3-legged auth)
    #[command(subcommand)]
    Hub(HubCommands),

    /// List and manage projects (requires 3-legged auth)
    #[command(subcommand)]
    Project(ProjectCommands),

    /// List and manage folders (requires 3-legged auth)
    #[command(subcommand)]
    Folder(FolderCommands),

    /// List and manage items/files (requires 3-legged auth)
    #[command(subcommand)]
    Item(ItemCommands),

    /// Manage webhook subscriptions
    #[command(subcommand)]
    Webhook(WebhookCommands),

    /// Design Automation (engines, appbundles, activities, workitems)
    #[command(subcommand)]
    Da(DaCommands),

    /// ACC/BIM 360 Issues management (requires 3-legged auth)
    #[command(subcommand)]
    Issue(IssueCommands),

    /// Reality Capture / Photogrammetry
    #[command(subcommand)]
    Reality(RealityCommands),

    /// Generate synthetic engineering files for testing
    Generate(GenerateArgs),

    /// Run demo scenarios (bucket lifecycle, model pipeline, etc.)
    #[command(subcommand)]
    Demo(DemoCommands),

    /// Generate shell completions for bash, zsh, fish, PowerShell, or elvish
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{} {}", "Error:".red().bold(), err);

        // Print chain of errors
        let mut source = err.source();
        while let Some(cause) = source {
            eprintln!("  {} {}", "Caused by:".dimmed(), cause);
            source = cause.source();
        }

        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Handle completions command first (doesn't need config/auth)
    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = Cli::command();
        generate(*shell, &mut cmd, "raps", &mut io::stdout());
        return Ok(());
    }

    // Load configuration
    let config = Config::from_env()?;

    // Create API clients
    let auth_client = AuthClient::new(config.clone());
    let oss_client = OssClient::new(config.clone(), auth_client.clone());
    let derivative_client = DerivativeClient::new(config.clone(), auth_client.clone());
    let dm_client = DataManagementClient::new(config.clone(), auth_client.clone());
    let webhooks_client = WebhooksClient::new(config.clone(), auth_client.clone());
    let da_client = DesignAutomationClient::new(config.clone(), auth_client.clone());
    let issues_client = IssuesClient::new(config.clone(), auth_client.clone());
    let rc_client = RealityCaptureClient::new(config.clone(), auth_client.clone());

    match cli.command {
        Commands::Auth(cmd) => {
            cmd.execute(&auth_client).await?;
        }

        Commands::Bucket(cmd) => {
            cmd.execute(&oss_client).await?;
        }

        Commands::Object(cmd) => {
            cmd.execute(&oss_client).await?;
        }

        Commands::Translate(cmd) => {
            cmd.execute(&derivative_client).await?;
        }

        Commands::Hub(cmd) => {
            cmd.execute(&dm_client).await?;
        }

        Commands::Project(cmd) => {
            cmd.execute(&dm_client).await?;
        }

        Commands::Folder(cmd) => {
            cmd.execute(&dm_client).await?;
        }

        Commands::Item(cmd) => {
            cmd.execute(&dm_client).await?;
        }

        Commands::Webhook(cmd) => {
            cmd.execute(&webhooks_client).await?;
        }

        Commands::Da(cmd) => {
            cmd.execute(&da_client).await?;
        }

        Commands::Issue(cmd) => {
            cmd.execute(&issues_client).await?;
        }

        Commands::Reality(cmd) => {
            cmd.execute(&rc_client).await?;
        }

        Commands::Generate(args) => {
            commands::generate::execute(args).await?;
        }

        Commands::Demo(cmd) => {
            cmd.execute().await?;
        }

        Commands::Completions { .. } => {
            // Already handled above
            unreachable!()
        }
    }

    Ok(())
}

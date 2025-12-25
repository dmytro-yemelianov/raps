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
mod error;
mod http;
mod interactive;
mod logging;
mod output;
mod storage;

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
    AuthCommands, BucketCommands, ConfigCommands, DaCommands, DemoCommands, FolderCommands,
    GenerateArgs, HubCommands, IssueCommands, ItemCommands, ObjectCommands, ProjectCommands,
    RealityCommands, TranslateCommands, WebhookCommands,
};
use config::Config;
use error::ExitCode;
use output::OutputFormat;

/// RAPS - Rust APS CLI - Command-line interface for Autodesk Platform Services
#[derive(Parser)]
#[command(name = "raps")]
#[command(author = "APS Developer")]
#[command(version = "0.5.0")]
#[command(about = "Command-line interface for Autodesk Platform Services (APS)", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Output format: table, json, yaml, csv, or plain (default: auto-detect)
    #[arg(long, value_name = "FORMAT")]
    output: Option<String>,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,

    /// Print only the result payload (useful with JSON output)
    #[arg(short, long)]
    quiet: bool,

    /// Show verbose output (request summaries)
    #[arg(short, long)]
    verbose: bool,

    /// Show debug output (full trace, secrets redacted)
    #[arg(long)]
    debug: bool,

    /// Non-interactive mode: fail if prompts would be required
    #[arg(long)]
    non_interactive: bool,

    /// Auto-confirm destructive actions
    #[arg(long)]
    yes: bool,

    /// HTTP request timeout in seconds (default: 120)
    #[arg(long, value_name = "SECONDS")]
    timeout: Option<u64>,

    /// Maximum concurrent operations for bulk commands (default: 5)
    #[arg(long, value_name = "N")]
    concurrency: Option<usize>,

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

    /// Configuration management (profiles, settings)
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Generate shell completions for bash, zsh, fish, PowerShell, or elvish
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[tokio::main]
async fn main() {
    // Handle clap errors (invalid arguments) - clap already exits with code 2
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            e.print().unwrap();
            std::process::exit(2); // Invalid arguments
        }
    };

    // Initialize logging flags
    logging::init(cli.no_color, cli.quiet, cli.verbose, cli.debug);

    // Initialize interactive mode flags
    interactive::init(cli.non_interactive, cli.yes);

    if let Err(err) = run(cli).await {
        let exit_code = ExitCode::from_error(&err);

        // Only print errors if not in quiet mode
        if !logging::quiet() {
            eprintln!("{} {}", "Error:".red().bold(), err);

            // Print chain of errors
            let mut source = err.source();
            while let Some(cause) = source {
                eprintln!("  {} {}", "Caused by:".dimmed(), cause);
                source = cause.source();
            }
        }

        exit_code.exit();
    }
}

async fn run(mut cli: Cli) -> Result<()> {
    // Handle completions command first (doesn't need config/auth)
    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = Cli::command();
        generate(*shell, &mut cmd, "raps", &mut io::stdout());
        return Ok(());
    }

    // Handle config commands (they don't need authentication)
    if let Commands::Config(cmd) = std::mem::replace(
        &mut cli.command,
        Commands::Completions { shell: Shell::Bash },
    ) {
        // Determine output format for config commands
        let output_format = if let Some(format_str) = &cli.output {
            Some(format_str.parse()?)
        } else {
            None
        };
        let output_format = OutputFormat::determine(output_format);
        return cmd.execute(output_format).await;
    }

    // Determine output format
    let output_format = if let Some(format_str) = &cli.output {
        Some(format_str.parse()?)
    } else {
        None
    };
    let output_format = OutputFormat::determine(output_format);

    // Log startup info in verbose/debug mode
    if logging::verbose() || logging::debug() {
        logging::log_verbose("RAPS CLI starting...");
    }

    // Load configuration
    let config = Config::from_env()?;

    // Create HTTP client config from CLI flags
    let http_config = http::HttpClientConfig::from_cli_and_env(cli.timeout);

    // Create API clients with shared HTTP config
    let auth_client = AuthClient::new_with_http_config(config.clone(), http_config.clone());
    let oss_client = OssClient::new_with_http_config(config.clone(), auth_client.clone(), http_config.clone());
    let derivative_client = DerivativeClient::new_with_http_config(config.clone(), auth_client.clone(), http_config.clone());
    let dm_client = DataManagementClient::new_with_http_config(config.clone(), auth_client.clone(), http_config.clone());
    let webhooks_client = WebhooksClient::new_with_http_config(config.clone(), auth_client.clone(), http_config.clone());
    let da_client = DesignAutomationClient::new_with_http_config(config.clone(), auth_client.clone(), http_config.clone());
    let issues_client = IssuesClient::new_with_http_config(config.clone(), auth_client.clone(), http_config.clone());
    let rc_client = RealityCaptureClient::new_with_http_config(config.clone(), auth_client.clone(), http_config.clone());

    match cli.command {
        Commands::Auth(cmd) => {
            cmd.execute(&auth_client, output_format).await?;
        }

        Commands::Bucket(cmd) => {
            cmd.execute(&oss_client, output_format).await?;
        }

        Commands::Object(cmd) => {
            cmd.execute(&oss_client, output_format).await?;
        }

        Commands::Translate(cmd) => {
            cmd.execute(&derivative_client, output_format).await?;
        }

        Commands::Hub(cmd) => {
            cmd.execute(&dm_client, output_format).await?;
        }

        Commands::Project(cmd) => {
            cmd.execute(&dm_client, output_format).await?;
        }

        Commands::Folder(cmd) => {
            cmd.execute(&dm_client, output_format).await?;
        }

        Commands::Item(cmd) => {
            cmd.execute(&dm_client, output_format).await?;
        }

        Commands::Webhook(cmd) => {
            cmd.execute(&webhooks_client, output_format).await?;
        }

        Commands::Da(cmd) => {
            cmd.execute(&da_client, output_format).await?;
        }

        Commands::Issue(cmd) => {
            cmd.execute(&issues_client, output_format).await?;
        }

        Commands::Reality(cmd) => {
            cmd.execute(&rc_client, output_format).await?;
        }

        Commands::Generate(args) => {
            commands::generate::execute(args).await?;
        }

        Commands::Demo(cmd) => {
            let concurrency = cli.concurrency.unwrap_or(5);
            cmd.execute(concurrency).await?;
        }

        Commands::Config(_) => {
            // Already handled above
            unreachable!()
        }

        Commands::Completions { .. } => {
            // Already handled above
            unreachable!()
        }
    }

    Ok(())
}

// ============================================================================
// 🌼 RAPS (rapeseed) — Rust Autodesk Platform Services CLI
// Copyright 2024-2025 Dmytro Yemelianov
// SPDX-License-Identifier: Apache-2.0
//
// Allow older format string style - will migrate to inline format in a future PR
#![allow(clippy::uninlined_format_args)]
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// ============================================================================

//! 🌼 RAPS (rapeseed) — Rust Autodesk Platform Services CLI
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
mod mcp;
mod output;
mod plugins;
mod storage;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand, error::ErrorKind};
use clap_complete::{Shell, generate};
use colored::Colorize;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::io;

use api::{
    AuthClient, DataManagementClient, DerivativeClient, DesignAutomationClient, IssuesClient,
    OssClient, RealityCaptureClient, RfiClient, WebhooksClient,
};
use commands::{
    AccCommands, AuthCommands, BucketCommands, ConfigCommands, DaCommands, DemoCommands,
    FolderCommands, GenerateArgs, HubCommands, IssueCommands, ItemCommands, ObjectCommands,
    PipelineCommands, PluginCommands, ProjectCommands, RealityCommands, RfiCommands,
    TranslateCommands, WebhookCommands,
};
use config::Config;
use error::ExitCode;
use output::OutputFormat;

/// 🌼 RAPS (rapeseed) — Rust Autodesk Platform Services CLI
#[derive(Parser)]
#[command(name = "raps")]
#[command(author = "Dmytro Yemelianov <https://rapscli.xyz>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "🌼 RAPS (rapeseed) — Rust Autodesk Platform Services CLI", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Output format: table, json, yaml, csv, or plain (default: auto-detect)
    #[arg(long, value_name = "FORMAT", global = true)]
    output: Option<String>,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    /// Print only the result payload (useful with JSON output)
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Show verbose output (request summaries)
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Show debug output (full trace, secrets redacted)
    #[arg(long, global = true)]
    debug: bool,

    /// Non-interactive mode: fail if prompts would be required
    #[arg(long, global = true)]
    non_interactive: bool,

    /// Auto-confirm destructive actions
    #[arg(long, global = true)]
    yes: bool,

    /// HTTP request timeout in seconds (default: 120)
    #[arg(long, value_name = "SECONDS", global = true)]
    timeout: Option<u64>,

    /// Maximum concurrent operations for bulk commands (default: 5)
    #[arg(long, value_name = "N", global = true)]
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

    /// ACC extended modules: Assets, Submittals, Checklists (requires 3-legged auth)
    #[command(subcommand)]
    Acc(AccCommands),

    /// ACC RFIs (Requests for Information) (requires 3-legged auth)
    #[command(subcommand)]
    Rfi(RfiCommands),

    /// Reality Capture / Photogrammetry
    #[command(subcommand)]
    Reality(RealityCommands),

    /// Manage plugins, hooks, and aliases
    #[command(subcommand)]
    Plugin(PluginCommands),

    /// Generate synthetic engineering files for testing
    Generate(GenerateArgs),

    /// Run demo scenarios (bucket lifecycle, model pipeline, etc.)
    #[command(subcommand)]
    Demo(DemoCommands),

    /// Configuration management (profiles, settings)
    #[command(subcommand)]
    Config(ConfigCommands),

    /// Run pipeline from YAML/JSON file
    #[command(subcommand)]
    Pipeline(PipelineCommands),

    /// Generate shell completions for bash, zsh, fish, PowerShell, or elvish
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Start an interactive shell session
    Shell,

    /// Start MCP (Model Context Protocol) server for AI assistant integration
    Serve,
}

#[tokio::main]
async fn main() {
    // Handle clap errors (invalid arguments) - clap already exits with code 2
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let exit_code = match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
                _ => 2,
            };

            e.print().unwrap();
            std::process::exit(exit_code);
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

async fn run(cli: Cli) -> Result<()> {
    // Handle completions command first (doesn't need config/auth)
    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = Cli::command();
        generate(*shell, &mut cmd, "raps", &mut io::stdout());
        return Ok(());
    }

    // Handle MCP server command
    if let Commands::Serve = &cli.command {
        mcp::server::run_server()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        return Ok(());
    }

    // Handle config commands (they don't need authentication)
    if let Commands::Config(_) = &cli.command {
        // Determine output format for config commands
        let output_format = if let Some(format_str) = &cli.output {
            Some(format_str.parse()?)
        } else {
            None
        };
        let output_format = OutputFormat::determine(output_format);
        // Extract and execute the config command
        if let Commands::Config(cmd) = cli.command {
            return cmd.execute(output_format).await;
        }
        unreachable!()
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
        logging::log_verbose("🌼 RAPS CLI starting...");
    }

    // Load configuration
    let config = Config::from_env()?;

    // Create HTTP client with shared config
    let http_config = http::HttpClientConfig::from_cli_and_env(cli.timeout);

    if let Commands::Shell = cli.command {
        println!("{}", "Welcome to the RAPS interactive shell!".bold());
        println!("Type 'help' for a list of commands, and 'exit' to quit.");

        let mut rl = DefaultEditor::new()?;
        let history_path = ".raps_history";
        let _ = rl.load_history(history_path);

        loop {
            let readline = rl.readline("raps> ");
            match readline {
                Ok(line) => {
                    let _ = rl.add_history_entry(line.as_str());
                    let line = line.trim();

                    if line.is_empty() {
                        continue;
                    }

                    if line == "exit" || line == "quit" {
                        break;
                    }

                    let mut args = shlex::split(line).unwrap_or_default();
                    args.insert(0, "raps".to_string());

                    let sub_cli = match Cli::try_parse_from(&args) {
                        Ok(c) => c,
                        Err(e) => {
                            e.print().unwrap();
                            continue;
                        }
                    };

                    let sub_output_format = if let Some(format_str) = &sub_cli.output {
                        Some(format_str.parse()?)
                    } else {
                        None
                    };
                    let sub_output_format = OutputFormat::determine(sub_output_format);
                    let sub_http_config = http::HttpClientConfig::from_cli_and_env(sub_cli.timeout);

                    if let Err(err) = execute_command(
                        sub_cli.command,
                        &config,
                        &sub_http_config,
                        sub_output_format,
                        sub_cli.concurrency.unwrap_or(5),
                    )
                    .await
                    {
                        eprintln!("{} {}", "Error:".red().bold(), err);
                        let mut source = err.source();
                        while let Some(cause) = source {
                            eprintln!("  {} {}", "Caused by:".dimmed(), cause);
                            source = cause.source();
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
        rl.save_history(history_path).unwrap();
        return Ok(());
    }

    execute_command(
        cli.command,
        &config,
        &http_config,
        output_format,
        cli.concurrency.unwrap_or(5),
    )
    .await?;

    Ok(())
}

async fn execute_command(
    command: Commands,
    config: &Config,
    http_config: &http::HttpClientConfig,
    output_format: OutputFormat,
    concurrency: usize,
) -> Result<()> {
    // Helper closure to create clients on demand
    let get_auth_client =
        || -> AuthClient { AuthClient::new_with_http_config(config.clone(), http_config.clone()) };

    let get_oss_client = || -> OssClient {
        let auth = get_auth_client();
        OssClient::new_with_http_config(config.clone(), auth, http_config.clone())
    };

    let get_derivative_client = || -> DerivativeClient {
        let auth = get_auth_client();
        DerivativeClient::new_with_http_config(config.clone(), auth, http_config.clone())
    };

    let get_dm_client = || -> DataManagementClient {
        let auth = get_auth_client();
        DataManagementClient::new_with_http_config(config.clone(), auth, http_config.clone())
    };

    let get_webhooks_client = || -> WebhooksClient {
        let auth = get_auth_client();
        WebhooksClient::new_with_http_config(config.clone(), auth, http_config.clone())
    };

    let get_da_client = || -> DesignAutomationClient {
        let auth = get_auth_client();
        DesignAutomationClient::new_with_http_config(config.clone(), auth, http_config.clone())
    };

    let get_issues_client = || -> IssuesClient {
        let auth = get_auth_client();
        IssuesClient::new_with_http_config(config.clone(), auth, http_config.clone())
    };

    let get_rc_client = || -> RealityCaptureClient {
        let auth = get_auth_client();
        RealityCaptureClient::new_with_http_config(config.clone(), auth, http_config.clone())
    };

    match command {
        Commands::Auth(cmd) => {
            cmd.execute(&get_auth_client(), output_format).await?;
        }

        Commands::Bucket(cmd) => {
            cmd.execute(&get_oss_client(), output_format).await?;
        }

        Commands::Object(cmd) => {
            cmd.execute(&get_oss_client(), output_format).await?;
        }

        Commands::Translate(cmd) => {
            cmd.execute(&get_derivative_client(), output_format).await?;
        }

        Commands::Hub(cmd) => {
            cmd.execute(&get_dm_client(), output_format).await?;
        }

        Commands::Project(cmd) => {
            cmd.execute(&get_dm_client(), output_format).await?;
        }

        Commands::Folder(cmd) => {
            cmd.execute(&get_dm_client(), output_format).await?;
        }

        Commands::Item(cmd) => {
            cmd.execute(&get_dm_client(), output_format).await?;
        }

        Commands::Webhook(cmd) => {
            cmd.execute(&get_webhooks_client(), output_format).await?;
        }

        Commands::Da(cmd) => {
            cmd.execute(&get_da_client(), output_format).await?;
        }

        Commands::Issue(cmd) => {
            cmd.execute(&get_issues_client(), output_format).await?;
        }

        Commands::Acc(cmd) => {
            let auth_client = get_auth_client();
            let acc_client = api::AccClient::new(config.clone(), auth_client);
            cmd.execute(&acc_client, output_format).await?;
        }

        Commands::Rfi(cmd) => {
            let auth_client = get_auth_client();
            let rfi_client =
                RfiClient::new_with_http_config(config.clone(), auth_client, http_config.clone());
            cmd.execute(&rfi_client, output_format).await?;
        }

        Commands::Reality(cmd) => {
            cmd.execute(&get_rc_client(), output_format).await?;
        }

        Commands::Plugin(cmd) => {
            cmd.execute(output_format)?;
        }

        Commands::Generate(args) => {
            commands::generate::execute(args).await?;
        }

        Commands::Demo(cmd) => {
            cmd.execute(concurrency).await?;
        }

        Commands::Config(_) => {
            unreachable!()
        }

        Commands::Pipeline(cmd) => cmd.execute(output_format).await?,

        Commands::Completions { .. } => {
            unreachable!()
        }

        Commands::Shell => {
            unreachable!()
        }

        Commands::Serve => {
            unreachable!()
        }
    }

    Ok(())
}

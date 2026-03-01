// ============================================================================
// RAPS (rapeseed) - Rust Autodesk Platform Services CLI
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

//! RAPS (rapeseed) - Rust Autodesk Platform Services CLI
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

mod commands;
mod credits;
mod mcp;
mod output;
mod plugins;
mod shell;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand, error::ErrorKind};
use clap_complete::{Shell, generate};
use colored::Colorize;
use reedline::{
    ColumnarMenu, Emacs, FileBackedHistory, KeyCode, KeyModifiers, MenuBuilder, Reedline,
    ReedlineEvent, ReedlineMenu, Signal, default_emacs_keybindings,
};
use std::io::{self, BufRead, IsTerminal};

use commands::{
    AccCommands, AdminCommands, ApiCommands, AuthCommands, BucketCommands, ConfigCommands,
    DaCommands, DemoCommands, FolderCommands, GenerateArgs, HubCommands, IssueCommands,
    ItemCommands, ObjectCommands, PipelineCommands, PluginCommands, ProjectCommands,
    RealityCommands, ReportCommands, RfiCommands, TemplateCommands, TranslateCommands,
    WebhookCommands,
};

use raps_acc::admin::AccountAdminClient;
use raps_acc::permissions::FolderPermissionsClient;
use raps_acc::users::ProjectUsersClient;
use raps_acc::{AccClient, IssuesClient, RfiClient};
use raps_da::DesignAutomationClient;
use raps_derivative::DerivativeClient;
use raps_dm::DataManagementClient;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::error::ExitCode;
use raps_kernel::http::HttpClientConfig;
use raps_kernel::interactive;
use raps_kernel::logging;
// use raps_kernel::output::OutputFormat; // Removed old import
use crate::output::OutputFormat;
use raps_oss::OssClient;
use raps_reality::RealityCaptureClient;
use raps_webhooks::WebhooksClient;

/// Grouped commands help text, organized by authentication requirement
const GROUPED_COMMANDS_HELP: &str = "\
\x1b[1;33m2-Legged Auth (Client Credentials)\x1b[0m
  auth          Authentication management (login, logout, test)
  bucket        Manage OSS buckets
  object        Manage objects in OSS buckets
  translate     Translate files using Model Derivative API
  webhook       Manage webhook subscriptions
  da            Design Automation (engines, appbundles, activities, workitems)
  reality       Reality Capture / Photogrammetry
  api           Execute custom API calls to APS endpoints
  inspect       Inspect archive contents via HTTP Range (no full download)

\x1b[1;36m3-Legged Auth (User Login)\x1b[0m
  hub           List and manage hubs
  project       List and manage projects
  folder        List and manage folders
  item          List and manage items/files
  issue         ACC/BIM 360 Issues management
  acc           ACC extended modules: Assets, Submittals, Checklists
  rfi           ACC RFIs (Requests for Information)
  admin         Account admin bulk management (add/remove users, update roles)
  report        Portfolio reports (RFI summary, issues summary across projects)
  template      Project templates management (create, list, update, archive)

\x1b[1;32mUtility (No Auth Required)\x1b[0m
  config        Configuration management (profiles, settings)
  completions   Generate shell completions for bash, zsh, fish, PowerShell
  shell         Start an interactive shell session
  serve         Start MCP server for AI assistant integration
  generate      Generate synthetic engineering files for testing
  demo          Run demo scenarios (bucket lifecycle, model pipeline, etc.)
  cache         Manage the download cache (stats, clear)
  pipeline      Run pipeline from YAML/JSON file
  plugin        Manage plugins, hooks, and aliases
  man           Generate man pages for raps and its subcommands";

/// RAPS (rapeseed) - Rust Autodesk Platform Services CLI
#[derive(Parser)]
#[command(name = "raps")]
#[command(author = "Dmytro Yemelianov <https://rapscli.xyz>")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "RAPS (rapeseed) - Rust Autodesk Platform Services CLI", long_about = None)]
#[command(propagate_version = true)]
#[command(help_template = concat!(
    "\x1b[1;33mraps\x1b[0m v",
    env!("CARGO_PKG_VERSION"),
    " \x1b[32m·\x1b[0m Rust Autodesk Platform Services CLI\n",
    "\n{usage-heading} {usage}\n\n{all-args}\n{after-help}"
))]
#[command(after_help = GROUPED_COMMANDS_HELP)]
struct Cli {
    /// Output format: table, json, yaml, csv, or plain (default: auto-detect)
    #[arg(long, value_name = "FORMAT", global = true)]
    output: Option<OutputFormat>,

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

    /// Profile performance (execution time, HTTP network time)
    #[arg(long, global = true)]
    profile: bool,

    /// Non-interactive mode: fail if prompts would be required
    #[arg(long, global = true)]
    non_interactive: bool,

    /// Auto-confirm destructive actions
    #[arg(long, global = true)]
    yes: bool,

    /// Strict mode for CI: fail on any ambiguity, no silent defaults
    #[arg(long, global = true)]
    strict: bool,

    /// HTTP request timeout in seconds (default: 120)
    #[arg(long, value_name = "SECONDS", global = true)]
    timeout: Option<u64>,

    /// Maximum number of retries for 429/5xx responses (default: 3)
    #[arg(long, value_name = "N", global = true)]
    max_retries: Option<u32>,

    /// Base delay in seconds for exponential backoff (default: 1)
    #[arg(long, value_name = "SECONDS", global = true)]
    base_delay: Option<u64>,

    /// Maximum wait time in seconds between retries (default: 60)
    #[arg(long, value_name = "SECONDS", global = true)]
    max_wait: Option<u64>,

    /// Maximum concurrent operations for bulk commands (default: 5)
    #[arg(long, value_name = "N", global = true)]
    concurrency: Option<usize>,

    /// Disable the download cache
    #[arg(long, global = true)]
    no_cache: bool,

    /// Custom cache directory (default: platform cache dir)
    #[arg(long, value_name = "PATH", global = true)]
    cache_dir: Option<std::path::PathBuf>,

    /// Ignore cached artifacts and re-download
    #[arg(long, global = true)]
    refresh: bool,

    /// Fail if artifact is not cached (no network requests for downloads)
    #[arg(long, global = true)]
    offline: bool,

    #[command(subcommand)]
    command: Option<Commands>,
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

    /// List and manage hubs
    #[command(subcommand)]
    Hub(HubCommands),

    /// List and manage projects
    #[command(subcommand)]
    Project(ProjectCommands),

    /// List and manage folders
    #[command(subcommand)]
    Folder(FolderCommands),

    /// List and manage items/files
    #[command(subcommand)]
    Item(ItemCommands),

    /// Manage webhook subscriptions
    #[command(subcommand)]
    Webhook(WebhookCommands),

    /// Design Automation (engines, appbundles, activities, workitems)
    #[command(subcommand)]
    Da(DaCommands),

    /// ACC/BIM 360 Issues management
    #[command(subcommand)]
    Issue(IssueCommands),

    /// ACC extended modules: Assets, Submittals, Checklists
    #[command(subcommand)]
    Acc(AccCommands),

    /// Account admin bulk management (add/remove users, update roles, folder rights)
    #[command(subcommand)]
    Admin(AdminCommands),

    /// Execute custom API calls to APS endpoints
    #[command(subcommand)]
    Api(ApiCommands),

    /// ACC RFIs (Requests for Information)
    #[command(subcommand)]
    Rfi(RfiCommands),

    /// Portfolio reports (RFI summary, issues summary across projects)
    #[command(subcommand)]
    Report(ReportCommands),

    /// Project templates management (create, list, update, archive)
    #[command(subcommand)]
    Template(TemplateCommands),

    /// Reality Capture / Photogrammetry
    #[command(subcommand)]
    Reality(RealityCommands),

    /// Inspect archive contents via HTTP Range requests (no full download)
    #[command(subcommand)]
    Inspect(commands::inspect::InspectCommands),

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

    /// Open the interactive TUI dashboard (requires --features dashboard)
    #[cfg(feature = "dashboard")]
    Dashboard,

    /// Start MCP (Model Context Protocol) server for AI assistant integration
    Serve,

    /// Manage the download cache (stats, clear)
    #[command(subcommand)]
    Cache(commands::cache::CacheCommands),

    /// Generate JSON Schema for CLI output types
    #[command(subcommand)]
    Schema(commands::schema::SchemaCommands),

    /// Generate man pages for raps and its subcommands
    Man {
        /// Output directory for all man pages (default: print main page to stdout)
        #[arg(long, value_name = "DIR")]
        output_dir: Option<std::path::PathBuf>,
    },

    /// External plugins and custom commands
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[tokio::main]
async fn main() -> Result<()> {
    raps_kernel::profiler::init();

    // Handle clap errors (invalid arguments) - clap already exits with code 2
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let exit_code = match e.kind() {
                ErrorKind::DisplayVersion => {
                    credits::print_version();
                    0
                }
                ErrorKind::DisplayHelp => {
                    print!("{}", include_str!("../logo.ansi"));
                    let _ = e.print();
                    0
                }
                _ => {
                    let _ = e.print();
                    2
                }
            };
            std::process::exit(exit_code);
        }
    };

    // Initialize logging
    logging::init(cli.no_color, cli.quiet, cli.verbose, cli.debug);

    if cli.profile {
        raps_kernel::profiler::enable();
    }

    // Kernel base is successfully loaded and initialized
    raps_kernel::profiler::mark_kernel_loaded();

    // Setup interactive shell check flags
    interactive::init_full(cli.non_interactive, cli.yes, cli.strict);

    // Initialize download cache
    raps_kernel::cache::init(!cli.no_cache, cli.offline, cli.refresh);
    if let Some(ref dir) = cli.cache_dir {
        raps_kernel::cache::set_cache_dir(dir.clone());
    }

    let cmd_name = cli.command.as_ref().map(command_name).unwrap_or("help");

    if let Err(err) = run(cli).await {
        let exit_code = ExitCode::from_error(&err);

        // Log errors to file log for diagnostics
        tracing::error!(error = %err, exit_code = exit_code as i32, "Command failed");

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

        raps_kernel::profiler::report(Some(cmd_name), Some(exit_code as i32));
        raps_kernel::logging::flush();
        exit_code.exit();
    }

    raps_kernel::profiler::report(Some(cmd_name), Some(0));
    raps_kernel::logging::flush();
    Ok(())
}

async fn run(cli: Cli) -> Result<()> {
    // Default behaviour when no subcommand is given
    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            // If stdin is piped, read commands line-by-line (batch mode)
            if !io::stdin().is_terminal() {
                return run_piped_stdin(cli.timeout, cli.output, cli.concurrency).await;
            }
            print!("{}", include_str!("../logo.ansi"));
            Cli::command().print_help()?;
            return Ok(());
        }
    };

    // Handle completions command first (doesn't need config/auth)
    if let Commands::Completions { shell } = &command {
        let mut cmd = Cli::command();
        generate(*shell, &mut cmd, "raps", &mut io::stdout());
        return Ok(());
    }

    // Handle man page generation (doesn't need config/auth)
    if let Commands::Man { ref output_dir } = command {
        let cmd = Cli::command();
        if let Some(dir) = output_dir {
            std::fs::create_dir_all(dir)?;
            clap_mangen::generate_to(cmd, dir)?;
            println!("Man pages generated in {}", dir.display());
        } else if !io::stdout().is_terminal() {
            // Piped: output raw troff (e.g. `raps man | man -l -`)
            let man = clap_mangen::Man::new(cmd);
            man.render(&mut io::stdout())?;
        } else {
            // Interactive terminal: show usage hints
            #[cfg(windows)]
            {
                eprintln!(
                    "{}",
                    "Man pages use troff format (no native viewer on Windows).".yellow()
                );
                eprintln!();
                eprintln!(
                    "Generate to files:  {}",
                    "raps man --output-dir ./man-pages".green()
                );
                eprintln!("Pipe via WSL:       {}", "raps man | wsl man -l -".green());
                eprintln!("Raw troff to file:  {}", "raps man > raps.1".green());
            }
            #[cfg(not(windows))]
            {
                eprintln!("{}", "Hint: pipe to man for rendered output.".yellow());
                eprintln!();
                eprintln!("View now:           {}", "raps man | man -l -".green());
                eprintln!(
                    "Generate to files:  {}",
                    "raps man --output-dir ./man-pages".green()
                );
                eprintln!("Save to file:       {}", "raps man > raps.1".green());
            }
        }
        return Ok(());
    }

    // Handle dashboard command (before config loading, it manages its own setup)
    #[cfg(feature = "dashboard")]
    if let Commands::Dashboard = &command {
        let config = Config::from_env_lenient()?;
        let http_config = HttpClientConfig::from_cli_and_env_full(cli.timeout, cli.max_retries, cli.base_delay, cli.max_wait);
        return commands::dashboard::run_dashboard(config, http_config).await;
    }

    // Handle MCP server command
    if let Commands::Serve = &command {
        mcp::server::run_server()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        return Ok(());
    }

    // Handle config commands (they don't need authentication)
    if let Commands::Config(_) = &command {
        // Determine output format for config commands
        let output_format = OutputFormat::determine(cli.output);
        // Extract and execute the config command
        if let Commands::Config(cmd) = command {
            return cmd.execute(output_format).await;
        }
        unreachable!()
    }

    // Determine output format
    let output_format = OutputFormat::determine(cli.output);

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "RAPS CLI starting");

    // Load configuration leniently — missing credentials default to empty strings.
    // Commands that need credentials will fail with a clear API error when they
    // attempt to authenticate, rather than blocking all commands upfront.
    let config = Config::from_env_lenient()?;

    // Create HTTP client with shared config
    let http_config = HttpClientConfig::from_cli_and_env_full(cli.timeout, cli.max_retries, cli.base_delay, cli.max_wait);

    if let Commands::Shell = command {
        credits::shell_welcome();
        println!("{}", "Welcome to the RAPS interactive shell!".bold());
        println!("Type 'help' for a list of commands, 'exit' to quit.");
        println!(
            "Use TAB for completion, {} hints show required parameters.",
            "cyan".cyan()
        );
        println!();

        // History file — stored in config directory, fallback to CWD
        let history_path = directories::ProjectDirs::from("com", "autodesk", "raps")
            .map(|d| d.config_dir().join("history"))
            .unwrap_or_else(|| std::path::PathBuf::from(".raps_history"));
        let history: Box<dyn reedline::History> =
            match FileBackedHistory::with_file(1000, history_path) {
                Ok(h) => Box::new(h),
                Err(e) => {
                    tracing::warn!("Could not initialize command history: {e}");
                    Box::new(FileBackedHistory::default())
                }
            };

        // Keybindings: Tab -> completion menu
        let mut keybindings = default_emacs_keybindings();
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Menu("completion_menu".to_string()),
                ReedlineEvent::MenuNext,
            ]),
        );

        let edit_mode = Box::new(Emacs::new(keybindings));
        let completer = Box::new(shell::RapsCompleter::new());
        let hinter = Box::new(shell::RapsHinter::new());
        let highlighter = Box::new(shell::RapsHighlighter::new());
        let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));

        let mut editor = Reedline::create()
            .with_history(history)
            .with_completer(completer)
            .with_hinter(hinter)
            .with_highlighter(highlighter)
            .with_edit_mode(edit_mode)
            .with_menu(ReedlineMenu::EngineCompleter(completion_menu));

        let prompt = shell::RapsPrompt;

        loop {
            match editor.read_line(&prompt) {
                Ok(Signal::Success(line)) => {
                    let line = line.trim();

                    if line.is_empty() {
                        continue;
                    }

                    if line == "exit" || line == "quit" {
                        break;
                    }

                    // Handle help command specially for shell context
                    if line == "help" || line == "?" {
                        println!();
                        println!("{}", "2-Legged Auth (Client Credentials)".yellow().bold());
                        println!(
                            "  {:<16} Authentication (login, logout, status, test, whoami)",
                            "auth".cyan()
                        );
                        println!(
                            "  {:<16} Bucket operations (list, create, get, delete)",
                            "bucket".cyan()
                        );
                        println!(
                            "  {:<16} Object operations (list, upload, download, delete)",
                            "object".cyan()
                        );
                        println!(
                            "  {:<16} Model Derivative (start, status, manifest, metadata)",
                            "translate".cyan()
                        );
                        println!(
                            "  {:<16} Webhook management (list, create, get, delete)",
                            "webhook".cyan()
                        );
                        println!(
                            "  {:<16} Design Automation (engines, appbundles, activities)",
                            "da".cyan()
                        );
                        println!(
                            "  {:<16} Reality Capture / Photogrammetry",
                            "reality".cyan()
                        );
                        println!(
                            "  {:<16} Execute custom API calls to APS endpoints",
                            "api".cyan()
                        );
                        println!();
                        println!("{}", "3-Legged Auth (User Login)".cyan().bold());
                        println!("  {:<16} Hub operations (list, get)", "hub".cyan());
                        println!("  {:<16} Project operations (list, get)", "project".cyan());
                        println!(
                            "  {:<16} Folder operations (list, get, create)",
                            "folder".cyan()
                        );
                        println!("  {:<16} Item operations (get, versions)", "item".cyan());
                        println!(
                            "  {:<16} ACC/BIM 360 Issues (list, get, create)",
                            "issue".cyan()
                        );
                        println!(
                            "  {:<16} ACC extended modules (assets, submittals, checklists)",
                            "acc".cyan()
                        );
                        println!("  {:<16} ACC RFIs (list, get)", "rfi".cyan());
                        println!("  {:<16} Account admin bulk management", "admin".cyan());
                        println!(
                            "  {:<16} Portfolio reports (RFI/issues summary)",
                            "report".cyan()
                        );
                        println!("  {:<16} Project templates management", "template".cyan());
                        println!();
                        println!("{}", "Utility (No Auth Required)".green().bold());
                        println!("  {:<16} Configuration management", "config".cyan());
                        println!("  {:<16} Generate shell completions", "completions".cyan());
                        println!(
                            "  {:<16} Generate synthetic files for testing",
                            "generate".cyan()
                        );
                        println!("  {:<16} Run demo scenarios", "demo".cyan());
                        println!("  {:<16} Run pipeline from YAML/JSON", "pipeline".cyan());
                        println!(
                            "  {:<16} Manage plugins, hooks, and aliases",
                            "plugin".cyan()
                        );
                        println!("  {:<16} Generate man pages", "man".cyan());
                        println!("  {:<16} Exit the shell", "exit".cyan());
                        println!();
                        println!("{}", "Key bindings:".bold());
                        println!("  {}        Show completions", "TAB".green());
                        println!("  {}      Accept hint completion", "Right".green());
                        println!("  {}     History navigation", "Up/Down".green());
                        println!("  {}     Cancel current line", "Ctrl-C".green());
                        println!("  {}     Exit shell", "Ctrl-D".green());
                        println!();
                        println!("{}", "Tips:".bold());
                        println!("  * {} hints show required parameters", "Cyan text".cyan());
                        println!(
                            "  * Use {} or {} for command help",
                            "<command> --help".green(),
                            "<command> -h".green()
                        );
                        continue;
                    }

                    let mut args = match shlex::split(line) {
                        Some(args) => args,
                        None => {
                            println!(
                                "{} Unmatched quote or escape character",
                                "Error:".red().bold()
                            );
                            continue;
                        }
                    };
                    args.insert(0, "raps".to_string());

                    let sub_cli = match Cli::try_parse_from(&args) {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = e.print();
                            continue;
                        }
                    };

                    let sub_output_format = OutputFormat::determine(sub_cli.output);
                    let sub_http_config = HttpClientConfig::from_cli_and_env_full(sub_cli.timeout, sub_cli.max_retries, sub_cli.base_delay, sub_cli.max_wait);

                    let sub_command = match sub_cli.command {
                        Some(cmd) => cmd,
                        None => {
                            println!("Use 'help' for available commands, 'exit' to quit.");
                            continue;
                        }
                    };

                    if let Err(err) = execute_command(
                        sub_command,
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
                Ok(Signal::CtrlC) => {
                    println!("CTRL-C");
                    break;
                }
                Ok(Signal::CtrlD) => {
                    println!("CTRL-D");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }
        return Ok(());
    }

    execute_command(
        command,
        &config,
        &http_config,
        output_format,
        cli.concurrency.unwrap_or(5),
    )
    .await?;

    Ok(())
}

/// Read commands from piped stdin and execute each line as a raps command.
/// Supports: `echo "bucket list" | raps` or `Get-Content commands.txt | raps`
async fn run_piped_stdin(
    timeout: Option<u64>,
    output: Option<OutputFormat>,
    concurrency: Option<usize>,
) -> Result<()> {
    let config = Config::from_env_lenient()?;
    let mut exit_code = 0i32;

    let stdin = io::stdin().lock();
    for line in stdin.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut args = match shlex::split(line) {
            Some(args) => args,
            None => {
                eprintln!("{} Unmatched quote in: {}", "Error:".red().bold(), line);
                exit_code = 2;
                continue;
            }
        };
        args.insert(0, "raps".to_string());

        let sub_cli = match Cli::try_parse_from(&args) {
            Ok(c) => c,
            Err(e) => {
                let _ = e.print();
                exit_code = 2;
                continue;
            }
        };

        let sub_output_format = OutputFormat::determine(sub_cli.output.or(output));
        let sub_http_config = HttpClientConfig::from_cli_and_env_full(sub_cli.timeout.or(timeout), sub_cli.max_retries, sub_cli.base_delay, sub_cli.max_wait);

        let sub_command = match sub_cli.command {
            Some(cmd) => cmd,
            None => continue,
        };

        if let Err(err) = execute_command(
            sub_command,
            &config,
            &sub_http_config,
            sub_output_format,
            sub_cli.concurrency.or(concurrency).unwrap_or(5),
        )
        .await
        {
            let code = ExitCode::from_error(&err);
            exit_code = code as i32;
            eprintln!("{} {}", "Error:".red().bold(), err);
            let mut source = err.source();
            while let Some(cause) = source {
                eprintln!("  {} {}", "Caused by:".dimmed(), cause);
                source = cause.source();
            }
        }
    }

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

fn command_name(cmd: &Commands) -> &'static str {
    match cmd {
        Commands::Auth(_) => "auth",
        Commands::Bucket(_) => "bucket",
        Commands::Object(_) => "object",
        Commands::Translate(_) => "translate",
        Commands::Hub(_) => "hub",
        Commands::Project(_) => "project",
        Commands::Folder(_) => "folder",
        Commands::Item(_) => "item",
        Commands::Webhook(_) => "webhook",
        Commands::Da(_) => "da",
        Commands::Issue(_) => "issue",
        Commands::Acc(_) => "acc",
        Commands::Admin(_) => "admin",
        Commands::Api(_) => "api",
        Commands::Rfi(_) => "rfi",
        Commands::Report(_) => "report",
        Commands::Template(_) => "template",
        Commands::Reality(_) => "reality",
        Commands::Inspect(_) => "inspect",
        Commands::Plugin(_) => "plugin",
        Commands::Generate(_) => "generate",
        Commands::Demo(_) => "demo",
        Commands::Config(_) => "config",
        Commands::Pipeline(_) => "pipeline",
        Commands::Completions { .. } => "completions",
        Commands::Shell => "shell",
        #[cfg(feature = "dashboard")]
        Commands::Dashboard => "dashboard",
        Commands::Serve => "serve",
        Commands::Cache(_) => "cache",
        Commands::Schema(_) => "schema",
        Commands::Man { .. } => "man",
        Commands::External(_) => "external",
    }
}

async fn execute_command(
    command: Commands,
    config: &Config,
    http_config: &HttpClientConfig,
    output_format: OutputFormat,
    concurrency: usize,
) -> Result<()> {
    // Log command name for diagnostics
    let cmd_name = command_name(&command);
    tracing::info!(command = cmd_name, "Executing command");

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

    let get_admin_client = || -> AccountAdminClient {
        let auth = get_auth_client();
        AccountAdminClient::new_with_http_config(config.clone(), auth, http_config.clone())
    };

    let _get_project_users_client = || -> ProjectUsersClient {
        let auth = get_auth_client();
        ProjectUsersClient::new_with_http_config(config.clone(), auth, http_config.clone())
    };

    let get_permissions_client = || -> FolderPermissionsClient {
        let auth = get_auth_client();
        FolderPermissionsClient::new_with_http_config(config.clone(), auth, http_config.clone())
    };

    match command {
        Commands::Auth(cmd) => {
            cmd.execute(&get_auth_client(), output_format).await?;
        }

        Commands::Bucket(cmd) => {
            cmd.execute(&get_oss_client(), output_format).await?;
        }

        Commands::Object(cmd) => {
            cmd.execute(&get_oss_client(), output_format, concurrency)
                .await?;
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
            cmd.execute(&get_dm_client(), &get_permissions_client(), output_format)
                .await?;
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
            let acc_client = AccClient::new(config.clone(), auth_client);
            cmd.execute(&acc_client, output_format).await?;
        }

        Commands::Admin(cmd) => {
            let auth_client = get_auth_client();
            cmd.execute(config, &auth_client, output_format, concurrency)
                .await?;
        }

        Commands::Api(cmd) => {
            let auth_client = get_auth_client();
            cmd.execute(config, &auth_client, http_config, output_format)
                .await?;
        }

        Commands::Rfi(cmd) => {
            let auth_client = get_auth_client();
            let rfi_client =
                RfiClient::new_with_http_config(config.clone(), auth_client, http_config.clone());
            cmd.execute(&rfi_client, &get_dm_client(), output_format)
                .await?;
        }

        Commands::Report(cmd) => {
            let auth_client = get_auth_client();
            cmd.execute(config, &auth_client, output_format).await?;
        }

        Commands::Template(cmd) => {
            cmd.execute(&get_admin_client(), output_format).await?;
        }

        Commands::Reality(cmd) => {
            cmd.execute(&get_rc_client(), output_format).await?;
        }

        Commands::Inspect(cmd) => {
            cmd.execute(&get_oss_client(), output_format).await?;
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

        Commands::Cache(cmd) => {
            cmd.execute(output_format)?;
        }

        Commands::Schema(cmd) => {
            cmd.execute()?;
        }

        Commands::Man { .. } => {
            unreachable!()
        }

        #[cfg(feature = "dashboard")]
        Commands::Dashboard => {
            unreachable!()
        }

        Commands::External(args) => {
            if args.is_empty() {
                anyhow::bail!("No plugin name provided");
            }
            let plugin_name = &args[0];
            let mut plugin_args = vec![];
            for arg in &args[1..] {
                plugin_args.push(arg.clone());
            }

            // We need to pass global flags to the plugin if necessary,
            // but for now we just pass what was given to the external subcommand.
            let pm = crate::plugins::PluginManager::new()?;
            let exec_args: Vec<&str> = plugin_args.iter().map(|s| s.as_str()).collect();

            // Execute plugin will return the exit code
            let code = pm.execute_plugin(plugin_name, &exec_args)?;
            if code != 0 {
                anyhow::bail!("Plugin '{}' exited with code {}", plugin_name, code);
            }
        }
    }

    Ok(())
}

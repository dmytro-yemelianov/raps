// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account Admin Bulk Management Commands
//!
//! Commands for bulk user management across ACC/BIM 360 projects:
//! - Add users to multiple projects
//! - Remove users from multiple projects
//! - Update user roles across projects
//! - Manage folder-level permissions

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::{Subcommand, ValueEnum};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use uuid::Uuid;

use raps_acc::admin::AccountAdminClient;
use raps_acc::users::ProjectUsersClient;
use raps_admin::{
    BulkConfig, BulkOperationResult, ItemResult, OperationStatus, PermissionLevel, ProgressUpdate,
    ProjectFilter, StateManager, bulk_add_user,
};
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

use crate::output::OutputFormat;

/// Account admin bulk management commands
#[derive(Debug, Subcommand)]
pub enum AdminCommands {
    /// Bulk user management operations
    #[command(subcommand)]
    User(UserCommands),

    /// Bulk folder permission management
    #[command(subcommand)]
    Folder(FolderCommands),

    /// Project listing with filtering
    #[command(subcommand)]
    Project(AdminProjectCommands),

    /// Bulk operation management (status, resume, cancel)
    #[command(subcommand)]
    Operation(OperationCommands),
}

/// User management subcommands
#[derive(Debug, Subcommand)]
pub enum UserCommands {
    /// Add a user to multiple projects
    Add {
        /// Email address of the user to add
        email: String,

        /// Account ID (defaults to current profile account)
        #[arg(short, long)]
        account: Option<String>,

        /// Role to assign (e.g., "Project Admin", "Document Manager")
        #[arg(short, long)]
        role: Option<String>,

        /// Project filter expression (e.g., "name:*Hospital*,status:active")
        #[arg(short, long)]
        filter: Option<String>,

        /// File containing project IDs (one per line)
        #[arg(long, value_name = "FILE")]
        project_ids: Option<PathBuf>,

        /// Parallel requests (default: 10, max: 50)
        #[arg(long, default_value = "10")]
        concurrency: usize,

        /// Preview changes without executing
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Remove a user from multiple projects
    Remove {
        /// Email address of the user to remove
        email: String,

        /// Account ID
        #[arg(short, long)]
        account: Option<String>,

        /// Project filter expression
        #[arg(short, long)]
        filter: Option<String>,

        /// File containing project IDs (one per line)
        #[arg(long, value_name = "FILE")]
        project_ids: Option<PathBuf>,

        /// Parallel requests (default: 10, max: 50)
        #[arg(long, default_value = "10")]
        concurrency: usize,

        /// Preview changes without executing
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Update user roles across multiple projects
    Update {
        /// Email address of the user to update
        email: String,

        /// Account ID
        #[arg(short, long)]
        account: Option<String>,

        /// New role to assign (required)
        #[arg(short, long)]
        role: String,

        /// Only update users with this current role
        #[arg(long)]
        from_role: Option<String>,

        /// Project filter expression
        #[arg(short, long)]
        filter: Option<String>,

        /// File containing project IDs (one per line)
        #[arg(long, value_name = "FILE")]
        project_ids: Option<PathBuf>,

        /// Parallel requests (default: 10, max: 50)
        #[arg(long, default_value = "10")]
        concurrency: usize,

        /// Preview changes without executing
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

/// Folder permission management subcommands
#[derive(Debug, Subcommand)]
pub enum FolderCommands {
    /// Update folder permissions for a user across projects
    Rights {
        /// Email address of the user
        email: String,

        /// Account ID
        #[arg(short, long)]
        account: Option<String>,

        /// Permission level (required)
        #[arg(short, long, value_enum)]
        level: PermissionLevelArg,

        /// Folder type: project-files, plans, or custom path
        #[arg(long, default_value = "project-files")]
        folder: String,

        /// Project filter expression
        #[arg(short, long)]
        filter: Option<String>,

        /// File containing project IDs (one per line)
        #[arg(long, value_name = "FILE")]
        project_ids: Option<PathBuf>,

        /// Parallel requests (default: 10, max: 50)
        #[arg(long, default_value = "10")]
        concurrency: usize,

        /// Preview changes without executing
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },
}

/// Permission level argument for CLI
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PermissionLevelArg {
    /// View only access
    ViewOnly,
    /// View and download access
    ViewDownload,
    /// Upload only access
    UploadOnly,
    /// View, download, and upload access
    ViewDownloadUpload,
    /// View, download, upload, and edit access
    ViewDownloadUploadEdit,
    /// Full folder control
    FolderControl,
}

impl From<PermissionLevelArg> for PermissionLevel {
    fn from(arg: PermissionLevelArg) -> Self {
        match arg {
            PermissionLevelArg::ViewOnly => PermissionLevel::ViewOnly,
            PermissionLevelArg::ViewDownload => PermissionLevel::ViewDownload,
            PermissionLevelArg::UploadOnly => PermissionLevel::UploadOnly,
            PermissionLevelArg::ViewDownloadUpload => PermissionLevel::ViewDownloadUpload,
            PermissionLevelArg::ViewDownloadUploadEdit => PermissionLevel::ViewDownloadUploadEdit,
            PermissionLevelArg::FolderControl => PermissionLevel::FolderControl,
        }
    }
}

/// Project listing subcommands (for admin context)
#[derive(Debug, Subcommand)]
pub enum AdminProjectCommands {
    /// List projects with filtering
    List {
        /// Account ID
        #[arg(short, long)]
        account: Option<String>,

        /// Filter expression
        #[arg(short, long)]
        filter: Option<String>,

        /// Project status: active, inactive, archived
        #[arg(long)]
        status: Option<String>,

        /// Platform: acc, bim360, all (default: all)
        #[arg(long, default_value = "all")]
        platform: String,

        /// Maximum projects to return
        #[arg(long)]
        limit: Option<usize>,
    },
}

/// Operation management subcommands
#[derive(Debug, Subcommand)]
pub enum OperationCommands {
    /// Check status of a bulk operation
    Status {
        /// Operation ID (defaults to most recent)
        operation_id: Option<Uuid>,
    },

    /// Resume an interrupted operation
    Resume {
        /// Operation ID to resume (defaults to most recent incomplete)
        operation_id: Option<Uuid>,

        /// Override concurrency setting
        #[arg(long)]
        concurrency: Option<usize>,
    },

    /// Cancel an in-progress operation
    Cancel {
        /// Operation ID to cancel
        operation_id: Option<Uuid>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// List all operations
    List {
        /// Filter by status: pending, in_progress, completed, failed, cancelled
        #[arg(long)]
        status: Option<String>,

        /// Maximum operations to show
        #[arg(long, default_value = "10")]
        limit: usize,
    },
}

impl AdminCommands {
    pub async fn execute(
        self,
        config: &Config,
        auth_client: &AuthClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            AdminCommands::User(cmd) => cmd.execute(config, auth_client, output_format).await,
            AdminCommands::Folder(cmd) => cmd.execute(config, auth_client, output_format).await,
            AdminCommands::Project(cmd) => cmd.execute(config, auth_client, output_format).await,
            AdminCommands::Operation(cmd) => cmd.execute(output_format).await,
        }
    }
}

impl UserCommands {
    pub async fn execute(
        self,
        config: &Config,
        auth_client: &AuthClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            UserCommands::Add {
                email,
                account,
                role,
                filter,
                project_ids,
                concurrency,
                dry_run,
                yes: _,
            } => {
                // Get account ID from parameter or environment
                let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());

                let account_id = match account_id {
                    Some(id) if !id.is_empty() => id,
                    _ => {
                        anyhow::bail!(
                            "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
                        );
                    }
                };

                // Parse filter expression
                let mut project_filter = if let Some(f) = &filter {
                    ProjectFilter::from_expression(f)?
                } else {
                    ProjectFilter::new()
                };

                // Load project IDs from file if specified
                if let Some(ids_file) = &project_ids {
                    let content = std::fs::read_to_string(ids_file)?;
                    let ids: Vec<String> = content
                        .lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty() && !l.starts_with('#'))
                        .collect();
                    project_filter.include_ids = Some(ids);
                }

                // Create bulk config
                let bulk_config = BulkConfig {
                    concurrency: concurrency.min(50),
                    dry_run,
                    ..Default::default()
                };

                if output_format.supports_colors() {
                    println!(
                        "\n{} Bulk add user: {} to account {}",
                        "→".cyan(),
                        email.green(),
                        account_id.cyan()
                    );
                    if let Some(r) = &role {
                        println!("  Role: {}", r.yellow());
                    }
                    if let Some(f) = &filter {
                        println!("  Filter: {}", f);
                    }
                    println!("  Concurrency: {}", concurrency.min(50));
                    if dry_run {
                        println!("  {} Dry-run mode enabled", "⚠".yellow());
                    }
                    println!();
                }

                // Create API clients
                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                );
                let users_client = Arc::new(ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                ));

                // Create progress bar
                let progress_bar = if output_format.supports_colors() {
                    let pb = ProgressBar::new(0);
                    pb.set_style(
                        ProgressStyle::default_bar()
                            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                            .unwrap()
                            .progress_chars("=>-"),
                    );
                    Some(pb)
                } else {
                    None
                };

                // Progress callback
                let pb_clone = progress_bar.clone();
                let on_progress = move |progress: ProgressUpdate| {
                    if let Some(ref pb) = pb_clone {
                        pb.set_length(progress.total as u64);
                        pb.set_position(
                            (progress.completed + progress.failed + progress.skipped) as u64,
                        );
                        pb.set_message(format!(
                            "✓{} ○{} ✗{}",
                            progress.completed, progress.skipped, progress.failed
                        ));
                    }
                };

                // Execute bulk operation
                let result = bulk_add_user(
                    &admin_client,
                    users_client,
                    &account_id,
                    &email,
                    role.as_deref(),
                    &project_filter,
                    bulk_config,
                    on_progress,
                )
                .await?;

                // Finish progress bar
                if let Some(pb) = progress_bar {
                    pb.finish_and_clear();
                }

                // Display results
                display_bulk_result(&result, output_format)?;

                // Exit with appropriate code
                if result.failed > 0 {
                    std::process::exit(1); // Partial success
                }

                Ok(())
            }

            UserCommands::Remove {
                email,
                account,
                filter,
                project_ids,
                concurrency,
                dry_run,
                yes: _,
            } => {
                // Get account ID from parameter or environment
                let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());

                let account_id = match account_id {
                    Some(id) if !id.is_empty() => id,
                    _ => {
                        anyhow::bail!(
                            "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
                        );
                    }
                };

                // Parse filter expression
                let mut project_filter = if let Some(f) = &filter {
                    ProjectFilter::from_expression(f)?
                } else {
                    ProjectFilter::new()
                };

                // Load project IDs from file if specified
                if let Some(ids_file) = &project_ids {
                    let content = std::fs::read_to_string(ids_file)?;
                    let ids: Vec<String> = content
                        .lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty() && !l.starts_with('#'))
                        .collect();
                    project_filter.include_ids = Some(ids);
                }

                // Create bulk config
                let bulk_config = BulkConfig {
                    concurrency: concurrency.min(50),
                    dry_run,
                    ..Default::default()
                };

                if output_format.supports_colors() {
                    println!(
                        "\n{} Bulk remove user: {} from account {}",
                        "→".cyan(),
                        email.red(),
                        account_id.cyan()
                    );
                    if let Some(f) = &filter {
                        println!("  Filter: {}", f);
                    }
                    println!("  Concurrency: {}", concurrency.min(50));
                    if dry_run {
                        println!("  {} Dry-run mode enabled", "⚠".yellow());
                    }
                    println!();
                }

                // Create API clients
                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                );
                let users_client = Arc::new(ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                ));

                // Create progress bar
                let progress_bar = if output_format.supports_colors() {
                    let pb = ProgressBar::new(0);
                    pb.set_style(
                        ProgressStyle::default_bar()
                            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                            .unwrap()
                            .progress_chars("=>-"),
                    );
                    Some(pb)
                } else {
                    None
                };

                // Progress callback
                let pb_clone = progress_bar.clone();
                let on_progress = move |progress: ProgressUpdate| {
                    if let Some(ref pb) = pb_clone {
                        pb.set_length(progress.total as u64);
                        pb.set_position(
                            (progress.completed + progress.failed + progress.skipped) as u64,
                        );
                        pb.set_message(format!(
                            "✓{} ○{} ✗{}",
                            progress.completed, progress.skipped, progress.failed
                        ));
                    }
                };

                // Execute bulk operation
                let result = raps_admin::bulk_remove_user(
                    &admin_client,
                    users_client,
                    &account_id,
                    &email,
                    &project_filter,
                    bulk_config,
                    on_progress,
                )
                .await?;

                // Finish progress bar
                if let Some(pb) = progress_bar {
                    pb.finish_and_clear();
                }

                // Display results
                display_bulk_result(&result, output_format)?;

                // Exit with appropriate code
                if result.failed > 0 {
                    std::process::exit(1); // Partial success
                }

                Ok(())
            }

            UserCommands::Update {
                email,
                account,
                role,
                from_role,
                filter,
                project_ids,
                concurrency,
                dry_run,
                yes: _,
            } => {
                // Get account ID from parameter or environment
                let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());

                let account_id = match account_id {
                    Some(id) if !id.is_empty() => id,
                    _ => {
                        anyhow::bail!(
                            "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
                        );
                    }
                };

                // Parse filter expression
                let mut project_filter = if let Some(f) = &filter {
                    ProjectFilter::from_expression(f)?
                } else {
                    ProjectFilter::new()
                };

                // Load project IDs from file if specified
                if let Some(ids_file) = &project_ids {
                    let content = std::fs::read_to_string(ids_file)?;
                    let ids: Vec<String> = content
                        .lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty() && !l.starts_with('#'))
                        .collect();
                    project_filter.include_ids = Some(ids);
                }

                // Create bulk config
                let bulk_config = BulkConfig {
                    concurrency: concurrency.min(50),
                    dry_run,
                    ..Default::default()
                };

                if output_format.supports_colors() {
                    println!(
                        "\n{} Bulk update user: {} to role: {}",
                        "→".cyan(),
                        email.green(),
                        role.yellow()
                    );
                    if let Some(fr) = &from_role {
                        println!("  From role: {}", fr);
                    }
                    if let Some(f) = &filter {
                        println!("  Filter: {}", f);
                    }
                    println!("  Concurrency: {}", concurrency.min(50));
                    if dry_run {
                        println!("  {} Dry-run mode enabled", "⚠".yellow());
                    }
                    println!();
                }

                // Create API clients
                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                );
                let users_client = Arc::new(ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                ));

                // Create progress bar
                let progress_bar = if output_format.supports_colors() {
                    let pb = ProgressBar::new(0);
                    pb.set_style(
                        ProgressStyle::default_bar()
                            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                            .unwrap()
                            .progress_chars("=>-"),
                    );
                    Some(pb)
                } else {
                    None
                };

                // Progress callback
                let pb_clone = progress_bar.clone();
                let on_progress = move |progress: ProgressUpdate| {
                    if let Some(ref pb) = pb_clone {
                        pb.set_length(progress.total as u64);
                        pb.set_position(
                            (progress.completed + progress.failed + progress.skipped) as u64,
                        );
                        pb.set_message(format!(
                            "✓{} ○{} ✗{}",
                            progress.completed, progress.skipped, progress.failed
                        ));
                    }
                };

                // Execute bulk operation
                let result = raps_admin::bulk_update_role(
                    &admin_client,
                    users_client,
                    &account_id,
                    &email,
                    &role,
                    from_role.as_deref(),
                    &project_filter,
                    bulk_config,
                    on_progress,
                )
                .await?;

                // Finish progress bar
                if let Some(pb) = progress_bar {
                    pb.finish_and_clear();
                }

                // Display results
                display_bulk_result(&result, output_format)?;

                // Exit with appropriate code
                if result.failed > 0 {
                    std::process::exit(1); // Partial success
                }

                Ok(())
            }
        }
    }
}

impl FolderCommands {
    pub async fn execute(
        self,
        config: &Config,
        auth_client: &AuthClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            FolderCommands::Rights {
                email,
                account,
                level,
                folder,
                filter,
                project_ids,
                concurrency,
                dry_run,
                yes: _,
            } => {
                // Get account ID from parameter or environment
                let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());

                let account_id = match account_id {
                    Some(id) if !id.is_empty() => id,
                    _ => {
                        anyhow::bail!(
                            "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
                        );
                    }
                };

                // Parse filter expression
                let mut project_filter = if let Some(f) = &filter {
                    ProjectFilter::from_expression(f)?
                } else {
                    ProjectFilter::new()
                };

                // Load project IDs from file if specified
                if let Some(ids_file) = &project_ids {
                    let content = std::fs::read_to_string(ids_file)?;
                    let ids: Vec<String> = content
                        .lines()
                        .map(|l| l.trim().to_string())
                        .filter(|l| !l.is_empty() && !l.starts_with('#'))
                        .collect();
                    project_filter.include_ids = Some(ids);
                }

                // Parse folder type
                let folder_type = match folder.to_lowercase().as_str() {
                    "project-files" | "projectfiles" => raps_admin::FolderType::ProjectFiles,
                    "plans" => raps_admin::FolderType::Plans,
                    _ => raps_admin::FolderType::Custom(folder.clone()),
                };

                // Create bulk config
                let bulk_config = BulkConfig {
                    concurrency: concurrency.min(50),
                    dry_run,
                    ..Default::default()
                };

                if output_format.supports_colors() {
                    println!(
                        "\n{} Bulk update folder rights for: {} in account {}",
                        "→".cyan(),
                        email.green(),
                        account_id.cyan()
                    );
                    println!("  Folder: {}", folder);
                    println!("  Permission level: {:?}", level);
                    if let Some(f) = &filter {
                        println!("  Filter: {}", f);
                    }
                    println!("  Concurrency: {}", concurrency.min(50));
                    if dry_run {
                        println!("  {} Dry-run mode enabled", "⚠".yellow());
                    }
                    println!();
                }

                // Create API clients
                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                );
                let permissions_client = Arc::new(
                    raps_acc::permissions::FolderPermissionsClient::new_with_http_config(
                        config.clone(),
                        auth_client.clone(),
                        http_config,
                    ),
                );

                // Create progress bar
                let progress_bar = if output_format.supports_colors() {
                    let pb = ProgressBar::new(0);
                    pb.set_style(
                        ProgressStyle::default_bar()
                            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
                            .unwrap()
                            .progress_chars("=>-"),
                    );
                    Some(pb)
                } else {
                    None
                };

                // Progress callback
                let pb_clone = progress_bar.clone();
                let on_progress = move |progress: ProgressUpdate| {
                    if let Some(ref pb) = pb_clone {
                        pb.set_length(progress.total as u64);
                        pb.set_position(
                            (progress.completed + progress.failed + progress.skipped) as u64,
                        );
                        pb.set_message(format!(
                            "✓{} ○{} ✗{}",
                            progress.completed, progress.skipped, progress.failed
                        ));
                    }
                };

                // Execute bulk operation
                let result = raps_admin::bulk_update_folder_rights(
                    &admin_client,
                    permissions_client,
                    &account_id,
                    &email,
                    level.into(),
                    folder_type,
                    &project_filter,
                    bulk_config,
                    on_progress,
                )
                .await?;

                // Finish progress bar
                if let Some(pb) = progress_bar {
                    pb.finish_and_clear();
                }

                // Display results
                display_bulk_result(&result, output_format)?;

                // Exit with appropriate code
                if result.failed > 0 {
                    std::process::exit(1); // Partial success
                }

                Ok(())
            }
        }
    }
}

impl AdminProjectCommands {
    pub async fn execute(
        self,
        config: &Config,
        auth_client: &AuthClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            AdminProjectCommands::List {
                account,
                filter,
                status,
                platform,
                limit,
            } => {
                // Get account ID from parameter or environment
                let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());

                let account_id = match account_id {
                    Some(id) if !id.is_empty() => id,
                    _ => {
                        anyhow::bail!(
                            "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
                        );
                    }
                };

                // Build filter expression from individual flags
                let mut filter_parts = Vec::new();
                if let Some(f) = &filter {
                    filter_parts.push(f.clone());
                }
                if let Some(s) = &status {
                    filter_parts.push(format!("status:{}", s));
                }
                if platform != "all" {
                    filter_parts.push(format!("platform:{}", platform));
                }

                let filter_expr = if filter_parts.is_empty() {
                    None
                } else {
                    Some(filter_parts.join(","))
                };

                let project_filter = if let Some(ref expr) = filter_expr {
                    ProjectFilter::from_expression(expr)?
                } else {
                    ProjectFilter::new()
                };

                if output_format.supports_colors() {
                    println!(
                        "\n{} List projects in account {}",
                        "→".cyan(),
                        account_id.cyan()
                    );
                    if let Some(ref expr) = filter_expr {
                        println!("  Filter: {}", expr);
                    }
                    if let Some(l) = limit {
                        println!("  Limit: {}", l);
                    }
                    println!();
                }

                // Create admin client
                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                );

                // List all projects
                let all_projects = admin_client.list_all_projects(&account_id).await?;

                // Apply filter
                let mut filtered_projects = project_filter.apply(all_projects);

                // Apply limit
                if let Some(l) = limit {
                    filtered_projects.truncate(l);
                }

                // Build output
                let outputs: Vec<ProjectListOutput> = filtered_projects
                    .iter()
                    .map(|p| ProjectListOutput {
                        id: p.id.clone(),
                        name: p.name.clone(),
                        status: p.status.clone().unwrap_or_else(|| "unknown".to_string()),
                        platform: if p.is_acc() {
                            "acc".to_string()
                        } else if p.is_bim360() {
                            "bim360".to_string()
                        } else {
                            "unknown".to_string()
                        },
                        created_at: p.created_at.map(|d| d.to_rfc3339()),
                    })
                    .collect();

                match output_format {
                    OutputFormat::Table => {
                        if outputs.is_empty() {
                            println!("{}", "No projects found matching the filter.".yellow());
                        } else {
                            println!("{}", "Projects:".bold());
                            println!("{}", "─".repeat(100));
                            println!(
                                "{:<38} {:<30} {:<10} {:<10} {}",
                                "ID".bold(),
                                "Name".bold(),
                                "Status".bold(),
                                "Platform".bold(),
                                "Created".bold()
                            );
                            println!("{}", "─".repeat(100));

                            for p in &outputs {
                                let created = p.created_at.as_deref().unwrap_or("-");
                                let name_truncated = if p.name.len() > 28 {
                                    format!("{}...", &p.name[..25])
                                } else {
                                    p.name.clone()
                                };
                                println!(
                                    "{:<38} {:<30} {:<10} {:<10} {}",
                                    p.id.cyan(),
                                    name_truncated,
                                    format_project_status(&p.status),
                                    p.platform,
                                    created.dimmed()
                                );
                            }

                            println!("{}", "─".repeat(100));
                            println!("{} {} project(s) found", "→".cyan(), outputs.len());
                        }
                    }
                    _ => {
                        output_format.write(&outputs)?;
                    }
                }

                Ok(())
            }
        }
    }
}

#[derive(Serialize)]
struct ProjectListOutput {
    id: String,
    name: String,
    status: String,
    platform: String,
    created_at: Option<String>,
}

fn format_project_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "active" => status.green().to_string(),
        "inactive" => status.yellow().to_string(),
        "archived" => status.dimmed().to_string(),
        _ => status.to_string(),
    }
}

impl OperationCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            OperationCommands::Status { operation_id } => {
                let state_manager = StateManager::new()?;

                let op_id = match operation_id {
                    Some(id) => id,
                    None => {
                        // Get most recent operation
                        let ops = state_manager.list_operations(None).await?;
                        if ops.is_empty() {
                            anyhow::bail!("No operations found");
                        }
                        ops[0].operation_id
                    }
                };

                let state = state_manager.load_operation(op_id).await?;

                let output = OperationStatusOutput {
                    operation_id: state.operation_id.to_string(),
                    operation_type: format!("{:?}", state.operation_type),
                    status: format!("{:?}", state.status),
                    total: state.project_ids.len(),
                    completed: state
                        .results
                        .values()
                        .filter(|r| matches!(r.result, raps_admin::ItemResult::Success))
                        .count(),
                    skipped: state
                        .results
                        .values()
                        .filter(|r| matches!(r.result, raps_admin::ItemResult::Skipped { .. }))
                        .count(),
                    failed: state
                        .results
                        .values()
                        .filter(|r| matches!(r.result, raps_admin::ItemResult::Failed { .. }))
                        .count(),
                    created_at: state.created_at.to_rfc3339(),
                    updated_at: state.updated_at.to_rfc3339(),
                };

                match output_format {
                    OutputFormat::Table => {
                        println!("\n{}", "Operation Status:".bold());
                        println!("{}", "─".repeat(60));
                        println!("{:<15} {}", "Operation:".bold(), output.operation_id.cyan());
                        println!("{:<15} {}", "Type:".bold(), output.operation_type);
                        println!("{:<15} {}", "Status:".bold(), format_status(&output.status));
                        println!(
                            "{:<15} {}/{} ({}%)",
                            "Progress:".bold(),
                            output.completed + output.skipped + output.failed,
                            output.total,
                            if output.total > 0 {
                                ((output.completed + output.skipped + output.failed) * 100)
                                    / output.total
                            } else {
                                100
                            }
                        );
                        println!(
                            "{:<15} {}",
                            "Completed:".bold(),
                            output.completed.to_string().green()
                        );
                        println!(
                            "{:<15} {}",
                            "Skipped:".bold(),
                            output.skipped.to_string().yellow()
                        );
                        println!(
                            "{:<15} {}",
                            "Failed:".bold(),
                            output.failed.to_string().red()
                        );
                        println!("{:<15} {}", "Created:".bold(), output.created_at);
                        println!("{:<15} {}", "Updated:".bold(), output.updated_at);
                        println!("{}", "─".repeat(60));
                    }
                    _ => {
                        output_format.write(&output)?;
                    }
                }

                Ok(())
            }

            OperationCommands::Resume {
                operation_id,
                concurrency,
            } => {
                let state_manager = StateManager::new()?;

                // Find operation to resume
                let op_id = match operation_id {
                    Some(id) => id,
                    None => {
                        // Get most recent resumable operation
                        match state_manager.get_resumable_operation().await? {
                            Some(id) => id,
                            None => anyhow::bail!("No resumable operation found"),
                        }
                    }
                };

                let state = state_manager.load_operation(op_id).await?;

                // Verify operation can be resumed
                if state.status != OperationStatus::InProgress
                    && state.status != OperationStatus::Pending
                {
                    anyhow::bail!(
                        "Operation cannot be resumed (current status: {:?})",
                        state.status
                    );
                }

                let pending = state_manager.get_pending_projects(&state);
                if pending.is_empty() {
                    if output_format.supports_colors() {
                        println!("{} Operation {} is already complete", "✓".green(), op_id);
                    }
                    return Ok(());
                }

                let concurrency_limit = concurrency.unwrap_or(10).min(50);

                if output_format.supports_colors() {
                    println!(
                        "\n{} Resuming operation: {}",
                        "→".cyan(),
                        op_id.to_string().cyan()
                    );
                    println!("  Type: {:?}", state.operation_type);
                    println!(
                        "  Pending: {}/{} items",
                        pending.len(),
                        state.project_ids.len()
                    );
                    println!("  Concurrency: {}", concurrency_limit);
                    println!();

                    // Note: For full resume support, we'd need the original API clients
                    // For now, just report pending items
                    println!(
                        "{} Resume requires re-running with the original command and credentials.",
                        "⚠".yellow()
                    );
                    println!("  Pending projects:");
                    for (i, project_id) in pending.iter().take(10).enumerate() {
                        println!("    {}. {}", i + 1, project_id.dimmed());
                    }
                    if pending.len() > 10 {
                        println!("    ... and {} more", pending.len() - 10);
                    }
                }

                Ok(())
            }

            OperationCommands::Cancel {
                operation_id,
                yes: _,
            } => {
                let state_manager = StateManager::new()?;

                // Find operation to cancel
                let op_id = match operation_id {
                    Some(id) => id,
                    None => {
                        // Get most recent in-progress operation
                        match state_manager.get_resumable_operation().await? {
                            Some(id) => id,
                            None => anyhow::bail!("No active operation found to cancel"),
                        }
                    }
                };

                let state = state_manager.load_operation(op_id).await?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} Cancelling operation: {}",
                        "→".cyan(),
                        op_id.to_string().cyan()
                    );
                    println!("  Type: {:?}", state.operation_type);
                    println!("  Current status: {:?}", state.status);
                }

                // Cancel the operation
                state_manager.cancel_operation(op_id).await?;

                if output_format.supports_colors() {
                    let processed = state.results.len();
                    let total = state.project_ids.len();
                    println!("\n{} Operation cancelled", "✓".green());
                    println!(
                        "  Processed: {}/{} items before cancellation",
                        processed, total
                    );
                }

                Ok(())
            }

            OperationCommands::List { status, limit } => {
                let state_manager = StateManager::new()?;

                let status_filter = status
                    .as_ref()
                    .and_then(|s| match s.to_lowercase().as_str() {
                        "pending" => Some(OperationStatus::Pending),
                        "in_progress" | "in-progress" => Some(OperationStatus::InProgress),
                        "completed" => Some(OperationStatus::Completed),
                        "failed" => Some(OperationStatus::Failed),
                        "cancelled" => Some(OperationStatus::Cancelled),
                        _ => None,
                    });

                let operations = state_manager.list_operations(status_filter).await?;
                let operations: Vec<_> = operations.into_iter().take(limit).collect();

                if operations.is_empty() {
                    match output_format {
                        OutputFormat::Table => println!("{}", "No operations found.".yellow()),
                        _ => output_format.write(&Vec::<OperationListOutput>::new())?,
                    }
                    return Ok(());
                }

                let outputs: Vec<OperationListOutput> = operations
                    .iter()
                    .map(|op| OperationListOutput {
                        operation_id: op.operation_id.to_string(),
                        operation_type: format!("{:?}", op.operation_type),
                        status: format!("{:?}", op.status),
                        progress: format!("{}/{}", op.completed + op.skipped + op.failed, op.total),
                        updated_at: op.updated_at.to_rfc3339(),
                    })
                    .collect();

                match output_format {
                    OutputFormat::Table => {
                        println!("\n{}", "Operations:".bold());
                        println!("{}", "─".repeat(100));
                        println!(
                            "{:<38} {:<15} {:<12} {:<12} {}",
                            "ID".bold(),
                            "Type".bold(),
                            "Status".bold(),
                            "Progress".bold(),
                            "Updated".bold()
                        );
                        println!("{}", "─".repeat(100));

                        for op in &outputs {
                            println!(
                                "{:<38} {:<15} {:<12} {:<12} {}",
                                op.operation_id.cyan(),
                                op.operation_type,
                                format_status(&op.status),
                                op.progress,
                                op.updated_at.dimmed()
                            );
                        }

                        println!("{}", "─".repeat(100));
                        println!("{} {} operation(s) found", "→".cyan(), outputs.len());
                    }
                    _ => {
                        output_format.write(&outputs)?;
                    }
                }

                Ok(())
            }
        }
    }
}

#[derive(Serialize)]
struct OperationStatusOutput {
    operation_id: String,
    operation_type: String,
    status: String,
    total: usize,
    completed: usize,
    skipped: usize,
    failed: usize,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
struct OperationListOutput {
    operation_id: String,
    operation_type: String,
    status: String,
    progress: String,
    updated_at: String,
}

fn format_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "completed" => status.green().to_string(),
        "failed" => status.red().to_string(),
        "inprogress" | "in_progress" => status.yellow().to_string(),
        "cancelled" => status.dimmed().to_string(),
        _ => status.to_string(),
    }
}

/// Output format for bulk operation results
#[derive(Serialize)]
struct BulkResultOutput {
    operation_id: String,
    total: usize,
    completed: usize,
    skipped: usize,
    failed: usize,
    duration_secs: f64,
    details: Vec<BulkResultDetailOutput>,
}

#[derive(Serialize)]
struct BulkResultDetailOutput {
    project_id: String,
    project_name: Option<String>,
    status: String,
    message: Option<String>,
    attempts: u32,
}

/// Display bulk operation results
fn display_bulk_result(result: &BulkOperationResult, output_format: OutputFormat) -> Result<()> {
    let details: Vec<BulkResultDetailOutput> = result
        .details
        .iter()
        .map(|d| {
            let (status, message) = match &d.result {
                ItemResult::Success => ("success".to_string(), None),
                ItemResult::Skipped { reason } => ("skipped".to_string(), Some(reason.clone())),
                ItemResult::Failed { error, .. } => ("failed".to_string(), Some(error.clone())),
            };
            BulkResultDetailOutput {
                project_id: d.project_id.clone(),
                project_name: d.project_name.clone(),
                status,
                message,
                attempts: d.attempts,
            }
        })
        .collect();

    let output = BulkResultOutput {
        operation_id: result.operation_id.to_string(),
        total: result.total,
        completed: result.completed,
        skipped: result.skipped,
        failed: result.failed,
        duration_secs: result.duration.as_secs_f64(),
        details,
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Bulk Operation Results:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "Operation:".bold(), output.operation_id.cyan());
            println!("{:<15} {}", "Total:".bold(), output.total);
            println!(
                "{:<15} {}",
                "Completed:".bold(),
                output.completed.to_string().green()
            );
            println!(
                "{:<15} {}",
                "Skipped:".bold(),
                output.skipped.to_string().yellow()
            );
            println!(
                "{:<15} {}",
                "Failed:".bold(),
                output.failed.to_string().red()
            );
            println!("{:<15} {:.2}s", "Duration:".bold(), output.duration_secs);
            println!("{}", "─".repeat(60));

            // Show failed items if any
            if result.failed > 0 {
                println!("\n{}", "Failed Projects:".red().bold());
                for detail in &output.details {
                    if detail.status == "failed" {
                        let name = detail.project_name.as_deref().unwrap_or(&detail.project_id);
                        let msg = detail.message.as_deref().unwrap_or("Unknown error");
                        println!("  {} {} - {}", "✗".red(), name, msg.dimmed());
                    }
                }
            }

            // Summary
            println!();
            if result.failed == 0 && result.total > 0 {
                println!("{} Operation completed successfully!", "✓".green().bold());
            } else if result.failed > 0 {
                println!(
                    "{} Operation completed with {} failure(s)",
                    "⚠".yellow().bold(),
                    result.failed
                );
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

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

use anyhow::{Context, Result};
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
    /// List users in an account or project
    List {
        /// Account ID (defaults to APS_ACCOUNT_ID env var)
        #[arg(short, long)]
        account: Option<String>,

        /// Optional: list users for a specific project only
        #[arg(short, long)]
        project: Option<String>,

        /// Filter by role name
        #[arg(long)]
        role: Option<String>,

        /// Filter by status (active, inactive, not_invited)
        #[arg(long)]
        status: Option<String>,

        /// Search by email or name
        #[arg(long)]
        search: Option<String>,
    },

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

    /// Update user roles and/or company across multiple projects
    Update {
        /// Email address of the user to update
        email: String,

        /// Account ID
        #[arg(short, long)]
        account: Option<String>,

        /// New role to assign (required unless --company is provided)
        #[arg(short, long)]
        role: Option<String>,

        /// Company name to assign at account level
        #[arg(long)]
        company: Option<String>,

        /// Only update users with this current role
        #[arg(long)]
        from_role: Option<String>,

        /// Project filter expression
        #[arg(short, long)]
        filter: Option<String>,

        /// File containing project IDs (one per line)
        #[arg(long, value_name = "FILE")]
        project_ids: Option<PathBuf>,

        /// Import updates from a CSV file (columns: email, role, company)
        #[arg(long, value_name = "FILE")]
        from_csv: Option<PathBuf>,

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

    /// Add a user to a single project by email
    #[command(name = "add-to-project")]
    AddToProject {
        /// Project ID
        #[arg(short, long)]
        project: String,

        /// Email address of the user (used as user identifier)
        #[arg(short, long)]
        email: String,

        /// Role ID to assign
        #[arg(short, long)]
        role_id: Option<String>,
    },

    /// Remove a user from a single project
    #[command(name = "remove-from-project")]
    RemoveFromProject {
        /// Project ID
        #[arg(short, long)]
        project: String,

        /// User ID to remove
        #[arg(short, long)]
        user_id: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Update a user's role in a single project
    #[command(name = "update-in-project")]
    UpdateInProject {
        /// Project ID
        #[arg(short, long)]
        project: String,

        /// User ID to update
        #[arg(short, long)]
        user_id: String,

        /// New role ID to assign
        #[arg(short, long)]
        role_id: Option<String>,
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
            UserCommands::List {
                account,
                project,
                role,
                status,
                search,
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

                let http_config = HttpClientConfig::default();

                if let Some(project_id) = project {
                    // Project-level user listing
                    if output_format.supports_colors() {
                        println!(
                            "\n{} List users in project {}",
                            "→".cyan(),
                            project_id.cyan()
                        );
                        println!();
                    }

                    let users_client = ProjectUsersClient::new_with_http_config(
                        config.clone(),
                        auth_client.clone(),
                        http_config,
                    );

                    let all_users = users_client.list_all_project_users(&project_id).await?;

                    // Apply filters
                    let filtered: Vec<_> = all_users
                        .into_iter()
                        .filter(|u| {
                            if let Some(ref r) = role {
                                if let Some(ref role_name) = u.role_name {
                                    if !role_name.to_lowercase().contains(&r.to_lowercase()) {
                                        return false;
                                    }
                                } else {
                                    return false;
                                }
                            }
                            if let Some(ref s) = search {
                                let s_lower = s.to_lowercase();
                                let email_match = u
                                    .email
                                    .as_ref()
                                    .map(|e| e.to_lowercase().contains(&s_lower))
                                    .unwrap_or(false);
                                let name_match = u
                                    .name
                                    .as_ref()
                                    .map(|n| n.to_lowercase().contains(&s_lower))
                                    .unwrap_or(false);
                                if !email_match && !name_match {
                                    return false;
                                }
                            }
                            true
                        })
                        .collect();

                    let outputs: Vec<UserListOutput> = filtered
                        .iter()
                        .map(|u| UserListOutput {
                            id: u.id.clone(),
                            email: u.email.clone().unwrap_or_default(),
                            name: u.name.clone().unwrap_or_default(),
                            role: u.role_name.clone().unwrap_or_default(),
                            company: None,
                            status: None,
                        })
                        .collect();

                    display_user_list(&outputs, output_format)?;
                } else {
                    // Account-level user listing
                    if output_format.supports_colors() {
                        println!(
                            "\n{} List users in account {}",
                            "→".cyan(),
                            account_id.cyan()
                        );
                        println!();
                    }

                    let admin_client = AccountAdminClient::new_with_http_config(
                        config.clone(),
                        auth_client.clone(),
                        http_config,
                    );

                    let all_users = admin_client.list_all_users(&account_id).await?;

                    // Apply filters
                    let filtered: Vec<_> = all_users
                        .into_iter()
                        .filter(|u| {
                            if let Some(ref s) = status {
                                if let Some(ref user_status) = u.status {
                                    if !user_status.to_lowercase().eq(&s.to_lowercase()) {
                                        return false;
                                    }
                                } else {
                                    return false;
                                }
                            }
                            if let Some(ref s) = search {
                                let s_lower = s.to_lowercase();
                                let email_match = u.email.to_lowercase().contains(&s_lower);
                                let name_match = u
                                    .name
                                    .as_ref()
                                    .map(|n| n.to_lowercase().contains(&s_lower))
                                    .unwrap_or(false);
                                let first_match = u
                                    .first_name
                                    .as_ref()
                                    .map(|n| n.to_lowercase().contains(&s_lower))
                                    .unwrap_or(false);
                                let last_match = u
                                    .last_name
                                    .as_ref()
                                    .map(|n| n.to_lowercase().contains(&s_lower))
                                    .unwrap_or(false);
                                if !email_match && !name_match && !first_match && !last_match {
                                    return false;
                                }
                            }
                            true
                        })
                        .collect();

                    let outputs: Vec<UserListOutput> = filtered
                        .iter()
                        .map(|u| {
                            let display_name = match (&u.first_name, &u.last_name) {
                                (Some(f), Some(l)) => format!("{} {}", f, l),
                                (Some(f), None) => f.clone(),
                                (None, Some(l)) => l.clone(),
                                (None, None) => {
                                    u.name.clone().unwrap_or_default()
                                }
                            };
                            UserListOutput {
                                id: u.id.clone(),
                                email: u.email.clone(),
                                name: display_name,
                                role: String::new(),
                                company: u.company_id.clone(),
                                status: u.status.clone(),
                            }
                        })
                        .collect();

                    display_user_list(&outputs, output_format)?;
                }

                Ok(())
            }

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
                company,
                from_role,
                filter,
                project_ids,
                from_csv,
                concurrency,
                dry_run,
                yes: _,
            } => {
                // Handle --from-csv mode
                if let Some(csv_path) = from_csv {
                    return execute_csv_update(
                        config,
                        auth_client,
                        account.clone(),
                        filter.clone(),
                        project_ids.clone(),
                        &csv_path,
                        concurrency,
                        dry_run,
                        output_format,
                    )
                    .await;
                }

                // Validate: at least --role or --company must be provided
                if role.is_none() && company.is_none() {
                    anyhow::bail!(
                        "At least one of --role or --company must be provided."
                    );
                }

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

                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                );

                // Handle company update at account level
                if let Some(ref company_name) = company {
                    if output_format.supports_colors() {
                        println!(
                            "\n{} Update company for user: {} to: {}",
                            "→".cyan(),
                            email.green(),
                            company_name.yellow()
                        );
                        if dry_run {
                            println!("  {} Dry-run mode enabled", "⚠".yellow());
                        }
                    }

                    if !dry_run {
                        // Look up user by email to get user ID
                        let user = admin_client
                            .find_user_by_email(&account_id, &email)
                            .await?
                            .ok_or_else(|| anyhow::anyhow!("User not found: {}", email))?;

                        let update_req = raps_acc::admin::UpdateAccountUserRequest {
                            company_id: None,
                            company_name: Some(company_name.clone()),
                        };

                        admin_client
                            .update_user(&account_id, &user.id, update_req)
                            .await?;

                        if output_format.supports_colors() {
                            println!(
                                "{} Company updated for {} to '{}'",
                                "✓".green().bold(),
                                email,
                                company_name
                            );
                        }
                    } else if output_format.supports_colors() {
                        println!(
                            "  {} Would update company for {} to '{}'",
                            "→".dimmed(),
                            email,
                            company_name
                        );
                    }
                }

                // Handle role update across projects (if --role is provided)
                if let Some(ref role_value) = role {
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
                            role_value.yellow()
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
                        role_value,
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
                        std::process::exit(1);
                    }
                }

                Ok(())
            }

            UserCommands::AddToProject {
                project,
                email,
                role_id,
            } => {
                let http_config = HttpClientConfig::default();
                let users_client = ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                );

                if output_format.supports_colors() {
                    println!(
                        "\n{} Adding user {} to project {}",
                        "→".cyan(),
                        email.cyan(),
                        project.cyan()
                    );
                }

                let request = raps_acc::users::AddProjectUserRequest {
                    user_id: email.clone(),
                    role_id: role_id.clone(),
                    products: vec![],
                };

                let user = users_client.add_user(&project, request).await?;

                #[derive(Serialize)]
                struct AddResult {
                    user_id: String,
                    email: String,
                    role: Option<String>,
                    project: String,
                }

                let result = AddResult {
                    user_id: user.id,
                    email: user.email.unwrap_or(email),
                    role: user.role_name,
                    project,
                };

                output_format.write(&result)?;

                if output_format.supports_colors() {
                    println!("\n{} User added successfully", "✓".green());
                }

                Ok(())
            }

            UserCommands::RemoveFromProject {
                project,
                user_id,
                yes,
            } => {
                let http_config = HttpClientConfig::default();
                let users_client = ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                );

                if !yes && output_format.supports_colors() {
                    println!(
                        "\n{} Remove user {} from project {}?",
                        "⚠".yellow(),
                        user_id.cyan(),
                        project.cyan()
                    );
                    print!("Continue? [y/N] ");
                    use std::io::{self, Write};
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    if !input.trim().eq_ignore_ascii_case("y") {
                        println!("Cancelled.");
                        return Ok(());
                    }
                }

                users_client.remove_user(&project, &user_id).await?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} User {} removed from project {}",
                        "✓".green(),
                        user_id.cyan(),
                        project.cyan()
                    );
                } else {
                    println!("User {} removed from project {}", user_id, project);
                }

                Ok(())
            }

            UserCommands::UpdateInProject {
                project,
                user_id,
                role_id,
            } => {
                let http_config = HttpClientConfig::default();
                let users_client = ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                );

                if output_format.supports_colors() {
                    println!(
                        "\n{} Updating user {} in project {}",
                        "→".cyan(),
                        user_id.cyan(),
                        project.cyan()
                    );
                }

                let request = raps_acc::users::UpdateProjectUserRequest {
                    role_id: role_id.clone(),
                    products: None,
                };

                let user = users_client.update_user(&project, &user_id, request).await?;

                #[derive(Serialize)]
                struct UpdateResult {
                    user_id: String,
                    email: Option<String>,
                    role: Option<String>,
                    project: String,
                }

                let result = UpdateResult {
                    user_id: user.id,
                    email: user.email,
                    role: user.role_name,
                    project,
                };

                output_format.write(&result)?;

                if output_format.supports_colors() {
                    println!("\n{} User updated successfully", "✓".green());
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

#[derive(Serialize)]
struct UserListOutput {
    id: String,
    email: String,
    name: String,
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    company: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

fn display_user_list(users: &Vec<UserListOutput>, output_format: OutputFormat) -> Result<()> {
    if users.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No users found.".yellow()),
            _ => output_format.write(&Vec::<UserListOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("{}", "Users:".bold());
            println!("{}", "─".repeat(110));
            println!(
                "{:<30} {:<25} {:<18} {:<18} {}",
                "Email".bold(),
                "Name".bold(),
                "Role".bold(),
                "Status".bold(),
                "Company".bold()
            );
            println!("{}", "─".repeat(110));

            for u in users {
                let email_truncated = if u.email.len() > 28 {
                    format!("{}...", &u.email[..25])
                } else {
                    u.email.clone()
                };
                let name_truncated = if u.name.len() > 23 {
                    format!("{}...", &u.name[..20])
                } else {
                    u.name.clone()
                };
                let role_display = if u.role.is_empty() {
                    "-".to_string()
                } else if u.role.len() > 16 {
                    format!("{}...", &u.role[..13])
                } else {
                    u.role.clone()
                };
                let status_display = u.status.as_deref().unwrap_or("-");
                let company_display = u.company.as_deref().unwrap_or("-");

                println!(
                    "{:<30} {:<25} {:<18} {:<18} {}",
                    email_truncated.cyan(),
                    name_truncated,
                    role_display,
                    format_user_status(status_display),
                    company_display.dimmed()
                );
            }

            println!("{}", "─".repeat(110));
            println!("{} {} user(s) found", "→".cyan(), users.len());
        }
        _ => {
            output_format.write(users)?;
        }
    }

    Ok(())
}

fn format_user_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "active" => status.green().to_string(),
        "inactive" | "not_invited" => status.yellow().to_string(),
        "disabled" => status.red().to_string(),
        _ => status.to_string(),
    }
}

// ============================================================================
// CSV UPDATE
// ============================================================================

/// A single row from the CSV update file
#[derive(Debug, serde::Deserialize)]
struct CsvUpdateRow {
    email: String,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    company: Option<String>,
}

#[derive(Serialize)]
struct CsvUpdateResultOutput {
    total: usize,
    updated: usize,
    skipped: usize,
    failed: usize,
    errors: Vec<CsvUpdateErrorOutput>,
}

#[derive(Serialize)]
struct CsvUpdateErrorOutput {
    email: String,
    error: String,
}

/// Execute bulk user updates from a CSV file
///
/// Expected columns: email (required), role (optional), company (optional)
#[allow(clippy::too_many_arguments)]
async fn execute_csv_update(
    config: &Config,
    auth_client: &AuthClient,
    account: Option<String>,
    filter: Option<String>,
    project_ids: Option<PathBuf>,
    csv_path: &PathBuf,
    concurrency: usize,
    dry_run: bool,
    output_format: OutputFormat,
) -> Result<()> {
    // Get account ID
    let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());
    let account_id = match account_id {
        Some(id) if !id.is_empty() => id,
        _ => {
            anyhow::bail!(
                "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
            );
        }
    };

    // Parse CSV file
    let mut reader = csv::Reader::from_path(csv_path)
        .with_context(|| format!("Failed to open CSV file: {}", csv_path.display()))?;

    let mut rows: Vec<CsvUpdateRow> = Vec::new();
    let mut validation_errors: Vec<String> = Vec::new();

    for (i, result) in reader.deserialize().enumerate() {
        match result {
            Ok(row) => {
                let row: CsvUpdateRow = row;
                // Validate email
                if row.email.is_empty() || !row.email.contains('@') {
                    validation_errors.push(format!(
                        "Row {}: invalid email '{}'",
                        i + 2,
                        row.email
                    ));
                    continue;
                }
                // Validate at least one field to update
                if row.role.is_none() && row.company.is_none() {
                    validation_errors.push(format!(
                        "Row {}: email '{}' has no role or company to update",
                        i + 2,
                        row.email
                    ));
                    continue;
                }
                rows.push(row);
            }
            Err(e) => {
                validation_errors.push(format!("Row {}: parse error: {}", i + 2, e));
            }
        }
    }

    // Report validation errors
    if !validation_errors.is_empty() {
        if output_format.supports_colors() {
            println!("{} CSV validation errors:", "✗".red().bold());
            for err in &validation_errors {
                println!("  {} {}", "•".red(), err);
            }
        }
        anyhow::bail!(
            "CSV validation failed with {} error(s). Fix errors before proceeding.",
            validation_errors.len()
        );
    }

    if rows.is_empty() {
        anyhow::bail!("No valid rows found in CSV file");
    }

    if output_format.supports_colors() {
        println!(
            "\n{} CSV update: {} rows from {}",
            "→".cyan(),
            rows.len().to_string().green(),
            csv_path.display().to_string().cyan()
        );
        if dry_run {
            println!("  {} Dry-run mode enabled", "⚠".yellow());
        }
        println!();
    }

    let http_config = HttpClientConfig::default();
    let admin_client = AccountAdminClient::new_with_http_config(
        config.clone(),
        auth_client.clone(),
        http_config.clone(),
    );

    // Parse filter for role updates
    let mut project_filter = if let Some(f) = &filter {
        ProjectFilter::from_expression(f)?
    } else {
        ProjectFilter::new()
    };
    if let Some(ids_file) = &project_ids {
        let content = std::fs::read_to_string(ids_file)?;
        let ids: Vec<String> = content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        project_filter.include_ids = Some(ids);
    }

    let mut updated = 0usize;
    let skipped = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();

    // Progress bar
    let progress_bar = if output_format.supports_colors() {
        let pb = ProgressBar::new(rows.len() as u64);
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

    for row in &rows {
        if let Some(ref pb) = progress_bar {
            pb.set_message(format!("{}", row.email));
        }

        if dry_run {
            if output_format.supports_colors() {
                let mut changes = Vec::new();
                if let Some(ref r) = row.role {
                    changes.push(format!("role={}", r));
                }
                if let Some(ref c) = row.company {
                    changes.push(format!("company={}", c));
                }
                if let Some(ref pb) = progress_bar {
                    pb.println(format!(
                        "  {} {} → {}",
                        "→".dimmed(),
                        row.email,
                        changes.join(", ")
                    ));
                }
            }
            updated += 1;
        } else {
            let mut row_updated = false;

            // Update company at account level if specified
            if let Some(ref company_name) = row.company {
                match admin_client
                    .find_user_by_email(&account_id, &row.email)
                    .await
                {
                    Ok(Some(user)) => {
                        let update_req = raps_acc::admin::UpdateAccountUserRequest {
                            company_id: None,
                            company_name: Some(company_name.clone()),
                        };
                        match admin_client
                            .update_user(&account_id, &user.id, update_req)
                            .await
                        {
                            Ok(_) => {
                                row_updated = true;
                            }
                            Err(e) => {
                                failed += 1;
                                errors.push(CsvUpdateErrorOutput {
                                    email: row.email.clone(),
                                    error: format!("company update failed: {}", e),
                                });
                                if let Some(ref pb) = progress_bar {
                                    pb.inc(1);
                                }
                                continue;
                            }
                        }
                    }
                    Ok(None) => {
                        failed += 1;
                        errors.push(CsvUpdateErrorOutput {
                            email: row.email.clone(),
                            error: "user not found in account".to_string(),
                        });
                        if let Some(ref pb) = progress_bar {
                            pb.inc(1);
                        }
                        continue;
                    }
                    Err(e) => {
                        failed += 1;
                        errors.push(CsvUpdateErrorOutput {
                            email: row.email.clone(),
                            error: format!("user lookup failed: {}", e),
                        });
                        if let Some(ref pb) = progress_bar {
                            pb.inc(1);
                        }
                        continue;
                    }
                }
            }

            // Update role across projects if specified
            if let Some(ref role_value) = row.role {
                let users_client = Arc::new(ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                ));

                let bulk_config = BulkConfig {
                    concurrency: concurrency.min(50),
                    dry_run: false,
                    ..Default::default()
                };

                let noop_progress = |_: ProgressUpdate| {};

                match raps_admin::bulk_update_role(
                    &admin_client,
                    users_client,
                    &account_id,
                    &row.email,
                    role_value,
                    None,
                    &project_filter,
                    bulk_config,
                    noop_progress,
                )
                .await
                {
                    Ok(result) => {
                        if result.failed > 0 {
                            failed += 1;
                            errors.push(CsvUpdateErrorOutput {
                                email: row.email.clone(),
                                error: format!(
                                    "role update: {}/{} projects failed",
                                    result.failed, result.total
                                ),
                            });
                        } else {
                            row_updated = true;
                        }
                    }
                    Err(e) => {
                        failed += 1;
                        errors.push(CsvUpdateErrorOutput {
                            email: row.email.clone(),
                            error: format!("role update failed: {}", e),
                        });
                    }
                }
            }

            if row_updated {
                updated += 1;
            }
        }

        if let Some(ref pb) = progress_bar {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    let output = CsvUpdateResultOutput {
        total: rows.len(),
        updated,
        skipped,
        failed,
        errors,
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "CSV Update Results:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "Total:".bold(), output.total);
            println!(
                "{:<15} {}",
                "Updated:".bold(),
                output.updated.to_string().green()
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
            println!("{}", "─".repeat(60));

            if !output.errors.is_empty() {
                println!("\n{}", "Errors:".red().bold());
                for err in &output.errors {
                    println!("  {} {} - {}", "✗".red(), err.email, err.error.dimmed());
                }
            }

            if output.failed == 0 {
                println!(
                    "\n{} All {} user(s) updated successfully!",
                    "✓".green().bold(),
                    output.updated
                );
            } else {
                println!(
                    "\n{} Completed with {} failure(s)",
                    "⚠".yellow().bold(),
                    output.failed
                );
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    if output.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
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

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account Admin Bulk Management Commands
//!
//! Commands for bulk user management across ACC/BIM 360 projects:
//! - Add users to multiple projects
//! - Remove users from multiple projects
//! - Update user roles across projects
//! - Manage folder-level permissions

mod csv_ops;
mod folder;
mod operations;
mod project;
mod user;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Subcommand, ValueEnum};
use indicatif::ProgressBar;

use raps_admin::{PermissionLevel, ProgressUpdate, ProjectFilter};
use raps_dm::DataManagementClient;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;

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

    /// List companies in an account
    #[command(name = "company-list")]
    CompanyList {
        /// Account ID (defaults to APS_ACCOUNT_ID env var)
        #[arg(short, long)]
        account: Option<String>,
    },
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

        /// Parallel requests (defaults to global --concurrency, max: 50)
        #[arg(long)]
        concurrency: Option<usize>,

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

        /// Parallel requests (defaults to global --concurrency, max: 50)
        #[arg(long)]
        concurrency: Option<usize>,

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

        /// Parallel requests (defaults to global --concurrency, max: 50)
        #[arg(long)]
        concurrency: Option<usize>,

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

    /// Import new users to a project from CSV
    #[command(name = "import")]
    Import {
        /// Project ID to import users into
        #[arg(short, long)]
        project: String,

        /// CSV file with columns: email, role_id (optional)
        #[arg(long, value_name = "FILE")]
        from_csv: PathBuf,
    },

    /// Add a user as Project Admin to all active projects in an account
    #[command(name = "add-to-all-projects")]
    AddToAllProjects {
        /// Email address of the user to add
        email: String,

        /// Account ID (defaults to APS_ACCOUNT_ID env var)
        #[arg(short, long)]
        account: Option<String>,

        /// Parallel requests (defaults to global --concurrency, max: 50)
        #[arg(long)]
        concurrency: Option<usize>,

        /// Preview changes without executing
        #[arg(long)]
        dry_run: bool,
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

        /// Parallel requests (defaults to global --concurrency, max: 50)
        #[arg(long)]
        concurrency: Option<usize>,

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

    /// Create a new project
    Create {
        /// Account ID (defaults to APS_ACCOUNT_ID env var)
        #[arg(short, long)]
        account: Option<String>,

        /// Project name
        #[arg(short, long)]
        name: String,

        /// Project type
        #[arg(short = 't', long)]
        r#type: Option<String>,

        /// Project classification (production, template, component, sample)
        #[arg(long)]
        classification: Option<String>,

        /// Project start date (ISO 8601 format)
        #[arg(long)]
        start_date: Option<String>,

        /// Project end date (ISO 8601 format)
        #[arg(long)]
        end_date: Option<String>,

        /// Time zone (e.g., "America/New_York")
        #[arg(long)]
        timezone: Option<String>,
    },

    /// Update an existing project
    Update {
        /// Account ID (defaults to APS_ACCOUNT_ID env var)
        #[arg(short, long)]
        account: Option<String>,

        /// Project ID
        #[arg(short, long)]
        project: String,

        /// New project name
        #[arg(short, long)]
        name: Option<String>,

        /// New project status (active, archived, suspended)
        #[arg(long)]
        status: Option<String>,

        /// New start date (ISO 8601 format)
        #[arg(long)]
        start_date: Option<String>,

        /// New end date (ISO 8601 format)
        #[arg(long)]
        end_date: Option<String>,
    },

    /// Archive a project (sets status to archived)
    Archive {
        /// Account ID (defaults to APS_ACCOUNT_ID env var)
        #[arg(short, long)]
        account: Option<String>,

        /// Project ID
        #[arg(short, long)]
        project: String,
    },
}

/// Operation management subcommands
#[derive(Debug, Subcommand)]
pub enum OperationCommands {
    /// Check status of a bulk operation
    Status {
        /// Operation ID (defaults to most recent)
        operation_id: Option<uuid::Uuid>,
    },

    /// Resume an interrupted operation
    Resume {
        /// Operation ID to resume (defaults to most recent incomplete)
        operation_id: Option<uuid::Uuid>,

        /// Override concurrency setting
        #[arg(long)]
        concurrency: Option<usize>,
    },

    /// Cancel an in-progress operation
    Cancel {
        /// Operation ID to cancel
        operation_id: Option<uuid::Uuid>,

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

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Resolve the account ID from explicit arg, env var, or hub auto-discovery.
///
/// Priority:
/// 1. `--account <id>` argument
/// 2. `APS_ACCOUNT_ID` environment variable
/// 3. Auto-discover: call `raps hub list`; use the sole hub or prompt when multiple
pub(crate) async fn resolve_account_id(
    account: Option<String>,
    dm_client: &DataManagementClient,
) -> Result<String> {
    // Fast path: explicit arg or env var
    if let Some(id) = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok()) {
        if !id.is_empty() {
            return Ok(id);
        }
    }

    // Auto-discover via hub list (requires 3-legged auth)
    let hubs = dm_client
        .list_hubs()
        .await
        .context("Failed to list hubs for account auto-discovery. Use --account or set APS_ACCOUNT_ID.")?;

    match hubs.len() {
        0 => anyhow::bail!(
            "No accessible hubs found. Use --account <id> or set APS_ACCOUNT_ID."
        ),
        1 => {
            let id = hubs[0].id.clone();
            eprintln!(
                "Using account: {} ({})",
                hubs[0].attributes.name, id
            );
            Ok(id)
        }
        _ => {
            // Multiple hubs: prompt interactively or bail in non-interactive mode
            let items: Vec<String> = hubs
                .iter()
                .map(|h| format!("{} — {}", h.id, h.attributes.name))
                .collect();
            let idx = raps_kernel::prompts::spawn_prompt(move || {
                raps_kernel::prompts::select("Select account", &items)
            })
            .await?;
            Ok(hubs[idx].id.clone())
        }
    }
}

pub(crate) fn parse_filter_with_ids(
    filter: &Option<String>,
    project_ids: &Option<PathBuf>,
) -> Result<ProjectFilter> {
    let mut project_filter = match filter {
        Some(f) => ProjectFilter::from_expression(f)?,
        None => ProjectFilter::new(),
    };
    if let Some(ids_file) = project_ids {
        let content = std::fs::read_to_string(ids_file)?;
        let ids: Vec<String> = content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        project_filter.include_ids = Some(ids);
    }
    Ok(project_filter)
}

pub(crate) fn create_bulk_progress_bar(output_format: OutputFormat) -> Option<ProgressBar> {
    if !output_format.supports_colors() {
        return None;
    }
    Some(raps_kernel::progress::bulk_progress(0, ""))
}

pub(crate) fn make_progress_callback(pb: Option<ProgressBar>) -> impl Fn(ProgressUpdate) {
    move |progress: ProgressUpdate| {
        if let Some(ref pb) = pb {
            pb.set_length(progress.total as u64);
            pb.set_position((progress.completed + progress.failed + progress.skipped) as u64);
            pb.set_message(format!(
                "✓{} ○{} ✗{}",
                progress.completed, progress.skipped, progress.failed
            ));
        }
    }
}

impl AdminCommands {
    pub async fn execute(
        self,
        config: &Config,
        auth_client: &AuthClient,
        dm_client: &DataManagementClient,
        output_format: OutputFormat,
        concurrency: usize,
    ) -> Result<()> {
        match self {
            AdminCommands::User(cmd) => {
                cmd.execute(config, auth_client, dm_client, output_format, concurrency)
                    .await
            }
            AdminCommands::Folder(cmd) => {
                cmd.execute(config, auth_client, dm_client, output_format, concurrency)
                    .await
            }
            AdminCommands::Project(cmd) => {
                cmd.execute(config, auth_client, dm_client, output_format).await
            }
            AdminCommands::Operation(cmd) => cmd.execute(output_format).await,
            AdminCommands::CompanyList { account } => {
                project::execute_company_list(config, auth_client, dm_client, account, output_format).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::csv_ops::{CsvUpdateErrorOutput, CsvUpdateResultOutput};
    use super::user::UserListOutput;

    #[test]
    fn test_csv_update_row_deserialization() {
        let csv_data = "email,role,company\njohn@example.com,Project Admin,Acme Corp\n";
        let mut rdr = csv::ReaderBuilder::new().from_reader(csv_data.as_bytes());
        let row: super::csv_ops::CsvUpdateRow = rdr.deserialize().next().unwrap().unwrap();
        assert_eq!(row.email, "john@example.com");
        assert_eq!(row.role.unwrap(), "Project Admin");
        assert_eq!(row.company.unwrap(), "Acme Corp");
    }

    #[test]
    fn test_csv_update_row_minimal() {
        let csv_data = "email,role,company\njohn@example.com,,\n";
        let mut rdr = csv::ReaderBuilder::new().from_reader(csv_data.as_bytes());
        let row: super::csv_ops::CsvUpdateRow = rdr.deserialize().next().unwrap().unwrap();
        assert_eq!(row.email, "john@example.com");
        // Empty strings from CSV become Some("") rather than None
        assert!(
            row.role.is_none() || row.role.as_deref() == Some(""),
            "Expected None or empty string for role, got {:?}",
            row.role
        );
        assert!(
            row.company.is_none() || row.company.as_deref() == Some(""),
            "Expected None or empty string for company, got {:?}",
            row.company
        );
    }

    #[test]
    fn test_csv_update_row_email_only_header() {
        // When only email column is present, optional fields should default
        let csv_data = "email\njohn@example.com\n";
        let mut rdr = csv::ReaderBuilder::new().from_reader(csv_data.as_bytes());
        let row: super::csv_ops::CsvUpdateRow = rdr.deserialize().next().unwrap().unwrap();
        assert_eq!(row.email, "john@example.com");
        assert!(row.role.is_none());
        assert!(row.company.is_none());
    }

    #[test]
    fn test_csv_update_row_multiple_rows() {
        let csv_data = "\
email,role,company
alice@example.com,Project Admin,Alpha Inc
bob@example.com,Document Manager,Beta LLC
carol@example.com,,
";
        let mut rdr = csv::ReaderBuilder::new().from_reader(csv_data.as_bytes());
        let rows: Vec<super::csv_ops::CsvUpdateRow> =
            rdr.deserialize().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].email, "alice@example.com");
        assert_eq!(rows[1].email, "bob@example.com");
        assert_eq!(rows[1].role.as_deref(), Some("Document Manager"));
        assert_eq!(rows[2].email, "carol@example.com");
    }

    #[test]
    fn test_user_list_output_serialization() {
        let output = UserListOutput {
            id: "abc-123".to_string(),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            role: "Project Admin".to_string(),
            company: Some("Acme Corp".to_string()),
            status: Some("active".to_string()),
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"email\":\"test@example.com\""));
        assert!(json.contains("\"name\":\"Test User\""));
        assert!(json.contains("\"id\":\"abc-123\""));
        assert!(json.contains("\"role\":\"Project Admin\""));
        assert!(json.contains("\"company\":\"Acme Corp\""));
        assert!(json.contains("\"status\":\"active\""));
    }

    #[test]
    fn test_user_list_output_skips_none_fields() {
        let output = UserListOutput {
            id: "abc-123".to_string(),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            role: "Admin".to_string(),
            company: None,
            status: None,
        };
        let json = serde_json::to_string(&output).unwrap();
        // Fields with skip_serializing_if = "Option::is_none" should be absent
        assert!(!json.contains("company"));
        assert!(!json.contains("status"));
    }

    #[test]
    fn test_csv_update_result_output_serialization() {
        let output = CsvUpdateResultOutput {
            total: 10,
            updated: 8,
            skipped: 1,
            failed: 1,
            errors: vec![CsvUpdateErrorOutput {
                email: "fail@test.com".to_string(),
                error: "not found".to_string(),
            }],
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"total\":10"));
        assert!(json.contains("\"updated\":8"));
        assert!(json.contains("\"skipped\":1"));
        assert!(json.contains("\"failed\":1"));
        assert!(json.contains("fail@test.com"));
        assert!(json.contains("not found"));
    }

    #[test]
    fn test_csv_update_result_output_empty_errors() {
        let output = CsvUpdateResultOutput {
            total: 5,
            updated: 5,
            skipped: 0,
            failed: 0,
            errors: vec![],
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"errors\":[]"));
    }

    #[test]
    fn test_csv_update_error_output_serialization() {
        let output = CsvUpdateErrorOutput {
            email: "bad@test.com".to_string(),
            error: "permission denied".to_string(),
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"email\":\"bad@test.com\""));
        assert!(json.contains("\"error\":\"permission denied\""));
    }

    #[test]
    fn test_format_project_status_active() {
        let result = super::project::format_project_status("active");
        // Should contain the original text (colored output still contains the word)
        assert!(result.contains("active"));
    }

    #[test]
    fn test_format_project_status_unknown() {
        let result = super::project::format_project_status("pending");
        assert_eq!(result, "pending");
    }

    #[test]
    fn test_format_user_status_active() {
        let result = super::user::format_user_status("active");
        assert!(result.contains("active"));
    }

    #[test]
    fn test_format_user_status_unknown() {
        let result = super::user::format_user_status("unknown");
        assert_eq!(result, "unknown");
    }
}

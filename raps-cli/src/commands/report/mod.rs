// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Portfolio report commands
//!
//! Aggregated reports across multiple projects: RFI summaries, issue summaries,
//! submittals, checklists, and assets.

mod extended_reports;
mod issues_report;
mod rfi_report;
#[cfg(test)]
mod tests;

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use indicatif::ProgressBar;
use serde::Serialize;

use raps_acc::admin::AccountAdminClient;
use raps_admin::ProjectFilter;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
#[allow(clippy::enum_variant_names)]
pub enum ReportCommands {
    /// RFI summary across portfolio projects
    RfiSummary {
        /// Account ID (defaults to APS_ACCOUNT_ID env var)
        #[arg(short, long)]
        account: Option<String>,

        /// Project filter expression (e.g., "status:active,name:*Hospital*")
        #[arg(short, long)]
        filter: Option<String>,

        /// Filter RFIs by status (open, answered, closed, void)
        #[arg(long)]
        status: Option<String>,

        /// Only include RFIs created after this date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
    },

    /// Issue summary across portfolio projects
    IssuesSummary {
        /// Account ID (defaults to APS_ACCOUNT_ID env var)
        #[arg(short, long)]
        account: Option<String>,

        /// Project filter expression
        #[arg(short, long)]
        filter: Option<String>,

        /// Filter issues by status (open, closed, etc.)
        #[arg(long)]
        status: Option<String>,

        /// Only include issues created after this date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
    },

    /// Submittal summary across portfolio projects
    SubmittalsSummary {
        /// Account ID (defaults to APS_ACCOUNT_ID env var)
        #[arg(short, long)]
        account: Option<String>,

        /// Project filter expression
        #[arg(short, long)]
        filter: Option<String>,

        /// Filter submittals by status
        #[arg(long)]
        status: Option<String>,
    },

    /// Checklist summary across portfolio projects
    ChecklistsSummary {
        /// Account ID (defaults to APS_ACCOUNT_ID env var)
        #[arg(short, long)]
        account: Option<String>,

        /// Project filter expression
        #[arg(short, long)]
        filter: Option<String>,

        /// Filter checklists by status
        #[arg(long)]
        status: Option<String>,
    },

    /// Asset summary across portfolio projects
    AssetsSummary {
        /// Account ID (defaults to APS_ACCOUNT_ID env var)
        #[arg(short, long)]
        account: Option<String>,

        /// Project filter expression
        #[arg(short, long)]
        filter: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Summary types
// ---------------------------------------------------------------------------

#[derive(Serialize, schemars::JsonSchema)]
pub(super) struct RfiProjectSummary {
    pub(super) project_id: String,
    pub(super) project_name: String,
    pub(super) total: usize,
    pub(super) open: usize,
    pub(super) answered: usize,
    pub(super) closed: usize,
    pub(super) void: usize,
}

#[derive(Serialize, schemars::JsonSchema)]
pub(super) struct IssueProjectSummary {
    pub(super) project_id: String,
    pub(super) project_name: String,
    pub(super) total: usize,
    pub(super) open: usize,
    pub(super) closed: usize,
    pub(super) other: usize,
}

#[derive(Serialize, schemars::JsonSchema)]
pub(super) struct SubmittalProjectSummary {
    pub(super) project_id: String,
    pub(super) project_name: String,
    pub(super) total: usize,
}

#[derive(Serialize, schemars::JsonSchema)]
pub(super) struct ChecklistProjectSummary {
    pub(super) project_id: String,
    pub(super) project_name: String,
    pub(super) total: usize,
}

#[derive(Serialize, schemars::JsonSchema)]
pub(super) struct AssetProjectSummary {
    pub(super) project_id: String,
    pub(super) project_name: String,
    pub(super) total: usize,
}

#[derive(Serialize, schemars::JsonSchema)]
pub(super) struct ReportSummaryOutput<T: Serialize> {
    pub(super) total_projects: usize,
    pub(super) projects: Vec<T>,
}

// ---------------------------------------------------------------------------
// Shared traits
// ---------------------------------------------------------------------------

pub(super) trait HasProjectName {
    fn project_name(&self) -> &str;
}

impl HasProjectName for SubmittalProjectSummary {
    fn project_name(&self) -> &str {
        &self.project_name
    }
}

impl HasProjectName for ChecklistProjectSummary {
    fn project_name(&self) -> &str {
        &self.project_name
    }
}

impl HasProjectName for AssetProjectSummary {
    fn project_name(&self) -> &str {
        &self.project_name
    }
}

pub(super) trait HasStatus {
    fn status(&self) -> &str;
}

impl HasStatus for raps_acc::Rfi {
    fn status(&self) -> &str {
        &self.status
    }
}

impl HasStatus for raps_acc::Issue {
    fn status(&self) -> &str {
        &self.status
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

pub(super) fn get_account_id(account: Option<String>) -> Result<String> {
    match account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok()) {
        Some(id) if !id.is_empty() => Ok(id),
        _ => {
            anyhow::bail!(
                "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
            );
        }
    }
}

pub(super) fn parse_project_filter(filter: &Option<String>) -> Result<ProjectFilter> {
    match filter {
        Some(f) => Ok(ProjectFilter::from_expression(f)?),
        None => Ok(ProjectFilter::new()),
    }
}

pub(super) fn create_progress_bar(
    output_format: OutputFormat,
    count: u64,
    message: &str,
) -> Option<ProgressBar> {
    if !output_format.supports_colors() {
        return None;
    }
    Some(raps_kernel::progress::bulk_progress(count, message))
}

pub(super) fn print_report_header(
    output_format: OutputFormat,
    label: &str,
    account_id: &str,
    filter: &Option<String>,
) {
    if !output_format.supports_colors() {
        return;
    }
    println!(
        "\n{} {} for account {}",
        "→".cyan(),
        label,
        account_id.cyan()
    );
    if let Some(f) = filter {
        println!("  Filter: {}", f);
    }
    println!();
}

pub(super) fn truncate_name(name: &str) -> String {
    if name.len() > 28 {
        format!("{}...", &name[..25])
    } else {
        name.to_string()
    }
}

pub(super) fn print_simple_table<T, F>(
    title: &str,
    output: &ReportSummaryOutput<T>,
    output_format: OutputFormat,
    get_total: F,
) -> Result<()>
where
    T: Serialize + HasProjectName,
    F: Fn(&T) -> usize,
{
    match output_format {
        OutputFormat::Table => {
            let grand_total: usize = output.projects.iter().map(&get_total).sum();

            println!("{}", format!("{} Portfolio Summary:", title).bold());
            println!("{}", "─".repeat(45));
            println!("{:<30} {:>8}", "Project".bold(), "Total".bold());
            println!("{}", "─".repeat(45));

            for s in &output.projects {
                println!(
                    "{:<30} {:>8}",
                    truncate_name(s.project_name()),
                    get_total(s).to_string().cyan(),
                );
            }

            println!("{}", "─".repeat(45));
            println!(
                "{:<30} {:>8}",
                "TOTAL".bold(),
                grand_total.to_string().bold(),
            );
            println!(
                "\n{} {} projects scanned",
                "→".cyan(),
                output.total_projects
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }
    Ok(())
}

pub(super) fn count_status(items: &[impl HasStatus], status: &str) -> usize {
    items
        .iter()
        .filter(|item| item.status().eq_ignore_ascii_case(status))
        .count()
}

// ---------------------------------------------------------------------------
// Shared project-listing boilerplate
// ---------------------------------------------------------------------------

pub(super) struct ReportContext {
    pub(super) http_config: HttpClientConfig,
    pub(super) filtered_projects: Vec<raps_acc::types::AccountProject>,
}

pub(super) async fn prepare_report(
    config: &Config,
    auth_client: &AuthClient,
    account: Option<String>,
    filter: &Option<String>,
    label: &str,
    output_format: OutputFormat,
) -> Result<Option<ReportContext>> {
    let account_id = get_account_id(account)?;
    let project_filter = parse_project_filter(filter)?;
    print_report_header(output_format, label, &account_id, filter);

    let http_config = HttpClientConfig::default();
    let admin_client = AccountAdminClient::new_with_http_config(
        config.clone(),
        auth_client.clone(),
        http_config.clone(),
    );

    let all_projects = admin_client.list_all_projects(&account_id).await?;
    let filtered_projects = project_filter.apply(all_projects);

    if filtered_projects.is_empty() {
        if output_format.supports_colors() {
            println!("{}", "No projects found matching the filter.".yellow());
        }
        return Ok(None);
    }

    Ok(Some(ReportContext {
        http_config,
        filtered_projects,
    }))
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

impl ReportCommands {
    pub async fn execute(
        self,
        config: &Config,
        auth_client: &AuthClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            ReportCommands::RfiSummary {
                account,
                filter,
                status,
                since,
            } => {
                rfi_report::rfi_summary(
                    config,
                    auth_client,
                    account,
                    filter,
                    status,
                    since,
                    output_format,
                )
                .await
            }
            ReportCommands::IssuesSummary {
                account,
                filter,
                status,
                since,
            } => {
                issues_report::issues_summary(
                    config,
                    auth_client,
                    account,
                    filter,
                    status,
                    since,
                    output_format,
                )
                .await
            }
            ReportCommands::SubmittalsSummary {
                account,
                filter,
                status,
            } => {
                extended_reports::submittals_summary(
                    config,
                    auth_client,
                    account,
                    filter,
                    status,
                    output_format,
                )
                .await
            }
            ReportCommands::ChecklistsSummary {
                account,
                filter,
                status,
            } => {
                extended_reports::checklists_summary(
                    config,
                    auth_client,
                    account,
                    filter,
                    status,
                    output_format,
                )
                .await
            }
            ReportCommands::AssetsSummary { account, filter } => {
                extended_reports::assets_summary(
                    config,
                    auth_client,
                    account,
                    filter,
                    output_format,
                )
                .await
            }
        }
    }
}

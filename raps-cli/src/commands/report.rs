// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Portfolio report commands
//!
//! Aggregated reports across multiple projects: RFI summaries, issue summaries.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

use raps_acc::admin::AccountAdminClient;
use raps_acc::{AccClient, IssuesClient, RfiClient};
use raps_admin::ProjectFilter;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
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

#[derive(Serialize)]
struct RfiProjectSummary {
    project_id: String,
    project_name: String,
    total: usize,
    open: usize,
    answered: usize,
    closed: usize,
    void: usize,
}

#[derive(Serialize)]
struct IssueProjectSummary {
    project_id: String,
    project_name: String,
    total: usize,
    open: usize,
    closed: usize,
    other: usize,
}

#[derive(Serialize)]
struct SubmittalProjectSummary {
    project_id: String,
    project_name: String,
    total: usize,
}

#[derive(Serialize)]
struct ChecklistProjectSummary {
    project_id: String,
    project_name: String,
    total: usize,
}

#[derive(Serialize)]
struct AssetProjectSummary {
    project_id: String,
    project_name: String,
    total: usize,
}

#[derive(Serialize)]
struct ReportSummaryOutput<T: Serialize> {
    total_projects: usize,
    projects: Vec<T>,
}

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
                rfi_summary(
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
                issues_summary(
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
                submittals_summary(config, auth_client, account, filter, status, output_format)
                    .await
            }
            ReportCommands::ChecklistsSummary {
                account,
                filter,
                status,
            } => {
                checklists_summary(config, auth_client, account, filter, status, output_format)
                    .await
            }
            ReportCommands::AssetsSummary { account, filter } => {
                assets_summary(config, auth_client, account, filter, output_format).await
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn rfi_summary(
    config: &Config,
    auth_client: &AuthClient,
    account: Option<String>,
    filter: Option<String>,
    status_filter: Option<String>,
    _since: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());
    let account_id = match account_id {
        Some(id) if !id.is_empty() => id,
        _ => {
            anyhow::bail!(
                "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
            );
        }
    };

    let project_filter = if let Some(ref f) = filter {
        ProjectFilter::from_expression(f)?
    } else {
        ProjectFilter::new()
    };

    if output_format.supports_colors() {
        println!(
            "\n{} RFI summary for account {}",
            "→".cyan(),
            account_id.cyan()
        );
        if let Some(ref f) = filter {
            println!("  Filter: {}", f);
        }
        println!();
    }

    // List projects
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
        return Ok(());
    }

    // Progress bar
    let progress_bar = if output_format.supports_colors() {
        let pb = ProgressBar::new(filtered_projects.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} Fetching RFIs...")
                .unwrap()
                .progress_chars("=>-"),
        );
        Some(pb)
    } else {
        None
    };

    let rfi_client =
        RfiClient::new_with_http_config(config.clone(), auth_client.clone(), http_config);

    let mut summaries = Vec::new();

    for project in &filtered_projects {
        if let Some(ref pb) = progress_bar {
            pb.set_message(format!("{}", project.name));
        }

        // Fetch RFIs for this project
        match rfi_client.list_rfis(&project.id).await {
            Ok(rfis) => {
                // Apply status filter if provided
                let filtered_rfis: Vec<_> = if let Some(ref sf) = status_filter {
                    rfis.into_iter()
                        .filter(|r| r.status.to_lowercase() == sf.to_lowercase())
                        .collect()
                } else {
                    rfis
                };

                let total = filtered_rfis.len();
                let open = filtered_rfis
                    .iter()
                    .filter(|r| r.status.to_lowercase() == "open")
                    .count();
                let answered = filtered_rfis
                    .iter()
                    .filter(|r| r.status.to_lowercase() == "answered")
                    .count();
                let closed = filtered_rfis
                    .iter()
                    .filter(|r| r.status.to_lowercase() == "closed")
                    .count();
                let void = filtered_rfis
                    .iter()
                    .filter(|r| r.status.to_lowercase() == "void")
                    .count();

                summaries.push(RfiProjectSummary {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    total,
                    open,
                    answered,
                    closed,
                    void,
                });
            }
            Err(_) => {
                // Skip projects where RFI access fails (permission issues)
                summaries.push(RfiProjectSummary {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    total: 0,
                    open: 0,
                    answered: 0,
                    closed: 0,
                    void: 0,
                });
            }
        }

        if let Some(ref pb) = progress_bar {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    let output = ReportSummaryOutput {
        total_projects: summaries.len(),
        projects: summaries,
    };

    match output_format {
        OutputFormat::Table => {
            // Calculate totals
            let grand_total: usize = output.projects.iter().map(|s| s.total).sum();
            let grand_open: usize = output.projects.iter().map(|s| s.open).sum();
            let grand_answered: usize = output.projects.iter().map(|s| s.answered).sum();
            let grand_closed: usize = output.projects.iter().map(|s| s.closed).sum();

            println!("{}", "RFI Portfolio Summary:".bold());
            println!("{}", "─".repeat(100));
            println!(
                "{:<30} {:>8} {:>8} {:>10} {:>8} {:>8}",
                "Project".bold(),
                "Total".bold(),
                "Open".bold(),
                "Answered".bold(),
                "Closed".bold(),
                "Void".bold()
            );
            println!("{}", "─".repeat(100));

            for s in &output.projects {
                let name = if s.project_name.len() > 28 {
                    format!("{}...", &s.project_name[..25])
                } else {
                    s.project_name.clone()
                };
                println!(
                    "{:<30} {:>8} {:>8} {:>10} {:>8} {:>8}",
                    name,
                    s.total.to_string().cyan(),
                    if s.open > 0 {
                        s.open.to_string().yellow().to_string()
                    } else {
                        s.open.to_string()
                    },
                    if s.answered > 0 {
                        s.answered.to_string().cyan().to_string()
                    } else {
                        s.answered.to_string()
                    },
                    if s.closed > 0 {
                        s.closed.to_string().green().to_string()
                    } else {
                        s.closed.to_string()
                    },
                    s.void
                );
            }

            println!("{}", "─".repeat(100));
            println!(
                "{:<30} {:>8} {:>8} {:>10} {:>8}",
                "TOTAL".bold(),
                grand_total.to_string().bold(),
                grand_open.to_string().yellow().bold(),
                grand_answered.to_string().cyan().bold(),
                grand_closed.to_string().green().bold()
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

#[allow(clippy::too_many_arguments)]
async fn issues_summary(
    config: &Config,
    auth_client: &AuthClient,
    account: Option<String>,
    filter: Option<String>,
    status_filter: Option<String>,
    _since: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());
    let account_id = match account_id {
        Some(id) if !id.is_empty() => id,
        _ => {
            anyhow::bail!(
                "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
            );
        }
    };

    let project_filter = if let Some(ref f) = filter {
        ProjectFilter::from_expression(f)?
    } else {
        ProjectFilter::new()
    };

    if output_format.supports_colors() {
        println!(
            "\n{} Issues summary for account {}",
            "→".cyan(),
            account_id.cyan()
        );
        if let Some(ref f) = filter {
            println!("  Filter: {}", f);
        }
        println!();
    }

    // List projects
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
        return Ok(());
    }

    // Progress bar
    let progress_bar = if output_format.supports_colors() {
        let pb = ProgressBar::new(filtered_projects.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} Fetching issues...")
                .unwrap()
                .progress_chars("=>-"),
        );
        Some(pb)
    } else {
        None
    };

    let issues_client =
        IssuesClient::new_with_http_config(config.clone(), auth_client.clone(), http_config);

    let mut summaries = Vec::new();

    for project in &filtered_projects {
        if let Some(ref pb) = progress_bar {
            pb.set_message(format!("{}", project.name));
        }

        match issues_client.list_issues(&project.id, None).await {
            Ok(issues) => {
                let filtered_issues: Vec<_> = if let Some(ref sf) = status_filter {
                    issues
                        .into_iter()
                        .filter(|i| i.status.to_lowercase() == sf.to_lowercase())
                        .collect()
                } else {
                    issues
                };

                let total = filtered_issues.len();
                let open = filtered_issues
                    .iter()
                    .filter(|i| i.status.to_lowercase() == "open")
                    .count();
                let closed = filtered_issues
                    .iter()
                    .filter(|i| i.status.to_lowercase() == "closed")
                    .count();
                let other = total - open - closed;

                summaries.push(IssueProjectSummary {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    total,
                    open,
                    closed,
                    other,
                });
            }
            Err(_) => {
                summaries.push(IssueProjectSummary {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    total: 0,
                    open: 0,
                    closed: 0,
                    other: 0,
                });
            }
        }

        if let Some(ref pb) = progress_bar {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    let output = ReportSummaryOutput {
        total_projects: summaries.len(),
        projects: summaries,
    };

    match output_format {
        OutputFormat::Table => {
            let grand_total: usize = output.projects.iter().map(|s| s.total).sum();
            let grand_open: usize = output.projects.iter().map(|s| s.open).sum();
            let grand_closed: usize = output.projects.iter().map(|s| s.closed).sum();
            let grand_other: usize = output.projects.iter().map(|s| s.other).sum();

            println!("{}", "Issues Portfolio Summary:".bold());
            println!("{}", "─".repeat(85));
            println!(
                "{:<30} {:>8} {:>8} {:>8} {:>8}",
                "Project".bold(),
                "Total".bold(),
                "Open".bold(),
                "Closed".bold(),
                "Other".bold()
            );
            println!("{}", "─".repeat(85));

            for s in &output.projects {
                let name = if s.project_name.len() > 28 {
                    format!("{}...", &s.project_name[..25])
                } else {
                    s.project_name.clone()
                };
                println!(
                    "{:<30} {:>8} {:>8} {:>8} {:>8}",
                    name,
                    s.total.to_string().cyan(),
                    if s.open > 0 {
                        s.open.to_string().yellow().to_string()
                    } else {
                        s.open.to_string()
                    },
                    if s.closed > 0 {
                        s.closed.to_string().green().to_string()
                    } else {
                        s.closed.to_string()
                    },
                    s.other
                );
            }

            println!("{}", "─".repeat(85));
            println!(
                "{:<30} {:>8} {:>8} {:>8} {:>8}",
                "TOTAL".bold(),
                grand_total.to_string().bold(),
                grand_open.to_string().yellow().bold(),
                grand_closed.to_string().green().bold(),
                grand_other
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

async fn submittals_summary(
    config: &Config,
    auth_client: &AuthClient,
    account: Option<String>,
    filter: Option<String>,
    status_filter: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());
    let account_id = match account_id {
        Some(id) if !id.is_empty() => id,
        _ => {
            anyhow::bail!(
                "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
            );
        }
    };

    let project_filter = if let Some(ref f) = filter {
        ProjectFilter::from_expression(f)?
    } else {
        ProjectFilter::new()
    };

    if output_format.supports_colors() {
        println!(
            "\n{} Submittals summary for account {}",
            "→".cyan(),
            account_id.cyan()
        );
        if let Some(ref f) = filter {
            println!("  Filter: {}", f);
        }
        println!();
    }

    // List projects
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
        return Ok(());
    }

    // Progress bar
    let progress_bar = if output_format.supports_colors() {
        let pb = ProgressBar::new(filtered_projects.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} Fetching submittals...",
                )
                .unwrap()
                .progress_chars("=>-"),
        );
        Some(pb)
    } else {
        None
    };

    let acc_client = AccClient::new(config.clone(), auth_client.clone());

    let mut summaries = Vec::new();

    for project in &filtered_projects {
        if let Some(ref pb) = progress_bar {
            pb.set_message(format!("{}", project.name));
        }

        match acc_client.list_submittals(&project.id).await {
            Ok(submittals) => {
                let filtered_submittals: Vec<_> = if let Some(ref sf) = status_filter {
                    submittals
                        .into_iter()
                        .filter(|s| s.status.to_lowercase() == sf.to_lowercase())
                        .collect()
                } else {
                    submittals
                };

                let total = filtered_submittals.len();

                summaries.push(SubmittalProjectSummary {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    total,
                });
            }
            Err(_) => {
                summaries.push(SubmittalProjectSummary {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    total: 0,
                });
            }
        }

        if let Some(ref pb) = progress_bar {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    let output = ReportSummaryOutput {
        total_projects: summaries.len(),
        projects: summaries,
    };

    match output_format {
        OutputFormat::Table => {
            let grand_total: usize = output.projects.iter().map(|s| s.total).sum();

            println!("{}", "Submittals Portfolio Summary:".bold());
            println!("{}", "─".repeat(45));
            println!("{:<30} {:>8}", "Project".bold(), "Total".bold());
            println!("{}", "─".repeat(45));

            for s in &output.projects {
                let name = if s.project_name.len() > 28 {
                    format!("{}...", &s.project_name[..25])
                } else {
                    s.project_name.clone()
                };
                println!("{:<30} {:>8}", name, s.total.to_string().cyan(),);
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

async fn checklists_summary(
    config: &Config,
    auth_client: &AuthClient,
    account: Option<String>,
    filter: Option<String>,
    status_filter: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());
    let account_id = match account_id {
        Some(id) if !id.is_empty() => id,
        _ => {
            anyhow::bail!(
                "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
            );
        }
    };

    let project_filter = if let Some(ref f) = filter {
        ProjectFilter::from_expression(f)?
    } else {
        ProjectFilter::new()
    };

    if output_format.supports_colors() {
        println!(
            "\n{} Checklists summary for account {}",
            "→".cyan(),
            account_id.cyan()
        );
        if let Some(ref f) = filter {
            println!("  Filter: {}", f);
        }
        println!();
    }

    // List projects
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
        return Ok(());
    }

    // Progress bar
    let progress_bar = if output_format.supports_colors() {
        let pb = ProgressBar::new(filtered_projects.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} Fetching checklists...",
                )
                .unwrap()
                .progress_chars("=>-"),
        );
        Some(pb)
    } else {
        None
    };

    let acc_client = AccClient::new(config.clone(), auth_client.clone());

    let mut summaries = Vec::new();

    for project in &filtered_projects {
        if let Some(ref pb) = progress_bar {
            pb.set_message(format!("{}", project.name));
        }

        match acc_client.list_checklists(&project.id).await {
            Ok(checklists) => {
                let filtered_checklists: Vec<_> = if let Some(ref sf) = status_filter {
                    checklists
                        .into_iter()
                        .filter(|c| c.status.to_lowercase() == sf.to_lowercase())
                        .collect()
                } else {
                    checklists
                };

                let total = filtered_checklists.len();

                summaries.push(ChecklistProjectSummary {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    total,
                });
            }
            Err(_) => {
                summaries.push(ChecklistProjectSummary {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    total: 0,
                });
            }
        }

        if let Some(ref pb) = progress_bar {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    let output = ReportSummaryOutput {
        total_projects: summaries.len(),
        projects: summaries,
    };

    match output_format {
        OutputFormat::Table => {
            let grand_total: usize = output.projects.iter().map(|s| s.total).sum();

            println!("{}", "Checklists Portfolio Summary:".bold());
            println!("{}", "─".repeat(45));
            println!("{:<30} {:>8}", "Project".bold(), "Total".bold());
            println!("{}", "─".repeat(45));

            for s in &output.projects {
                let name = if s.project_name.len() > 28 {
                    format!("{}...", &s.project_name[..25])
                } else {
                    s.project_name.clone()
                };
                println!("{:<30} {:>8}", name, s.total.to_string().cyan(),);
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

async fn assets_summary(
    config: &Config,
    auth_client: &AuthClient,
    account: Option<String>,
    filter: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());
    let account_id = match account_id {
        Some(id) if !id.is_empty() => id,
        _ => {
            anyhow::bail!(
                "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
            );
        }
    };

    let project_filter = if let Some(ref f) = filter {
        ProjectFilter::from_expression(f)?
    } else {
        ProjectFilter::new()
    };

    if output_format.supports_colors() {
        println!(
            "\n{} Assets summary for account {}",
            "→".cyan(),
            account_id.cyan()
        );
        if let Some(ref f) = filter {
            println!("  Filter: {}", f);
        }
        println!();
    }

    // List projects
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
        return Ok(());
    }

    // Progress bar
    let progress_bar = if output_format.supports_colors() {
        let pb = ProgressBar::new(filtered_projects.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} Fetching assets...")
                .unwrap()
                .progress_chars("=>-"),
        );
        Some(pb)
    } else {
        None
    };

    let acc_client = AccClient::new(config.clone(), auth_client.clone());

    let mut summaries = Vec::new();

    for project in &filtered_projects {
        if let Some(ref pb) = progress_bar {
            pb.set_message(format!("{}", project.name));
        }

        match acc_client.list_assets(&project.id).await {
            Ok(assets) => {
                let total = assets.len();

                summaries.push(AssetProjectSummary {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    total,
                });
            }
            Err(_) => {
                summaries.push(AssetProjectSummary {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    total: 0,
                });
            }
        }

        if let Some(ref pb) = progress_bar {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    let output = ReportSummaryOutput {
        total_projects: summaries.len(),
        projects: summaries,
    };

    match output_format {
        OutputFormat::Table => {
            let grand_total: usize = output.projects.iter().map(|s| s.total).sum();

            println!("{}", "Assets Portfolio Summary:".bold());
            println!("{}", "─".repeat(45));
            println!("{:<30} {:>8}", "Project".bold(), "Total".bold());
            println!("{}", "─".repeat(45));

            for s in &output.projects {
                let name = if s.project_name.len() > 28 {
                    format!("{}...", &s.project_name[..25])
                } else {
                    s.project_name.clone()
                };
                println!("{:<30} {:>8}", name, s.total.to_string().cyan(),);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfi_project_summary_serialization() {
        let summary = RfiProjectSummary {
            project_id: "proj-001".to_string(),
            project_name: "Hospital Wing A".to_string(),
            total: 25,
            open: 10,
            answered: 8,
            closed: 5,
            void: 2,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"project_id\":\"proj-001\""));
        assert!(json.contains("\"project_name\":\"Hospital Wing A\""));
        assert!(json.contains("\"total\":25"));
        assert!(json.contains("\"open\":10"));
        assert!(json.contains("\"answered\":8"));
        assert!(json.contains("\"closed\":5"));
        assert!(json.contains("\"void\":2"));
    }

    #[test]
    fn test_issue_project_summary_serialization() {
        let summary = IssueProjectSummary {
            project_id: "proj-002".to_string(),
            project_name: "Office Tower".to_string(),
            total: 40,
            open: 15,
            closed: 20,
            other: 5,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"project_id\":\"proj-002\""));
        assert!(json.contains("\"project_name\":\"Office Tower\""));
        assert!(json.contains("\"total\":40"));
        assert!(json.contains("\"open\":15"));
        assert!(json.contains("\"closed\":20"));
        assert!(json.contains("\"other\":5"));
    }

    #[test]
    fn test_submittal_project_summary_serialization() {
        let summary = SubmittalProjectSummary {
            project_id: "proj-003".to_string(),
            project_name: "Parking Garage".to_string(),
            total: 12,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"project_id\":\"proj-003\""));
        assert!(json.contains("\"total\":12"));
    }

    #[test]
    fn test_checklist_project_summary_serialization() {
        let summary = ChecklistProjectSummary {
            project_id: "proj-004".to_string(),
            project_name: "Bridge Inspection".to_string(),
            total: 7,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"project_name\":\"Bridge Inspection\""));
        assert!(json.contains("\"total\":7"));
    }

    #[test]
    fn test_asset_project_summary_serialization() {
        let summary = AssetProjectSummary {
            project_id: "proj-005".to_string(),
            project_name: "Data Center".to_string(),
            total: 150,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"project_name\":\"Data Center\""));
        assert!(json.contains("\"total\":150"));
    }

    #[test]
    fn test_report_summary_output_with_rfi_data() {
        let output = ReportSummaryOutput {
            total_projects: 2,
            projects: vec![
                RfiProjectSummary {
                    project_id: "p1".to_string(),
                    project_name: "Project A".to_string(),
                    total: 10,
                    open: 5,
                    answered: 3,
                    closed: 1,
                    void: 1,
                },
                RfiProjectSummary {
                    project_id: "p2".to_string(),
                    project_name: "Project B".to_string(),
                    total: 0,
                    open: 0,
                    answered: 0,
                    closed: 0,
                    void: 0,
                },
            ],
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"total_projects\":2"));
        assert!(json.contains("\"projects\":["));
        assert!(json.contains("Project A"));
        assert!(json.contains("Project B"));
    }

    #[test]
    fn test_report_summary_output_empty_projects() {
        let output: ReportSummaryOutput<IssueProjectSummary> = ReportSummaryOutput {
            total_projects: 0,
            projects: vec![],
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"total_projects\":0"));
        assert!(json.contains("\"projects\":[]"));
    }

    #[test]
    fn test_rfi_project_summary_zero_counts() {
        let summary = RfiProjectSummary {
            project_id: "empty".to_string(),
            project_name: "Empty Project".to_string(),
            total: 0,
            open: 0,
            answered: 0,
            closed: 0,
            void: 0,
        };
        let json = serde_json::to_string(&summary).unwrap();
        // Verify all count fields serialize as zero
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["total"], 0);
        assert_eq!(parsed["open"], 0);
        assert_eq!(parsed["answered"], 0);
        assert_eq!(parsed["closed"], 0);
        assert_eq!(parsed["void"], 0);
    }
}

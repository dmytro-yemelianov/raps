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

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn get_account_id(account: Option<String>) -> Result<String> {
    match account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok()) {
        Some(id) if !id.is_empty() => Ok(id),
        _ => {
            anyhow::bail!(
                "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
            );
        }
    }
}

fn parse_project_filter(filter: &Option<String>) -> Result<ProjectFilter> {
    match filter {
        Some(f) => Ok(ProjectFilter::from_expression(f)?),
        None => Ok(ProjectFilter::new()),
    }
}

fn create_progress_bar(
    output_format: OutputFormat,
    count: u64,
    message: &str,
) -> Option<ProgressBar> {
    if !output_format.supports_colors() {
        return None;
    }
    let template = format!(
        "{{spinner:.green}} [{{bar:40.cyan/blue}}] {{pos}}/{{len}} {}",
        message
    );
    let pb = ProgressBar::new(count);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(&template)
            .expect("valid progress template")
            .progress_chars("=>-"),
    );
    Some(pb)
}

fn print_report_header(
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

fn truncate_name(name: &str) -> String {
    if name.len() > 28 {
        format!("{}...", &name[..25])
    } else {
        name.to_string()
    }
}

fn print_simple_table<T, F>(
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

trait HasProjectName {
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

// ---------------------------------------------------------------------------
// Shared project-listing boilerplate
// ---------------------------------------------------------------------------

struct ReportContext {
    http_config: HttpClientConfig,
    filtered_projects: Vec<raps_acc::types::AccountProject>,
}

async fn prepare_report(
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
// RFI summary
// ---------------------------------------------------------------------------

fn count_status(items: &[impl HasStatus], status: &str) -> usize {
    items
        .iter()
        .filter(|item| item.status().eq_ignore_ascii_case(status))
        .count()
}

trait HasStatus {
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
    let ctx = match prepare_report(
        config,
        auth_client,
        account,
        &filter,
        "RFI summary",
        output_format,
    )
    .await?
    {
        Some(ctx) => ctx,
        None => return Ok(()),
    };

    let progress_bar = create_progress_bar(
        output_format,
        ctx.filtered_projects.len() as u64,
        "Fetching RFIs...",
    );

    let rfi_client =
        RfiClient::new_with_http_config(config.clone(), auth_client.clone(), ctx.http_config);

    let mut summaries = Vec::new();

    for project in &ctx.filtered_projects {
        if let Some(ref pb) = progress_bar {
            pb.set_message(project.name.to_string());
        }

        match rfi_client.list_rfis(&project.id).await {
            Ok(rfis) => {
                // Compute status breakdown from the UNFILTERED list
                let open = count_status(&rfis, "open");
                let answered = count_status(&rfis, "answered");
                let closed = count_status(&rfis, "closed");
                let void = count_status(&rfis, "void");

                // Total reflects filtered count when a status filter is active
                let total = if let Some(ref sf) = status_filter {
                    rfis.iter()
                        .filter(|r| r.status.eq_ignore_ascii_case(sf))
                        .count()
                } else {
                    rfis.len()
                };

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
                println!(
                    "{:<30} {:>8} {:>8} {:>10} {:>8} {:>8}",
                    truncate_name(&s.project_name),
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

// ---------------------------------------------------------------------------
// Issues summary
// ---------------------------------------------------------------------------

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
    let ctx = match prepare_report(
        config,
        auth_client,
        account,
        &filter,
        "Issues summary",
        output_format,
    )
    .await?
    {
        Some(ctx) => ctx,
        None => return Ok(()),
    };

    let progress_bar = create_progress_bar(
        output_format,
        ctx.filtered_projects.len() as u64,
        "Fetching issues...",
    );

    let issues_client =
        IssuesClient::new_with_http_config(config.clone(), auth_client.clone(), ctx.http_config);

    let mut summaries = Vec::new();

    for project in &ctx.filtered_projects {
        if let Some(ref pb) = progress_bar {
            pb.set_message(project.name.to_string());
        }

        match issues_client.list_issues(&project.id, None).await {
            Ok(issues) => {
                // Compute status breakdown from the UNFILTERED list
                let open = count_status(&issues, "open");
                let closed = count_status(&issues, "closed");
                let other = issues.len() - open - closed;

                // Total reflects filtered count when a status filter is active
                let total = if let Some(ref sf) = status_filter {
                    issues
                        .iter()
                        .filter(|i| i.status.eq_ignore_ascii_case(sf))
                        .count()
                } else {
                    issues.len()
                };

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
                println!(
                    "{:<30} {:>8} {:>8} {:>8} {:>8}",
                    truncate_name(&s.project_name),
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

// ---------------------------------------------------------------------------
// Submittals summary
// ---------------------------------------------------------------------------

async fn submittals_summary(
    config: &Config,
    auth_client: &AuthClient,
    account: Option<String>,
    filter: Option<String>,
    status_filter: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let ctx = match prepare_report(
        config,
        auth_client,
        account,
        &filter,
        "Submittals summary",
        output_format,
    )
    .await?
    {
        Some(ctx) => ctx,
        None => return Ok(()),
    };

    let progress_bar = create_progress_bar(
        output_format,
        ctx.filtered_projects.len() as u64,
        "Fetching submittals...",
    );

    let acc_client = AccClient::new(config.clone(), auth_client.clone());
    let mut summaries = Vec::new();

    for project in &ctx.filtered_projects {
        if let Some(ref pb) = progress_bar {
            pb.set_message(project.name.to_string());
        }

        match acc_client.list_submittals(&project.id).await {
            Ok(submittals) => {
                let total = if let Some(ref sf) = status_filter {
                    submittals
                        .iter()
                        .filter(|s| s.status.eq_ignore_ascii_case(sf))
                        .count()
                } else {
                    submittals.len()
                };

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

    print_simple_table("Submittals", &output, output_format, |s| s.total)
}

// ---------------------------------------------------------------------------
// Checklists summary
// ---------------------------------------------------------------------------

async fn checklists_summary(
    config: &Config,
    auth_client: &AuthClient,
    account: Option<String>,
    filter: Option<String>,
    status_filter: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let ctx = match prepare_report(
        config,
        auth_client,
        account,
        &filter,
        "Checklists summary",
        output_format,
    )
    .await?
    {
        Some(ctx) => ctx,
        None => return Ok(()),
    };

    let progress_bar = create_progress_bar(
        output_format,
        ctx.filtered_projects.len() as u64,
        "Fetching checklists...",
    );

    let acc_client = AccClient::new(config.clone(), auth_client.clone());
    let mut summaries = Vec::new();

    for project in &ctx.filtered_projects {
        if let Some(ref pb) = progress_bar {
            pb.set_message(project.name.to_string());
        }

        match acc_client.list_checklists(&project.id).await {
            Ok(checklists) => {
                let total = if let Some(ref sf) = status_filter {
                    checklists
                        .iter()
                        .filter(|c| c.status.eq_ignore_ascii_case(sf))
                        .count()
                } else {
                    checklists.len()
                };

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

    print_simple_table("Checklists", &output, output_format, |s| s.total)
}

// ---------------------------------------------------------------------------
// Assets summary
// ---------------------------------------------------------------------------

async fn assets_summary(
    config: &Config,
    auth_client: &AuthClient,
    account: Option<String>,
    filter: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let ctx = match prepare_report(
        config,
        auth_client,
        account,
        &filter,
        "Assets summary",
        output_format,
    )
    .await?
    {
        Some(ctx) => ctx,
        None => return Ok(()),
    };

    let progress_bar = create_progress_bar(
        output_format,
        ctx.filtered_projects.len() as u64,
        "Fetching assets...",
    );

    let acc_client = AccClient::new(config.clone(), auth_client.clone());
    let mut summaries = Vec::new();

    for project in &ctx.filtered_projects {
        if let Some(ref pb) = progress_bar {
            pb.set_message(project.name.to_string());
        }

        match acc_client.list_assets(&project.id).await {
            Ok(assets) => {
                summaries.push(AssetProjectSummary {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                    total: assets.len(),
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

    print_simple_table("Assets", &output, output_format, |s| s.total)
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
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["total"], 0);
        assert_eq!(parsed["open"], 0);
        assert_eq!(parsed["answered"], 0);
        assert_eq!(parsed["closed"], 0);
        assert_eq!(parsed["void"], 0);
    }

    #[test]
    fn test_truncate_name_short() {
        assert_eq!(truncate_name("Short Name"), "Short Name");
    }

    #[test]
    fn test_truncate_name_long() {
        let long = "A Very Long Project Name That Exceeds Limit";
        let result = truncate_name(long);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 28);
    }

    #[test]
    fn test_get_account_id_some() {
        let id = get_account_id(Some("abc-123".to_string())).unwrap();
        assert_eq!(id, "abc-123");
    }

    #[test]
    fn test_get_account_id_empty() {
        let result = get_account_id(Some(String::new()));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_account_id_none() {
        // Without APS_ACCOUNT_ID set, should fail
        unsafe { std::env::remove_var("APS_ACCOUNT_ID") };
        let result = get_account_id(None);
        assert!(result.is_err());
    }
}

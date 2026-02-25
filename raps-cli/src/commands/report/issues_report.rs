// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Issues summary report implementation

use anyhow::Result;
use colored::Colorize;

use raps_acc::IssuesClient;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;

use crate::output::OutputFormat;

use super::{
    IssueProjectSummary, ReportSummaryOutput, count_status, create_progress_bar, prepare_report,
    truncate_name,
};

#[allow(clippy::too_many_arguments)]
pub(super) async fn issues_summary(
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

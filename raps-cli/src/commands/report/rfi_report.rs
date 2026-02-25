// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! RFI summary report implementation

use anyhow::Result;
use colored::Colorize;

use raps_acc::RfiClient;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;

use crate::output::OutputFormat;

use super::{
    ReportSummaryOutput, RfiProjectSummary, count_status, create_progress_bar, prepare_report,
    truncate_name,
};

#[allow(clippy::too_many_arguments)]
pub(super) async fn rfi_summary(
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

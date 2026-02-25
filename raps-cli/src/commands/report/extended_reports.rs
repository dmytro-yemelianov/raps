// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Extended report implementations: submittals, checklists, and assets

use anyhow::Result;

use raps_acc::AccClient;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;

use crate::output::OutputFormat;

use super::{
    AssetProjectSummary, ChecklistProjectSummary, ReportSummaryOutput, SubmittalProjectSummary,
    create_progress_bar, prepare_report, print_simple_table,
};

// ---------------------------------------------------------------------------
// Submittals summary
// ---------------------------------------------------------------------------

pub(super) async fn submittals_summary(
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

pub(super) async fn checklists_summary(
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

pub(super) async fn assets_summary(
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

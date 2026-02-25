// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! RFI (Request for Information) Commands
//!
//! Commands for managing RFIs in ACC projects.

mod crud;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use serde::Serialize;

use crate::commands::interactive;
use crate::output::OutputFormat;
use raps_acc::RfiClient;
use raps_dm::DataManagementClient;

#[derive(Debug, Subcommand)]
pub enum RfiCommands {
    /// List RFIs in a project
    List {
        /// Project ID (without "b." prefix)
        project_id: Option<String>,

        /// Filter by status (open, answered, closed, void)
        #[arg(long)]
        status: Option<String>,

        /// Only show RFIs created after this date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,

        /// Hub ID (for interactive mode)
        #[arg(long, hide = true)]
        hub_id: Option<String>,
    },

    /// Get details of a specific RFI
    Get {
        /// Project ID (without "b." prefix)
        project_id: Option<String>,

        /// RFI ID
        rfi_id: Option<String>,

        /// Hub ID (for interactive mode)
        #[arg(long, hide = true)]
        hub_id: Option<String>,
    },

    /// Create a new RFI
    Create {
        /// Project ID (without "b." prefix)
        project_id: Option<String>,

        /// RFI title
        #[arg(long)]
        title: Option<String>,

        /// RFI question/description
        #[arg(long)]
        question: Option<String>,

        /// Priority (low, normal, high, critical)
        #[arg(long, default_value = "normal")]
        priority: String,

        /// Due date (ISO 8601 format: YYYY-MM-DD)
        #[arg(long)]
        due_date: Option<String>,

        /// User ID to assign to
        #[arg(long)]
        assigned_to: Option<String>,

        /// Location reference
        #[arg(long)]
        location: Option<String>,

        /// Discipline
        #[arg(long)]
        discipline: Option<String>,

        /// Create RFIs from CSV file (columns: title, description, assigned_to)
        #[arg(long, value_name = "FILE")]
        from_csv: Option<PathBuf>,

        /// Hub ID (for interactive mode)
        #[arg(long, hide = true)]
        hub_id: Option<String>,
    },

    /// Update an existing RFI
    Update {
        /// Project ID (without "b." prefix)
        project_id: Option<String>,

        /// RFI ID
        rfi_id: Option<String>,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// Update question
        #[arg(long)]
        question: Option<String>,

        /// Set answer (typically transitions to 'answered' status)
        #[arg(long)]
        answer: Option<String>,

        /// New status (open, answered, closed, void)
        #[arg(long)]
        status: Option<String>,

        /// New priority
        #[arg(long)]
        priority: Option<String>,

        /// New due date
        #[arg(long)]
        due_date: Option<String>,

        /// Reassign to user
        #[arg(long)]
        assigned_to: Option<String>,

        /// Update location
        #[arg(long)]
        location: Option<String>,

        /// Hub ID (for interactive mode)
        #[arg(long, hide = true)]
        hub_id: Option<String>,
    },

    /// Delete an RFI
    Delete {
        /// Project ID (without "b." prefix)
        project_id: Option<String>,

        /// RFI ID
        rfi_id: Option<String>,

        /// Hub ID (for interactive mode)
        #[arg(long, hide = true)]
        hub_id: Option<String>,
    },
}

impl RfiCommands {
    pub async fn execute(
        self,
        client: &RfiClient,
        dm_client: &DataManagementClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            RfiCommands::List {
                project_id,
                status,
                since,
                hub_id,
            } => {
                let (p_id, _) = resolve_rfi_args(
                    dm_client,
                    client,
                    hub_id,
                    project_id,
                    Some("ignore".to_string()),
                )
                .await?;
                crud::list_rfis(client, &p_id, status.as_deref(), since, output_format).await
            }
            RfiCommands::Get {
                project_id,
                rfi_id,
                hub_id,
            } => {
                let (p_id, r_id) =
                    resolve_rfi_args(dm_client, client, hub_id, project_id, rfi_id).await?;
                crud::get_rfi(client, &p_id, &r_id, output_format).await
            }
            RfiCommands::Create {
                project_id,
                title,
                question,
                priority,
                due_date,
                assigned_to,
                location,
                discipline,
                from_csv,
                hub_id,
            } => {
                let (p_id, _) = resolve_rfi_args(
                    dm_client,
                    client,
                    hub_id,
                    project_id,
                    Some("ignore".to_string()),
                )
                .await?;
                crud::create_rfi(
                    client,
                    &p_id,
                    title,
                    question,
                    &priority,
                    due_date,
                    assigned_to,
                    location,
                    discipline,
                    from_csv,
                    output_format,
                )
                .await
            }
            RfiCommands::Update {
                project_id,
                rfi_id,
                title,
                question,
                answer,
                status,
                priority,
                due_date,
                assigned_to,
                location,
                hub_id,
            } => {
                let (p_id, r_id) =
                    resolve_rfi_args(dm_client, client, hub_id, project_id, rfi_id).await?;
                crud::update_rfi(
                    client,
                    &p_id,
                    &r_id,
                    title,
                    question,
                    answer,
                    status,
                    priority,
                    due_date,
                    assigned_to,
                    location,
                    output_format,
                )
                .await
            }
            RfiCommands::Delete {
                project_id,
                rfi_id,
                hub_id,
            } => {
                let (p_id, r_id) =
                    resolve_rfi_args(dm_client, client, hub_id, project_id, rfi_id).await?;
                crud::delete_rfi(client, &p_id, &r_id, output_format).await
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

async fn resolve_rfi_args(
    dm_client: &DataManagementClient,
    rfi_client: &RfiClient,
    opt_hub_id: Option<String>,
    opt_project_id: Option<String>,
    opt_rfi_id: Option<String>,
) -> Result<(String, String)> {
    let hub_id = match (&opt_hub_id, &opt_project_id, &opt_rfi_id) {
        (Some(h), _, _) => h.clone(),
        (None, Some(_), Some(_)) => String::new(), // Not needed if both P and R are provided
        (None, _, _) => interactive::prompt_for_hub(dm_client).await?,
    };

    let project_id = match opt_project_id {
        Some(p) => p,
        None => interactive::prompt_for_project(dm_client, &hub_id).await?,
    };

    let rfi_id = match opt_rfi_id {
        Some(r) if r == "ignore" => String::new(),
        Some(r) => r,
        None => interactive::prompt_for_rfi(rfi_client, &project_id).await?,
    };

    Ok((project_id, rfi_id))
}

#[derive(Serialize)]
pub(super) struct RfiOutput {
    pub(super) id: String,
    pub(super) number: Option<String>,
    pub(super) title: String,
    pub(super) status: String,
    pub(super) priority: Option<String>,
    pub(super) question: Option<String>,
    pub(super) answer: Option<String>,
    pub(super) due_date: Option<String>,
    pub(super) assigned_to_name: Option<String>,
    pub(super) created_at: Option<String>,
}

pub(super) fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

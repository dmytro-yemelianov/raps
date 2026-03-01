// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC Extended Commands
//!
//! Commands for ACC modules: Assets, Submittals, Checklists

mod assets;
mod checklists;
mod submittals;

use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

use crate::output::OutputFormat;
use raps_acc::AccClient;

#[derive(Debug, Subcommand)]
pub enum AccCommands {
    /// Manage project assets
    #[command(subcommand)]
    Asset(AssetCommands),

    /// Manage project submittals
    #[command(subcommand)]
    Submittal(SubmittalCommands),

    /// Manage project checklists
    #[command(subcommand)]
    Checklist(ChecklistCommands),
}

#[derive(Debug, Subcommand)]
pub enum AssetCommands {
    /// List assets in a project
    List {
        /// Project ID (without "b." prefix)
        project_id: String,
    },

    /// Get a specific asset
    Get {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Asset ID
        asset_id: String,
    },

    /// Create a new asset
    Create {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Asset description
        #[arg(long)]
        description: Option<String>,
        /// Barcode
        #[arg(long)]
        barcode: Option<String>,
        /// Category ID
        #[arg(long)]
        category_id: Option<String>,
    },

    /// Update an existing asset
    Update {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Asset ID
        asset_id: String,
        /// New description
        #[arg(long)]
        description: Option<String>,
        /// New barcode
        #[arg(long)]
        barcode: Option<String>,
        /// New status ID
        #[arg(long)]
        status_id: Option<String>,
    },

    /// Delete an asset
    Delete {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Asset ID
        asset_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum SubmittalCommands {
    /// List submittals in a project
    List {
        /// Project ID (without "b." prefix)
        project_id: String,
    },

    /// Get a specific submittal
    Get {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Submittal ID
        submittal_id: String,
    },

    /// Create a new submittal
    Create {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Submittal title
        #[arg(long)]
        title: Option<String>,
        /// Spec section reference
        #[arg(long)]
        spec_section: Option<String>,
        /// Due date (ISO 8601 format)
        #[arg(long)]
        due_date: Option<String>,
        /// Create submittals from CSV file (columns: title, description, spec_id)
        #[arg(long, value_name = "FILE")]
        from_csv: Option<PathBuf>,
    },

    /// Update an existing submittal
    Update {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Submittal ID
        submittal_id: String,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New status
        #[arg(long)]
        status: Option<String>,
        /// New due date
        #[arg(long)]
        due_date: Option<String>,
    },

    /// Delete a submittal
    Delete {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Submittal ID
        submittal_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ChecklistCommands {
    /// List checklists in a project
    List {
        /// Project ID (without "b." prefix)
        project_id: String,
    },

    /// Get a specific checklist
    Get {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Checklist ID
        checklist_id: String,
    },

    /// Create a new checklist
    Create {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Checklist title
        #[arg(long)]
        title: String,
        /// Template ID to use
        #[arg(long)]
        template_id: Option<String>,
        /// Location reference
        #[arg(long)]
        location: Option<String>,
        /// Due date (ISO 8601 format)
        #[arg(long)]
        due_date: Option<String>,
        /// Assignee user ID
        #[arg(long)]
        assignee_id: Option<String>,
    },

    /// Update an existing checklist
    Update {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Checklist ID
        checklist_id: String,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New status
        #[arg(long)]
        status: Option<String>,
        /// New location
        #[arg(long)]
        location: Option<String>,
        /// New due date
        #[arg(long)]
        due_date: Option<String>,
    },

    /// Delete a checklist
    Delete {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Checklist ID
        checklist_id: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// List checklist templates
    Templates {
        /// Project ID (without "b." prefix)
        project_id: String,
    },
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

pub(super) fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

impl AccCommands {
    pub async fn execute(self, client: &AccClient, output_format: OutputFormat) -> Result<()> {
        match self {
            AccCommands::Asset(cmd) => cmd.execute(client, output_format).await,
            AccCommands::Submittal(cmd) => cmd.execute(client, output_format).await,
            AccCommands::Checklist(cmd) => cmd.execute(client, output_format).await,
        }
    }
}

impl AssetCommands {
    pub async fn execute(self, client: &AccClient, output_format: OutputFormat) -> Result<()> {
        match self {
            AssetCommands::List { project_id } => {
                assets::list_assets(client, &project_id, output_format).await
            }
            AssetCommands::Get {
                project_id,
                asset_id,
            } => assets::get_asset(client, &project_id, &asset_id, output_format).await,
            AssetCommands::Create {
                project_id,
                description,
                barcode,
                category_id,
            } => {
                assets::create_asset(
                    client,
                    &project_id,
                    description,
                    barcode,
                    category_id,
                    output_format,
                )
                .await
            }
            AssetCommands::Update {
                project_id,
                asset_id,
                description,
                barcode,
                status_id,
            } => {
                assets::update_asset(
                    client,
                    &project_id,
                    &asset_id,
                    description,
                    barcode,
                    status_id,
                    output_format,
                )
                .await
            }
            AssetCommands::Delete {
                project_id,
                asset_id,
            } => assets::delete_asset(client, &project_id, &asset_id, output_format).await,
        }
    }
}

impl SubmittalCommands {
    pub async fn execute(self, client: &AccClient, output_format: OutputFormat) -> Result<()> {
        match self {
            SubmittalCommands::List { project_id } => {
                submittals::list_submittals(client, &project_id, output_format).await
            }
            SubmittalCommands::Get {
                project_id,
                submittal_id,
            } => submittals::get_submittal(client, &project_id, &submittal_id, output_format).await,
            SubmittalCommands::Create {
                project_id,
                title,
                spec_section,
                due_date,
                from_csv,
            } => {
                submittals::create_submittal(
                    client,
                    &project_id,
                    title,
                    spec_section,
                    due_date,
                    from_csv,
                    output_format,
                )
                .await
            }
            SubmittalCommands::Update {
                project_id,
                submittal_id,
                title,
                status,
                due_date,
            } => {
                submittals::update_submittal(
                    client,
                    &project_id,
                    &submittal_id,
                    title,
                    status,
                    due_date,
                    output_format,
                )
                .await
            }
            SubmittalCommands::Delete {
                project_id,
                submittal_id,
            } => {
                submittals::delete_submittal(client, &project_id, &submittal_id, output_format)
                    .await
            }
        }
    }
}

impl ChecklistCommands {
    pub async fn execute(self, client: &AccClient, output_format: OutputFormat) -> Result<()> {
        match self {
            ChecklistCommands::List { project_id } => {
                checklists::list_checklists(client, &project_id, output_format).await
            }
            ChecklistCommands::Get {
                project_id,
                checklist_id,
            } => checklists::get_checklist(client, &project_id, &checklist_id, output_format).await,
            ChecklistCommands::Create {
                project_id,
                title,
                template_id,
                location,
                due_date,
                assignee_id,
            } => {
                checklists::create_checklist(
                    client,
                    &project_id,
                    &title,
                    template_id,
                    location,
                    due_date,
                    assignee_id,
                    output_format,
                )
                .await
            }
            ChecklistCommands::Update {
                project_id,
                checklist_id,
                title,
                status,
                location,
                due_date,
            } => {
                checklists::update_checklist(
                    client,
                    &project_id,
                    &checklist_id,
                    title,
                    status,
                    location,
                    due_date,
                    output_format,
                )
                .await
            }
            ChecklistCommands::Delete {
                project_id,
                checklist_id,
                yes,
            } => {
                checklists::delete_checklist(
                    client,
                    &project_id,
                    &checklist_id,
                    yes,
                    output_format,
                )
                .await
            }
            ChecklistCommands::Templates { project_id } => {
                checklists::list_templates(client, &project_id, output_format).await
            }
        }
    }
}

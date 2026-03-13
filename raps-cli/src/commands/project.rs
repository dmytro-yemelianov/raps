// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Project management commands
//!
//! Commands for listing and viewing projects (requires 3-legged auth).

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use crate::commands::interactive;
use crate::commands::tracked::tracked_op;
use crate::output::OutputFormat;
use raps_acc::admin::AccountAdminClient;
use raps_dm::DataManagementClient;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::security::validate_resource_id;
// use raps_kernel::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum ProjectCommands {
    /// List projects in a hub
    List {
        /// Hub ID (interactive if not provided)
        hub_id: Option<String>,
    },

    /// Get project details
    Info {
        /// Hub ID (interactive if not provided)
        hub_id: Option<String>,
        /// Project ID (interactive if not provided)
        project_id: Option<String>,
    },
}

impl ProjectCommands {
    pub async fn execute(
        self,
        client: &DataManagementClient,
        config: &Config,
        auth_client: &AuthClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            ProjectCommands::List { hub_id } => {
                list_projects(client, config, auth_client, hub_id, output_format).await
            }
            ProjectCommands::Info { hub_id, project_id } => {
                project_info(client, &hub_id, &project_id, output_format).await
            }
        }
    }
}

#[derive(Serialize, schemars::JsonSchema)]
pub struct ProjectListOutput {
    pub id: String,
    pub name: String,
    pub status: Option<String>,
    pub platform: Option<String>,
    pub project_type: Option<String>,
    pub scopes: Option<Vec<String>>,
}

/// Returns Some(account_id) if hub_id is a BIM 360 Business hub (prefix "b.")
fn bim360_account_id(hub_id: &str) -> Option<String> {
    hub_id.strip_prefix("b.").map(|s| s.to_string())
}

async fn list_projects(
    client: &DataManagementClient,
    config: &Config,
    auth_client: &AuthClient,
    hub_id: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    // Get hub ID interactively if not provided
    let hub = match hub_id {
        Some(h) => {
            validate_resource_id(&h).with_context(|| format!("Invalid hub ID: {:?}", h))?;
            h
        }
        None => interactive::prompt_for_hub(client).await?,
    };

    // BIM 360 hub → use Admin API
    if let Some(account_id) = bim360_account_id(&hub) {
        return list_projects_admin(
            config.clone(),
            auth_client.clone(),
            &account_id,
            output_format,
        )
        .await;
    }

    let projects = tracked_op("Fetching projects", output_format, || async {
        client.list_projects(&hub).await.context(format!(
            "Failed to list projects in hub '{}'. Verify the hub ID and your permissions",
            hub
        ))
    })
    .await?;

    let project_outputs: Vec<ProjectListOutput> = projects
        .iter()
        .map(|p| ProjectListOutput {
            id: p.id.clone(),
            name: p.attributes.name.clone(),
            status: None,
            platform: None,
            project_type: Some(p.project_type.clone()),
            scopes: p.attributes.scopes.clone(),
        })
        .collect();

    if project_outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No projects found in this hub.".yellow()),
            _ => {
                output_format.write(&Vec::<ProjectListOutput>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Projects:".bold());
            println!("{}", "-".repeat(80));

            for project in &project_outputs {
                println!("  {} {}", "-".cyan(), project.name.bold());
                println!("    {} {}", "ID:".dimmed(), project.id);
                if let Some(ref scopes) = project.scopes {
                    println!("    {} {:?}", "Scopes:".dimmed(), scopes);
                }
            }

            println!("{}", "-".repeat(80));
            println!(
                "\n{}",
                "Use 'raps folder list <hub-id> <project-id>' to see folders".dimmed()
            );
        }
        _ => {
            output_format.write(&project_outputs)?;
        }
    }
    Ok(())
}

async fn list_projects_admin(
    config: Config,
    auth_client: AuthClient,
    account_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    let admin_client = AccountAdminClient::new(config, auth_client);

    let projects = tracked_op("Fetching projects", output_format, || async {
        admin_client
            .list_all_projects(account_id)
            .await
            .context("Failed to list BIM 360 projects")
    })
    .await?;

    // This path is only reached for BIM 360 hubs (gated by bim360_account_id),
    // but the ACC admin API returns platform:"acc" for all projects regardless.
    // Override to "bim360" since we know the hub type.
    let outputs: Vec<ProjectListOutput> = projects
        .iter()
        .map(|p| ProjectListOutput {
            id: p.id.clone(),
            name: p.name.clone(),
            status: p.status.clone(),
            platform: Some("bim360".to_string()),
            project_type: None,
            scopes: None,
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No projects found.".yellow()),
            _ => output_format.write(&Vec::<ProjectListOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Projects:".bold());
            println!("{}", "-".repeat(80));
            for p in &outputs {
                println!("  {} {}", "-".cyan(), p.name.bold());
                println!("    {} {}", "ID:".dimmed(), p.id);
                if let Some(ref status) = p.status {
                    println!("    {} {}", "Status:".dimmed(), status);
                }
                if let Some(ref platform) = p.platform {
                    println!("    {} {}", "Platform:".dimmed(), platform);
                }
            }
            println!("{}", "-".repeat(80));
        }
        _ => output_format.write(&outputs)?,
    }
    Ok(())
}

#[derive(Serialize, schemars::JsonSchema)]
struct ProjectInfoOutput {
    id: String,
    name: String,
    project_type: String,
    scopes: Option<Vec<String>>,
    top_folders: Vec<FolderOutput>,
}

#[derive(Serialize, schemars::JsonSchema)]
struct FolderOutput {
    id: String,
    name: String,
    display_name: Option<String>,
}

async fn project_info(
    client: &DataManagementClient,
    opt_hub_id: &Option<String>,
    opt_project_id: &Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let hub_id = match opt_hub_id {
        Some(h) => {
            validate_resource_id(h).with_context(|| format!("Invalid hub ID: {:?}", h))?;
            h.clone()
        }
        None => interactive::prompt_for_hub(client).await?,
    };

    let project_id = match opt_project_id {
        Some(p) => {
            validate_resource_id(p).with_context(|| format!("Invalid project ID: {:?}", p))?;
            p.clone()
        }
        None => interactive::prompt_for_project(client, &hub_id).await?,
    };
    let (project, folders) = tracked_op("Fetching project details", output_format, || async {
        let project = client
            .get_project(&hub_id, &project_id)
            .await
            .context(format!(
                "Failed to get project '{}'. Verify the project ID and your permissions",
                project_id
            ))?;
        let folders = client
            .get_top_folders(&hub_id, &project_id)
            .await
            .context(format!(
                "Failed to get top folders for project '{}'. You may lack folder-level permissions",
                project_id
            ))?;
        Ok((project, folders))
    })
    .await?;

    let folder_outputs: Vec<FolderOutput> = folders
        .iter()
        .map(|f| FolderOutput {
            id: f.id.clone(),
            name: f.attributes.name.clone(),
            display_name: f.attributes.display_name.clone(),
        })
        .collect();

    let output = ProjectInfoOutput {
        id: project.id.clone(),
        name: project.attributes.name.clone(),
        project_type: project.project_type.clone(),
        scopes: project.attributes.scopes.clone(),
        top_folders: folder_outputs,
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Project Details".bold());
            println!("{}", "-".repeat(60));
            println!("  {} {}", "Name:".bold(), output.name.cyan());
            println!("  {} {}", "ID:".bold(), output.id);
            println!("  {} {}", "Type:".bold(), output.project_type);

            if let Some(ref scopes) = output.scopes {
                println!("  {} {:?}", "Scopes:".bold(), scopes);
            }

            println!("\n{}", "Top Folders:".bold());
            for folder in &output.top_folders {
                println!(
                    "  {} {} ({})",
                    "[folder]".dimmed(),
                    folder.display_name.as_ref().unwrap_or(&folder.name),
                    folder.id.dimmed()
                );
            }

            println!("{}", "-".repeat(60));
        }
        _ => {
            output_format.write(&output)?;
        }
    }
    Ok(())
}

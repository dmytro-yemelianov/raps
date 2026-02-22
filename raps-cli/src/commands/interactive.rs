// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Shared interactive dialoguer prompt wrappers for CLI dropdowns.

use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::Select;
use raps_acc::RfiClient;
use raps_dm::DataManagementClient;
pub use raps_kernel::interactive::is_non_interactive;

pub async fn prompt_for_hub(client: &DataManagementClient) -> Result<String> {
    if is_non_interactive() {
        anyhow::bail!(
            "Hub ID is required in non-interactive mode. Please provide it as an argument."
        );
    }

    println!("{}", "Fetching hubs...".dimmed());
    let hubs = client.list_hubs().await.context(
        "Failed to list hubs. This requires 3-legged auth \u{2014} run 'raps auth login' first",
    )?;

    if hubs.is_empty() {
        anyhow::bail!("No hubs found. Make sure you're logged in with 3-legged auth.");
    }

    let hub_names: Vec<String> = hubs
        .iter()
        .map(|h| format!("{} ({})", h.attributes.name, h.id))
        .collect();

    let selection = Select::new()
        .with_prompt("Select a Hub")
        .items(&hub_names)
        .interact()?;

    Ok(hubs[selection].id.clone())
}

pub async fn prompt_for_project(client: &DataManagementClient, hub_id: &str) -> Result<String> {
    if is_non_interactive() {
        anyhow::bail!(
            "Project ID is required in non-interactive mode. Please provide it as an argument."
        );
    }

    println!("{}", "Fetching projects...".dimmed());
    let projects = client
        .list_projects(hub_id)
        .await
        .context(format!("Failed to list projects in hub '{}'", hub_id))?;

    if projects.is_empty() {
        anyhow::bail!("No projects found in this hub.");
    }

    let project_names: Vec<String> = projects
        .iter()
        .map(|p| format!("{} ({})", p.attributes.name, p.id))
        .collect();

    let selection = Select::new()
        .with_prompt("Select a Project")
        .items(&project_names)
        .interact()?;

    Ok(projects[selection].id.clone())
}

pub async fn prompt_for_folder(
    client: &DataManagementClient,
    hub_id: &str,
    project_id: &str,
) -> Result<String> {
    if is_non_interactive() {
        anyhow::bail!(
            "Folder ID is required in non-interactive mode. Please provide it as an argument."
        );
    }

    println!("{}", "Fetching top folders...".dimmed());
    let folders = client
        .get_top_folders(hub_id, project_id)
        .await
        .context(format!(
            "Failed to get top folders for project '{}'",
            project_id
        ))?;

    if folders.is_empty() {
        anyhow::bail!("No folders found in this project.");
    }

    let folder_names: Vec<String> = folders
        .iter()
        .map(|f| {
            let name = f
                .attributes
                .display_name
                .as_deref()
                .unwrap_or(f.attributes.name.as_str());
            format!("{} ({})", name, f.id)
        })
        .collect();

    let selection = Select::new()
        .with_prompt("Select a Folder")
        .items(&folder_names)
        .interact()?;

    Ok(folders[selection].id.clone())
}

pub async fn prompt_for_rfi(client: &RfiClient, project_id: &str) -> Result<String> {
    if is_non_interactive() {
        anyhow::bail!(
            "RFI ID is required in non-interactive mode. Please provide it as an argument."
        );
    }

    println!("{}", "Fetching RFIs...".dimmed());
    let rfis = client
        .list_rfis(project_id)
        .await
        .context(format!("Failed to list RFIs for project '{}'", project_id))?;

    if rfis.is_empty() {
        anyhow::bail!("No RFIs found in this project.");
    }

    let rfi_names: Vec<String> = rfis
        .iter()
        .map(|r| {
            let num = r.number.as_deref().unwrap_or("-");
            format!("[{}] {} ({}) - {}", num, r.title, r.id, r.status)
        })
        .collect();

    let selection = Select::new()
        .with_prompt("Select an RFI")
        .items(&rfi_names)
        .interact()?;

    Ok(rfis[selection].id.clone())
}

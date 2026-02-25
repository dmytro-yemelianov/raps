// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Shared interactive dialoguer prompt wrappers for CLI dropdowns.

use anyhow::{Context, Result};
use dialoguer::Select;
use raps_acc::RfiClient;
use raps_dm::DataManagementClient;
pub use raps_kernel::interactive::is_non_interactive;
use raps_kernel::{api_health, progress};

pub async fn prompt_for_hub(client: &DataManagementClient) -> Result<String> {
    if is_non_interactive() {
        anyhow::bail!(
            "Hub ID is required in non-interactive mode. Please provide it as an argument."
        );
    }

    let spinner = progress::spinner("Fetching hubs...");
    let start = std::time::Instant::now();
    let hubs = client.list_hubs().await.context(
        "Failed to list hubs. This requires 3-legged auth \u{2014} run 'raps auth login' first",
    );
    let elapsed = start.elapsed();
    let snap = api_health::snapshot();
    let suffix = if snap.sample_count > 0 {
        format!(
            " ({}, avg: {}, API: {})",
            api_health::format_duration_ms(elapsed),
            api_health::format_duration_ms(snap.avg_latency),
            snap.health_status,
        )
    } else {
        format!(" ({})", api_health::format_duration_ms(elapsed))
    };
    match &hubs {
        Ok(_) => spinner.finish_with_message(format!("\u{2713} Fetching hubs{}", suffix)),
        Err(_) => spinner.finish_with_message(format!("\u{2717} Fetching hubs (after {})", api_health::format_duration_ms(elapsed))),
    }
    let hubs = hubs?;

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

    let spinner = progress::spinner("Fetching projects...");
    let start = std::time::Instant::now();
    let projects = client
        .list_projects(hub_id)
        .await
        .context(format!("Failed to list projects in hub '{}'", hub_id));
    let elapsed = start.elapsed();
    let snap = api_health::snapshot();
    let suffix = if snap.sample_count > 0 {
        format!(
            " ({}, avg: {}, API: {})",
            api_health::format_duration_ms(elapsed),
            api_health::format_duration_ms(snap.avg_latency),
            snap.health_status,
        )
    } else {
        format!(" ({})", api_health::format_duration_ms(elapsed))
    };
    match &projects {
        Ok(_) => spinner.finish_with_message(format!("\u{2713} Fetching projects{}", suffix)),
        Err(_) => spinner.finish_with_message(format!("\u{2717} Fetching projects (after {})", api_health::format_duration_ms(elapsed))),
    }
    let projects = projects?;

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

    let spinner = progress::spinner("Fetching top folders...");
    let start = std::time::Instant::now();
    let folders = client
        .get_top_folders(hub_id, project_id)
        .await
        .context(format!(
            "Failed to get top folders for project '{}'",
            project_id
        ));
    let elapsed = start.elapsed();
    let snap = api_health::snapshot();
    let suffix = if snap.sample_count > 0 {
        format!(
            " ({}, avg: {}, API: {})",
            api_health::format_duration_ms(elapsed),
            api_health::format_duration_ms(snap.avg_latency),
            snap.health_status,
        )
    } else {
        format!(" ({})", api_health::format_duration_ms(elapsed))
    };
    match &folders {
        Ok(_) => spinner.finish_with_message(format!("\u{2713} Fetching top folders{}", suffix)),
        Err(_) => spinner.finish_with_message(format!("\u{2717} Fetching top folders (after {})", api_health::format_duration_ms(elapsed))),
    }
    let folders = folders?;

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

    let spinner = progress::spinner("Fetching RFIs...");
    let start = std::time::Instant::now();
    let rfis = client
        .list_rfis(project_id)
        .await
        .context(format!("Failed to list RFIs for project '{}'", project_id));
    let elapsed = start.elapsed();
    let snap = api_health::snapshot();
    let suffix = if snap.sample_count > 0 {
        format!(
            " ({}, avg: {}, API: {})",
            api_health::format_duration_ms(elapsed),
            api_health::format_duration_ms(snap.avg_latency),
            snap.health_status,
        )
    } else {
        format!(" ({})", api_health::format_duration_ms(elapsed))
    };
    match &rfis {
        Ok(_) => spinner.finish_with_message(format!("\u{2713} Fetching RFIs{}", suffix)),
        Err(_) => spinner.finish_with_message(format!("\u{2717} Fetching RFIs (after {})", api_health::format_duration_ms(elapsed))),
    }
    let rfis = rfis?;

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

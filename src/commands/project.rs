//! Project management commands
//!
//! Commands for listing and viewing projects (requires 3-legged auth).

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::Select;

use crate::api::DataManagementClient;

#[derive(Debug, Subcommand)]
pub enum ProjectCommands {
    /// List projects in a hub
    List {
        /// Hub ID (interactive if not provided)
        hub_id: Option<String>,
    },

    /// Get project details
    Info {
        /// Hub ID
        hub_id: String,
        /// Project ID
        project_id: String,
    },
}

impl ProjectCommands {
    pub async fn execute(self, client: &DataManagementClient) -> Result<()> {
        match self {
            ProjectCommands::List { hub_id } => list_projects(client, hub_id).await,
            ProjectCommands::Info { hub_id, project_id } => {
                project_info(client, &hub_id, &project_id).await
            }
        }
    }
}

async fn list_projects(client: &DataManagementClient, hub_id: Option<String>) -> Result<()> {
    // Get hub ID interactively if not provided
    let hub = match hub_id {
        Some(h) => h,
        None => {
            println!("{}", "Fetching hubs...".dimmed());
            let hubs = client.list_hubs().await?;

            if hubs.is_empty() {
                anyhow::bail!("No hubs found. Make sure you're logged in with 3-legged auth.");
            }

            let hub_names: Vec<String> = hubs
                .iter()
                .map(|h| format!("{} ({})", h.attributes.name, h.id))
                .collect();

            let selection = Select::new()
                .with_prompt("Select a hub")
                .items(&hub_names)
                .interact()?;

            hubs[selection].id.clone()
        }
    };

    println!("{}", "Fetching projects...".dimmed());

    let projects = client.list_projects(&hub).await?;

    if projects.is_empty() {
        println!("{}", "No projects found in this hub.".yellow());
        return Ok(());
    }

    println!("\n{}", "Projects:".bold());
    println!("{}", "─".repeat(80));

    for project in projects {
        println!("  {} {}", "•".cyan(), project.attributes.name.bold());
        println!("    {} {}", "ID:".dimmed(), project.id);
        if let Some(ref scopes) = project.attributes.scopes {
            println!("    {} {:?}", "Scopes:".dimmed(), scopes);
        }
    }

    println!("{}", "─".repeat(80));
    println!(
        "\n{}",
        "Use 'raps folder list <hub-id> <project-id>' to see folders".dimmed()
    );
    Ok(())
}

async fn project_info(client: &DataManagementClient, hub_id: &str, project_id: &str) -> Result<()> {
    println!("{}", "Fetching project details...".dimmed());

    let project = client.get_project(hub_id, project_id).await?;

    println!("\n{}", "Project Details".bold());
    println!("{}", "─".repeat(60));
    println!("  {} {}", "Name:".bold(), project.attributes.name.cyan());
    println!("  {} {}", "ID:".bold(), project.id);
    println!("  {} {}", "Type:".bold(), project.project_type);

    if let Some(ref scopes) = project.attributes.scopes {
        println!("  {} {:?}", "Scopes:".bold(), scopes);
    }

    // Get top folders
    println!("\n{}", "Top Folders:".bold());
    let folders = client.get_top_folders(hub_id, project_id).await?;

    for folder in folders {
        println!(
            "  {} {} ({})",
            "📁".dimmed(),
            folder
                .attributes
                .display_name
                .as_ref()
                .unwrap_or(&folder.attributes.name),
            folder.id.dimmed()
        );
    }

    println!("{}", "─".repeat(60));
    Ok(())
}

//! Issue management commands
//! 
//! Commands for managing ACC (Autodesk Construction Cloud) issues.
//! Uses the Construction Issues API: /construction/issues/v1

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Input, Select};

use crate::api::IssuesClient;
use crate::api::issues::CreateIssueRequest;

#[derive(Debug, Subcommand)]
pub enum IssueCommands {
    /// List issues in a project
    List {
        /// Project ID (without "b." prefix used by Data Management API)
        project_id: String,
        
        /// Filter by status (open, closed, etc.)
        #[arg(short, long)]
        status: Option<String>,
    },
    
    /// Create a new issue
    Create {
        /// Project ID (without "b." prefix)
        project_id: String,
        
        /// Issue title
        #[arg(short, long)]
        title: Option<String>,
        
        /// Issue description
        #[arg(short, long)]
        description: Option<String>,
    },
    
    /// Update an issue
    Update {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,
        
        /// New status
        #[arg(short, long)]
        status: Option<String>,
        
        /// New title
        #[arg(short, long)]
        title: Option<String>,
    },
    
    /// List issue types (categories) for a project
    Types {
        /// Project ID (without "b." prefix)
        project_id: String,
    },
}

impl IssueCommands {
    pub async fn execute(self, client: &IssuesClient) -> Result<()> {
        match self {
            IssueCommands::List { project_id, status } => {
                list_issues(client, &project_id, status).await
            }
            IssueCommands::Create { project_id, title, description } => {
                create_issue(client, &project_id, title, description).await
            }
            IssueCommands::Update { project_id, issue_id, status, title } => {
                update_issue(client, &project_id, &issue_id, status, title).await
            }
            IssueCommands::Types { project_id } => {
                list_issue_types(client, &project_id).await
            }
        }
    }
}

async fn list_issues(client: &IssuesClient, project_id: &str, status: Option<String>) -> Result<()> {
    println!("{}", "Fetching issues...".dimmed());

    let filter = status.as_ref().map(|s| format!("status={}", s));
    let issues = client.list_issues(project_id, filter.as_deref()).await?;

    if issues.is_empty() {
        println!("{}", "No issues found.".yellow());
        return Ok(());
    }

    println!("\n{}", "Issues:".bold());
    println!("{}", "─".repeat(90));
    println!(
        "{:<8} {:<12} {:<40} {}",
        "ID".bold(),
        "Status".bold(),
        "Title".bold(),
        "Assigned To".bold()
    );
    println!("{}", "─".repeat(90));

    for issue in issues {
        let display_id = issue.display_id
            .map(|n| format!("#{}", n))
            .unwrap_or_else(|| "-".to_string());
        
        let status_colored = match issue.status.as_str() {
            "open" => issue.status.yellow(),
            "closed" => issue.status.green(),
            "answered" => issue.status.cyan(),
            _ => issue.status.normal(),
        };
        
        let assigned = issue.assigned_to.as_deref().unwrap_or("-");
        
        println!(
            "{:<8} {:<12} {:<40} {}",
            display_id.cyan(),
            status_colored,
            truncate_str(&issue.title, 40),
            assigned.dimmed()
        );
    }

    println!("{}", "─".repeat(90));
    Ok(())
}

async fn create_issue(
    client: &IssuesClient,
    project_id: &str,
    title: Option<String>,
    description: Option<String>,
) -> Result<()> {
    // Get title
    let issue_title = match title {
        Some(t) => t,
        None => {
            Input::new()
                .with_prompt("Enter issue title")
                .interact_text()?
        }
    };

    // Get description (optional)
    let issue_desc = match description {
        Some(d) => Some(d),
        None => {
            let desc: String = Input::new()
                .with_prompt("Enter description (optional)")
                .allow_empty(true)
                .interact_text()?;
            if desc.is_empty() { None } else { Some(desc) }
        }
    };

    println!("{}", "Creating issue...".dimmed());

    let request = CreateIssueRequest {
        title: issue_title,
        description: issue_desc,
        status: "open".to_string(),
        issue_type_id: None,
        issue_subtype_id: None,
        assigned_to: None,
        assigned_to_type: None,
        due_date: None,
    };

    let issue = client.create_issue(project_id, request).await?;

    println!("{} Issue created!", "✓".green().bold());
    println!("  {} {}", "ID:".bold(), issue.id);
    println!("  {} {}", "Title:".bold(), issue.title.cyan());
    println!("  {} {}", "Status:".bold(), issue.status);

    Ok(())
}

async fn update_issue(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    status: Option<String>,
    title: Option<String>,
) -> Result<()> {
    // Get current issue
    let current = client.get_issue(project_id, issue_id).await?;
    
    // Determine new status
    let new_status = match status {
        Some(s) => Some(s),
        None if title.is_none() => {
            // Prompt for status if no updates provided
            let statuses = vec!["open", "answered", "closed"];
            let selection = Select::new()
                .with_prompt("Select new status")
                .items(&statuses)
                .default(0)
                .interact()?;
            Some(statuses[selection].to_string())
        }
        None => None,
    };

    println!("{}", "Updating issue...".dimmed());

    let request = crate::api::issues::UpdateIssueRequest {
        title,
        description: None,
        status: new_status.clone(),
        assigned_to: None,
        due_date: None,
    };

    let issue = client.update_issue(project_id, issue_id, request).await?;

    println!("{} Issue updated!", "✓".green().bold());
    println!("  {} {}", "Title:".bold(), issue.title);
    println!("  {} {} → {}", "Status:".bold(), current.status.dimmed(), issue.status.cyan());

    Ok(())
}

async fn list_issue_types(client: &IssuesClient, project_id: &str) -> Result<()> {
    println!("{}", "Fetching issue types...".dimmed());

    let types = client.list_issue_types(project_id).await?;

    if types.is_empty() {
        println!("{}", "No issue types found.".yellow());
        return Ok(());
    }

    println!("\n{}", "Issue Types (Categories):".bold());
    println!("{}", "─".repeat(60));

    for issue_type in types {
        let active = if issue_type.is_active.unwrap_or(true) {
            "".to_string()
        } else {
            " (inactive)".dimmed().to_string()
        };
        
        println!("  {} {}{}", "•".cyan(), issue_type.title.bold(), active);
        println!("    {} {}", "ID:".dimmed(), issue_type.id);
        
        if let Some(ref subtypes) = issue_type.subtypes {
            for subtype in subtypes {
                let sub_active = if subtype.is_active.unwrap_or(true) {
                    "".to_string()
                } else {
                    " (inactive)".dimmed().to_string()
                };
                println!("    {} {}{}", "└".dimmed(), subtype.title, sub_active);
            }
        }
    }

    println!("{}", "─".repeat(60));
    Ok(())
}

/// Truncate string with ellipsis
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

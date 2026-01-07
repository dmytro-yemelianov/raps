// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Issue management commands
//!
//! Commands for managing ACC (Autodesk Construction Cloud) issues.
//! Uses the Construction Issues API: /construction/issues/v1

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Input, Select};
use serde::Serialize;

use crate::api::IssuesClient;
use crate::api::issues::CreateIssueRequest;
use crate::interactive;
use crate::output::OutputFormat;

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

    /// Manage issue comments
    #[command(subcommand)]
    Comment(CommentCommands),

    /// List attachments for an issue
    Attachments {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,
    },

    /// Transition an issue to a new status
    Transition {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,
        /// Target status (open, answered, closed, etc.)
        #[arg(short, long)]
        to: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum CommentCommands {
    /// List comments on an issue
    List {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,
    },

    /// Add a comment to an issue
    Add {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,
        /// Comment body
        #[arg(short, long)]
        body: String,
    },

    /// Delete a comment from an issue
    Delete {
        /// Project ID (without "b." prefix)
        project_id: String,
        /// Issue ID
        issue_id: String,
        /// Comment ID to delete
        comment_id: String,
    },
}

impl IssueCommands {
    pub async fn execute(self, client: &IssuesClient, output_format: OutputFormat) -> Result<()> {
        match self {
            IssueCommands::List { project_id, status } => {
                list_issues(client, &project_id, status, output_format).await
            }
            IssueCommands::Create {
                project_id,
                title,
                description,
            } => create_issue(client, &project_id, title, description, output_format).await,
            IssueCommands::Update {
                project_id,
                issue_id,
                status,
                title,
            } => update_issue(client, &project_id, &issue_id, status, title, output_format).await,
            IssueCommands::Types { project_id } => {
                list_issue_types(client, &project_id, output_format).await
            }
            IssueCommands::Comment(cmd) => cmd.execute(client, output_format).await,
            IssueCommands::Attachments {
                project_id,
                issue_id,
            } => list_attachments(client, &project_id, &issue_id, output_format).await,
            IssueCommands::Transition {
                project_id,
                issue_id,
                to,
            } => transition_issue(client, &project_id, &issue_id, to, output_format).await,
        }
    }
}

impl CommentCommands {
    pub async fn execute(self, client: &IssuesClient, output_format: OutputFormat) -> Result<()> {
        match self {
            CommentCommands::List {
                project_id,
                issue_id,
            } => list_comments(client, &project_id, &issue_id, output_format).await,
            CommentCommands::Add {
                project_id,
                issue_id,
                body,
            } => add_comment(client, &project_id, &issue_id, &body, output_format).await,
            CommentCommands::Delete {
                project_id,
                issue_id,
                comment_id,
            } => delete_comment(client, &project_id, &issue_id, &comment_id, output_format).await,
        }
    }
}

async fn list_issues(
    client: &IssuesClient,
    project_id: &str,
    status: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching issues...".dimmed());
    }

    let filter = status.as_ref().map(|s| format!("status={}", s));
    let issues = client.list_issues(project_id, filter.as_deref()).await?;

    if issues.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No issues found.".yellow()),
            OutputFormat::Json => println!("[]"),
            OutputFormat::Yaml => println!("[]"),
            OutputFormat::Csv => {
                println!("id,display_id,title,status,assigned_to,created_at,updated_at")
            }
            OutputFormat::Plain => println!("No issues found"),
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
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

            for issue in &issues {
                let display_id = issue
                    .display_id
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
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&issues)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&issues)?);
        }
        OutputFormat::Csv => {
            println!("id,display_id,title,status,assigned_to,created_at,updated_at");
            for issue in &issues {
                let display_id = issue
                    .display_id
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "".to_string());

                let assigned = issue.assigned_to.as_deref().unwrap_or("");

                // Properly escape CSV fields that might contain commas or quotes
                let title = format!("\"{}\"", issue.title.replace("\"", "\"\""));
                let assigned = format!("\"{}\"", assigned.replace("\"", "\"\""));

                println!(
                    "{},{},{},{},{},{},{}",
                    issue.id,
                    display_id,
                    title,
                    issue.status,
                    assigned,
                    issue.created_at.clone().unwrap_or_default(),
                    issue.updated_at.clone().unwrap_or_default()
                );
            }
        }
        OutputFormat::Plain => {
            for issue in &issues {
                let display_id = issue
                    .display_id
                    .map(|n| format!("#{}", n))
                    .unwrap_or_else(|| "-".to_string());

                let assigned = issue.assigned_to.as_deref().unwrap_or("-");

                println!(
                    "{} {} {} {}",
                    display_id, issue.status, issue.title, assigned
                );
            }
        }
    }

    Ok(())
}

async fn create_issue(
    client: &IssuesClient,
    project_id: &str,
    title: Option<String>,
    description: Option<String>,
    _output_format: OutputFormat,
) -> Result<()> {
    // Get title
    let issue_title = match title {
        Some(t) => t,
        None => {
            // In non-interactive mode, require the title
            if interactive::is_non_interactive() {
                anyhow::bail!("Issue title is required in non-interactive mode. Use --title flag.");
            }
            Input::new()
                .with_prompt("Enter issue title")
                .interact_text()?
        }
    };

    // Get description (optional)
    let issue_desc = match description {
        Some(d) => Some(d),
        None => {
            // In non-interactive mode, description is optional (None)
            if interactive::is_non_interactive() {
                None
            } else {
                let desc: String = Input::new()
                    .with_prompt("Enter description (optional)")
                    .allow_empty(true)
                    .interact_text()?;
                if desc.is_empty() { None } else { Some(desc) }
            }
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
    _output_format: OutputFormat,
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
    println!(
        "  {} {} → {}",
        "Status:".bold(),
        current.status.dimmed(),
        issue.status.cyan()
    );

    Ok(())
}

async fn list_issue_types(
    client: &IssuesClient,
    project_id: &str,
    _output_format: OutputFormat,
) -> Result<()> {
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

// ============== COMMENTS ==============

#[derive(Serialize)]
struct CommentOutput {
    id: String,
    body: String,
    created_at: Option<String>,
    created_by: Option<String>,
}

async fn list_comments(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching comments...".dimmed());
    }

    let comments = client.list_comments(project_id, issue_id).await?;

    let outputs: Vec<CommentOutput> = comments
        .iter()
        .map(|c| CommentOutput {
            id: c.id.clone(),
            body: c.body.clone(),
            created_at: c.created_at.clone(),
            created_by: c.created_by.clone(),
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No comments found.".yellow()),
            _ => output_format.write(&Vec::<CommentOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Comments:".bold());
            println!("{}", "─".repeat(80));

            for comment in &outputs {
                let created = comment.created_at.as_deref().unwrap_or("-");
                let author = comment.created_by.as_deref().unwrap_or("-");

                println!("{} {}", "ID:".bold(), comment.id.dimmed());
                println!("{} {}", "Author:".bold(), author);
                println!("{} {}", "Created:".bold(), created.dimmed());
                println!("{}", comment.body);
                println!("{}", "─".repeat(80));
            }
        }
        _ => output_format.write(&outputs)?,
    }

    Ok(())
}

async fn add_comment(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    body: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Adding comment...".dimmed());
    }

    let comment = client.add_comment(project_id, issue_id, body).await?;

    #[derive(Serialize)]
    struct AddCommentOutput {
        success: bool,
        id: String,
        body: String,
    }

    let output = AddCommentOutput {
        success: true,
        id: comment.id.clone(),
        body: comment.body.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Comment added!", "✓".green().bold());
            println!("  {} {}", "ID:".bold(), output.id);
            println!("  {} {}", "Body:".bold(), truncate_str(&output.body, 50));
        }
        _ => output_format.write(&output)?,
    }

    Ok(())
}

async fn delete_comment(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    comment_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Deleting comment...".dimmed());
    }

    client
        .delete_comment(project_id, issue_id, comment_id)
        .await?;

    #[derive(Serialize)]
    struct DeleteCommentOutput {
        success: bool,
        comment_id: String,
        message: String,
    }

    let output = DeleteCommentOutput {
        success: true,
        comment_id: comment_id.to_string(),
        message: "Comment deleted successfully".to_string(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} {}", "✓".green().bold(), output.message);
        }
        _ => output_format.write(&output)?,
    }

    Ok(())
}

// ============== ATTACHMENTS ==============

#[derive(Serialize)]
struct AttachmentOutput {
    id: String,
    name: String,
    urn: Option<String>,
    created_at: Option<String>,
    created_by: Option<String>,
}

async fn list_attachments(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching attachments...".dimmed());
    }

    let attachments = client.list_attachments(project_id, issue_id).await?;

    let outputs: Vec<AttachmentOutput> = attachments
        .iter()
        .map(|a| AttachmentOutput {
            id: a.id.clone(),
            name: a.name.clone(),
            urn: a.urn.clone(),
            created_at: a.created_at.clone(),
            created_by: a.created_by.clone(),
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No attachments found.".yellow()),
            _ => output_format.write(&Vec::<AttachmentOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Attachments:".bold());
            println!("{}", "─".repeat(80));
            println!(
                "{:<40} {:<20} {}",
                "Name".bold(),
                "Created".bold(),
                "ID".bold()
            );
            println!("{}", "─".repeat(80));

            for attachment in &outputs {
                let created = attachment.created_at.as_deref().unwrap_or("-");
                println!(
                    "{:<40} {:<20} {}",
                    truncate_str(&attachment.name, 40).cyan(),
                    created.dimmed(),
                    attachment.id.dimmed()
                );
            }

            println!("{}", "─".repeat(80));
        }
        _ => output_format.write(&outputs)?,
    }

    Ok(())
}

// ============== STATE TRANSITIONS ==============

/// Allowed issue status transitions
const STATUS_TRANSITIONS: &[(&str, &[&str])] = &[
    ("open", &["answered", "closed"]),
    ("answered", &["open", "closed"]),
    ("closed", &["open"]),
    ("draft", &["open"]),
];

fn get_allowed_transitions(current_status: &str) -> Vec<&'static str> {
    for (status, transitions) in STATUS_TRANSITIONS {
        if *status == current_status.to_lowercase() {
            return transitions.to_vec();
        }
    }
    // Default: allow any common transitions
    vec!["open", "answered", "closed"]
}

async fn transition_issue(
    client: &IssuesClient,
    project_id: &str,
    issue_id: &str,
    target_status: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    // Get current issue to determine valid transitions
    let current_issue = client.get_issue(project_id, issue_id).await?;
    let current_status = current_issue.status.clone();
    let allowed = get_allowed_transitions(&current_status);

    // Get target status
    let new_status = match target_status {
        Some(s) => {
            let s_lower = s.to_lowercase();
            if !allowed.contains(&s_lower.as_str()) {
                anyhow::bail!(
                    "Cannot transition from '{}' to '{}'. Allowed transitions: {:?}",
                    current_status,
                    s,
                    allowed
                );
            }
            s_lower
        }
        None => {
            // In non-interactive mode, require the target status
            if interactive::is_non_interactive() {
                anyhow::bail!(
                    "Target status is required. Current: '{}'. Allowed: {:?}",
                    current_status,
                    allowed
                );
            }

            // Interactive: show allowed transitions
            println!("{} Current status: {}", "→".cyan(), current_status.bold());

            let selection = Select::new()
                .with_prompt("Select new status")
                .items(&allowed)
                .interact()?;

            allowed[selection].to_string()
        }
    };

    if output_format.supports_colors() {
        println!("{}", "Transitioning issue...".dimmed());
    }

    let request = crate::api::issues::UpdateIssueRequest {
        title: None,
        description: None,
        status: Some(new_status.clone()),
        assigned_to: None,
        due_date: None,
    };

    let updated_issue = client.update_issue(project_id, issue_id, request).await?;

    #[derive(Serialize)]
    struct TransitionOutput {
        success: bool,
        issue_id: String,
        from_status: String,
        to_status: String,
    }

    let output = TransitionOutput {
        success: true,
        issue_id: updated_issue.id.clone(),
        from_status: current_status.clone(),
        to_status: updated_issue.status.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Issue transitioned!", "✓".green().bold());
            println!(
                "  {} {} → {}",
                "Status:".bold(),
                output.from_status.dimmed(),
                output.to_status.cyan()
            );
        }
        _ => output_format.write(&output)?,
    }

    Ok(())
}

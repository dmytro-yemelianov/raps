//! RFI (Request for Information) Commands
//!
//! Commands for managing RFIs in ACC projects.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use crate::api::rfi::{CreateRfiRequest, RfiClient, UpdateRfiRequest};
use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum RfiCommands {
    /// List RFIs in a project
    List {
        /// Project ID (without "b." prefix)
        project_id: String,

        /// Filter by status (open, answered, closed, void)
        #[arg(long)]
        status: Option<String>,
    },

    /// Get details of a specific RFI
    Get {
        /// Project ID (without "b." prefix)
        project_id: String,

        /// RFI ID
        rfi_id: String,
    },

    /// Create a new RFI
    Create {
        /// Project ID (without "b." prefix)
        project_id: String,

        /// RFI title
        #[arg(long)]
        title: String,

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
    },

    /// Update an existing RFI
    Update {
        /// Project ID (without "b." prefix)
        project_id: String,

        /// RFI ID
        rfi_id: String,

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
    },
}

impl RfiCommands {
    pub async fn execute(self, client: &RfiClient, output_format: OutputFormat) -> Result<()> {
        match self {
            RfiCommands::List { project_id, status } => {
                list_rfis(client, &project_id, status.as_deref(), output_format).await
            }
            RfiCommands::Get { project_id, rfi_id } => {
                get_rfi(client, &project_id, &rfi_id, output_format).await
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
            } => {
                create_rfi(
                    client,
                    &project_id,
                    &title,
                    question,
                    &priority,
                    due_date,
                    assigned_to,
                    location,
                    discipline,
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
            } => {
                update_rfi(
                    client,
                    &project_id,
                    &rfi_id,
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
        }
    }
}

#[derive(Serialize)]
struct RfiOutput {
    id: String,
    number: Option<String>,
    title: String,
    status: String,
    priority: Option<String>,
    question: Option<String>,
    answer: Option<String>,
    due_date: Option<String>,
    assigned_to_name: Option<String>,
    created_at: Option<String>,
}

async fn list_rfis(
    client: &RfiClient,
    project_id: &str,
    status_filter: Option<&str>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching RFIs...".dimmed());
    }

    let rfis = client.list_rfis(project_id).await?;

    // Filter by status if provided
    let filtered: Vec<_> = if let Some(status) = status_filter {
        rfis.into_iter()
            .filter(|r| r.status.to_lowercase() == status.to_lowercase())
            .collect()
    } else {
        rfis
    };

    let outputs: Vec<RfiOutput> = filtered
        .iter()
        .map(|r| RfiOutput {
            id: r.id.clone(),
            number: r.number.clone(),
            title: r.title.clone(),
            status: r.status.clone(),
            priority: r.priority.clone(),
            question: r.question.clone(),
            answer: r.answer.clone(),
            due_date: r.due_date.clone(),
            assigned_to_name: r.assigned_to_name.clone(),
            created_at: r.created_at.clone(),
        })
        .collect();

    if outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No RFIs found.".yellow()),
            _ => output_format.write(&Vec::<RfiOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "RFIs:".bold());
            println!("{}", "─".repeat(100));
            println!(
                "{:<10} {:<40} {:<12} {:<10} {}",
                "Number".bold(),
                "Title".bold(),
                "Status".bold(),
                "Priority".bold(),
                "Due Date".bold()
            );
            println!("{}", "─".repeat(100));

            for rfi in &outputs {
                let number = rfi.number.as_deref().unwrap_or("-");
                let priority = rfi.priority.as_deref().unwrap_or("-");
                let due = rfi.due_date.as_deref().unwrap_or("-");
                let status_color = match rfi.status.to_lowercase().as_str() {
                    "closed" => rfi.status.green().to_string(),
                    "answered" => rfi.status.cyan().to_string(),
                    "open" => rfi.status.yellow().to_string(),
                    "void" => rfi.status.dimmed().to_string(),
                    _ => rfi.status.clone(),
                };
                let priority_color = match priority.to_lowercase().as_str() {
                    "critical" => priority.red().bold().to_string(),
                    "high" => priority.red().to_string(),
                    "normal" => priority.to_string(),
                    "low" => priority.dimmed().to_string(),
                    _ => priority.to_string(),
                };

                println!(
                    "{:<10} {:<40} {:<12} {:<10} {}",
                    number.cyan(),
                    truncate_str(&rfi.title, 40),
                    status_color,
                    priority_color,
                    due.dimmed()
                );
            }

            println!("{}", "─".repeat(100));
            println!("{} {} RFI(s) found", "→".cyan(), outputs.len());
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

async fn get_rfi(
    client: &RfiClient,
    project_id: &str,
    rfi_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching RFI details...".dimmed());
    }

    let rfi = client.get_rfi(project_id, rfi_id).await?;

    let output = RfiOutput {
        id: rfi.id.clone(),
        number: rfi.number.clone(),
        title: rfi.title.clone(),
        status: rfi.status.clone(),
        priority: rfi.priority.clone(),
        question: rfi.question.clone(),
        answer: rfi.answer.clone(),
        due_date: rfi.due_date.clone(),
        assigned_to_name: rfi.assigned_to_name.clone(),
        created_at: rfi.created_at.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "RFI Details:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "ID:".bold(), rfi.id.cyan());
            println!(
                "{:<15} {}",
                "Number:".bold(),
                rfi.number.as_deref().unwrap_or("-")
            );
            println!("{:<15} {}", "Title:".bold(), rfi.title);
            println!("{:<15} {}", "Status:".bold(), rfi.status);
            println!(
                "{:<15} {}",
                "Priority:".bold(),
                rfi.priority.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Due Date:".bold(),
                rfi.due_date.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Assigned To:".bold(),
                rfi.assigned_to_name.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Created At:".bold(),
                rfi.created_at.as_deref().unwrap_or("-")
            );

            if let Some(q) = &rfi.question {
                println!("{}", "─".repeat(60));
                println!("{}", "Question:".bold());
                println!("{}", q);
            }

            if let Some(a) = &rfi.answer {
                println!("{}", "─".repeat(60));
                println!("{}", "Answer:".bold().green());
                println!("{}", a);
            }

            println!("{}", "─".repeat(60));
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn create_rfi(
    client: &RfiClient,
    project_id: &str,
    title: &str,
    question: Option<String>,
    priority: &str,
    due_date: Option<String>,
    assigned_to: Option<String>,
    location: Option<String>,
    discipline: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Creating RFI...".dimmed());
    }

    let request = CreateRfiRequest {
        title: title.to_string(),
        question,
        priority: Some(priority.to_string()),
        due_date,
        assigned_to,
        location,
        discipline,
    };

    let rfi = client.create_rfi(project_id, request).await?;

    let output = RfiOutput {
        id: rfi.id.clone(),
        number: rfi.number.clone(),
        title: rfi.title.clone(),
        status: rfi.status.clone(),
        priority: rfi.priority.clone(),
        question: rfi.question.clone(),
        answer: rfi.answer.clone(),
        due_date: rfi.due_date.clone(),
        assigned_to_name: rfi.assigned_to_name.clone(),
        created_at: rfi.created_at.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{} RFI created successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), rfi.id.cyan());
            println!(
                "{:<15} {}",
                "Number:".bold(),
                rfi.number.as_deref().unwrap_or("-")
            );
            println!("{:<15} {}", "Title:".bold(), rfi.title);
            println!("{:<15} {}", "Status:".bold(), rfi.status);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn update_rfi(
    client: &RfiClient,
    project_id: &str,
    rfi_id: &str,
    title: Option<String>,
    question: Option<String>,
    answer: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    due_date: Option<String>,
    assigned_to: Option<String>,
    location: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Updating RFI...".dimmed());
    }

    let request = UpdateRfiRequest {
        title,
        question,
        answer,
        status,
        priority,
        due_date,
        assigned_to,
        location,
    };

    let rfi = client.update_rfi(project_id, rfi_id, request).await?;

    let output = RfiOutput {
        id: rfi.id.clone(),
        number: rfi.number.clone(),
        title: rfi.title.clone(),
        status: rfi.status.clone(),
        priority: rfi.priority.clone(),
        question: rfi.question.clone(),
        answer: rfi.answer.clone(),
        due_date: rfi.due_date.clone(),
        assigned_to_name: rfi.assigned_to_name.clone(),
        created_at: rfi.created_at.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{} RFI updated successfully!", "✓".green().bold());
            println!("{:<15} {}", "ID:".bold(), rfi.id.cyan());
            println!("{:<15} {}", "Title:".bold(), rfi.title);
            println!("{:<15} {}", "Status:".bold(), rfi.status);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

//! Webhook management commands
//!
//! Commands for creating, listing, and deleting webhook subscriptions.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Input, Select};

use crate::api::webhooks::WEBHOOK_EVENTS;
use crate::api::WebhooksClient;

#[derive(Debug, Subcommand)]
pub enum WebhookCommands {
    /// List all webhooks
    List,

    /// Create a new webhook subscription
    Create {
        /// Callback URL for webhook notifications
        #[arg(short, long)]
        url: Option<String>,

        /// Event type (e.g., dm.version.added)
        #[arg(short, long)]
        event: Option<String>,
    },

    /// Delete a webhook
    Delete {
        /// Hook ID to delete
        hook_id: String,
        /// System (e.g., data)
        #[arg(short, long, default_value = "data")]
        system: String,
        /// Event type
        #[arg(short, long)]
        event: String,
    },

    /// List available webhook events
    Events,
}

impl WebhookCommands {
    pub async fn execute(self, client: &WebhooksClient) -> Result<()> {
        match self {
            WebhookCommands::List => list_webhooks(client).await,
            WebhookCommands::Create { url, event } => create_webhook(client, url, event).await,
            WebhookCommands::Delete {
                hook_id,
                system,
                event,
            } => delete_webhook(client, &system, &event, &hook_id).await,
            WebhookCommands::Events => list_events(client),
        }
    }
}

async fn list_webhooks(client: &WebhooksClient) -> Result<()> {
    println!("{}", "Fetching webhooks...".dimmed());

    let webhooks = client.list_all_webhooks().await?;

    if webhooks.is_empty() {
        println!("{}", "No webhooks found.".yellow());
        return Ok(());
    }

    println!("\n{}", "Webhooks:".bold());
    println!("{}", "─".repeat(90));
    println!(
        "{:<15} {:<25} {:<35} {}",
        "Status".bold(),
        "Event".bold(),
        "Callback URL".bold(),
        "Hook ID".bold()
    );
    println!("{}", "─".repeat(90));

    for webhook in webhooks {
        let status_icon = if webhook.status == "active" {
            "✓ active".green()
        } else {
            format!("✗ {}", webhook.status).red()
        };

        let url = truncate_str(&webhook.callback_url, 35);

        println!(
            "{:<15} {:<25} {:<35} {}",
            status_icon,
            webhook.event.cyan(),
            url,
            webhook.hook_id.dimmed()
        );
    }

    println!("{}", "─".repeat(90));
    Ok(())
}

async fn create_webhook(
    client: &WebhooksClient,
    callback_url: Option<String>,
    event: Option<String>,
) -> Result<()> {
    // Get callback URL
    let url = match callback_url {
        Some(u) => u,
        None => Input::new()
            .with_prompt("Enter callback URL")
            .validate_with(|input: &String| -> Result<(), &str> {
                if input.starts_with("http://") || input.starts_with("https://") {
                    Ok(())
                } else {
                    Err("URL must start with http:// or https://")
                }
            })
            .interact_text()?,
    };

    // Get event type
    let event_type = match event {
        Some(e) => e,
        None => {
            let event_labels: Vec<String> = WEBHOOK_EVENTS
                .iter()
                .map(|(e, d)| format!("{} - {}", e, d))
                .collect();

            let selection = Select::new()
                .with_prompt("Select event type")
                .items(&event_labels)
                .interact()?;

            WEBHOOK_EVENTS[selection].0.to_string()
        }
    };

    // Determine system from event
    let system = if event_type.starts_with("dm.") {
        "data"
    } else if event_type.starts_with("extraction.") {
        "derivative"
    } else {
        "data"
    };

    println!("{}", "Creating webhook...".dimmed());

    let webhook = client
        .create_webhook(system, &event_type, &url, None)
        .await?;

    println!("{} Webhook created successfully!", "✓".green().bold());
    println!("  {} {}", "Hook ID:".bold(), webhook.hook_id);
    println!("  {} {}", "Event:".bold(), webhook.event.cyan());
    println!("  {} {}", "Status:".bold(), webhook.status.green());
    println!("  {} {}", "Callback:".bold(), webhook.callback_url);

    Ok(())
}

async fn delete_webhook(
    client: &WebhooksClient,
    system: &str,
    event: &str,
    hook_id: &str,
) -> Result<()> {
    println!("{}", "Deleting webhook...".dimmed());

    client.delete_webhook(system, event, hook_id).await?;

    println!("{} Webhook deleted successfully!", "✓".green().bold());
    Ok(())
}

fn list_events(_client: &WebhooksClient) -> Result<()> {
    println!("\n{}", "Available Webhook Events:".bold());
    println!("{}", "─".repeat(60));

    for (event, description) in WEBHOOK_EVENTS {
        println!(
            "  {} {}",
            event.cyan(),
            format!("- {}", description).dimmed()
        );
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

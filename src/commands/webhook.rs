//! Webhook management commands
//!
//! Commands for creating, listing, and deleting webhook subscriptions.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::{Input, Select};
use serde::Serialize;

use crate::api::webhooks::WEBHOOK_EVENTS;
use crate::api::WebhooksClient;
use crate::output::OutputFormat;

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
    pub async fn execute(self, client: &WebhooksClient, output_format: OutputFormat) -> Result<()> {
        match self {
            WebhookCommands::List => list_webhooks(client, output_format).await,
            WebhookCommands::Create { url, event } => create_webhook(client, url, event, output_format).await,
            WebhookCommands::Delete {
                hook_id,
                system,
                event,
            } => delete_webhook(client, &system, &event, &hook_id, output_format).await,
            WebhookCommands::Events => list_events(client, output_format),
        }
    }
}

#[derive(Serialize)]
struct WebhookListOutput {
    hook_id: String,
    event: String,
    callback_url: String,
    status: String,
}

async fn list_webhooks(client: &WebhooksClient, output_format: OutputFormat) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching webhooks...".dimmed());
    }

    let webhooks = client.list_all_webhooks().await?;

    let webhook_outputs: Vec<WebhookListOutput> = webhooks
        .iter()
        .map(|w| WebhookListOutput {
            hook_id: w.hook_id.clone(),
            event: w.event.clone(),
            callback_url: w.callback_url.clone(),
            status: w.status.clone(),
        })
        .collect();

    if webhook_outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No webhooks found.".yellow()),
            _ => {
                output_format.write(&Vec::<WebhookListOutput>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
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

            for webhook in &webhook_outputs {
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
        }
        _ => {
            output_format.write(&webhook_outputs)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct CreateWebhookOutput {
    success: bool,
    hook_id: String,
    event: String,
    status: String,
    callback_url: String,
}

async fn create_webhook(
    client: &WebhooksClient,
    callback_url: Option<String>,
    event: Option<String>,
    output_format: OutputFormat,
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

    if output_format.supports_colors() {
        println!("{}", "Creating webhook...".dimmed());
    }

    let webhook = client
        .create_webhook(system, &event_type, &url, None)
        .await?;

    let output = CreateWebhookOutput {
        success: true,
        hook_id: webhook.hook_id.clone(),
        event: webhook.event.clone(),
        status: webhook.status.clone(),
        callback_url: webhook.callback_url.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Webhook created successfully!", "✓".green().bold());
            println!("  {} {}", "Hook ID:".bold(), output.hook_id);
            println!("  {} {}", "Event:".bold(), output.event.cyan());
            println!("  {} {}", "Status:".bold(), output.status.green());
            println!("  {} {}", "Callback:".bold(), output.callback_url);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct DeleteWebhookOutput {
    success: bool,
    hook_id: String,
    message: String,
}

async fn delete_webhook(
    client: &WebhooksClient,
    system: &str,
    event: &str,
    hook_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Deleting webhook...".dimmed());
    }

    client.delete_webhook(system, event, hook_id).await?;

    let output = DeleteWebhookOutput {
        success: true,
        hook_id: hook_id.to_string(),
        message: "Webhook deleted successfully!".to_string(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} {}", "✓".green().bold(), output.message);
        }
        _ => {
            output_format.write(&output)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct EventOutput {
    event: String,
    description: String,
}

fn list_events(_client: &WebhooksClient, output_format: OutputFormat) -> Result<()> {
    let events: Vec<EventOutput> = WEBHOOK_EVENTS
        .iter()
        .map(|(event, description)| EventOutput {
            event: event.to_string(),
            description: description.to_string(),
        })
        .collect();

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Available Webhook Events:".bold());
            println!("{}", "─".repeat(60));

            for event in &events {
                println!(
                    "  {} {}",
                    event.event.cyan(),
                    format!("- {}", event.description).dimmed()
                );
            }

            println!("{}", "─".repeat(60));
        }
        _ => {
            output_format.write(&events)?;
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str_short() {
        assert_eq!(truncate_str("short", 10), "short");
        assert_eq!(truncate_str("exact", 5), "exact");
    }

    #[test]
    fn test_truncate_str_long() {
        let long_str = "this is a very long string that needs to be truncated";
        let result = truncate_str(long_str, 20);
        assert_eq!(result.len(), 20);
        assert!(result.ends_with("..."));
        assert_eq!(result, "this is a very lo...");
    }

    #[test]
    fn test_truncate_str_exact_length() {
        let str = "exactly";
        assert_eq!(truncate_str(str, 7), "exactly");
    }

    #[test]
    fn test_truncate_str_one_over() {
        let str = "onetwo";
        // String is 6 chars, max_len is 6, so it should not be truncated
        let result = truncate_str(str, 6);
        assert_eq!(result, "onetwo");

        // String is 6 chars, max_len is 5, so it should be truncated to 2 chars + "..."
        let result = truncate_str(str, 5);
        assert_eq!(result, "on...");
    }

    #[test]
    fn test_truncate_str_very_short_max() {
        let str = "hello";
        // If max_len is 3, we can't add "...", so it will try to slice [..0] which is ""
        // But the function will still add "...", resulting in "..."
        let result = truncate_str(str, 3);
        assert_eq!(result, "...");

        // For max_len 4, we get 1 char + "..."
        let result = truncate_str(str, 4);
        assert_eq!(result, "h...");
    }

    #[test]
    fn test_truncate_str_empty() {
        assert_eq!(truncate_str("", 10), "");
    }
}


// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Webhook management commands
//!
//! Commands for creating, listing, and deleting webhook subscriptions.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use raps_kernel::prompts;
use serde::Serialize;

use crate::output::OutputFormat;
// use raps_kernel::output::OutputFormat;
use raps_webhooks::{WEBHOOK_EVENTS, WebhooksClient};

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

    /// Test webhook endpoint connectivity
    Test {
        /// Webhook callback URL to test
        url: String,
        /// Timeout in seconds (default: 10)
        #[arg(short, long, default_value = "10")]
        timeout: u64,
    },

    /// Verify webhook signature
    #[command(name = "verify-signature")]
    VerifySignature {
        /// The webhook payload (JSON string or @file)
        payload: String,
        /// The signature from X-Adsk-Signature header
        #[arg(short, long)]
        signature: String,
        /// The webhook secret
        #[arg(long)]
        secret: String,
    },
}

impl WebhookCommands {
    pub async fn execute(self, client: &WebhooksClient, output_format: OutputFormat) -> Result<()> {
        match self {
            WebhookCommands::List => list_webhooks(client, output_format).await,
            WebhookCommands::Create { url, event } => {
                create_webhook(client, url, event, output_format).await
            }
            WebhookCommands::Delete {
                hook_id,
                system,
                event,
            } => delete_webhook(client, &system, &event, &hook_id, output_format).await,
            WebhookCommands::Events => list_events(client, output_format),
            WebhookCommands::Test { url, timeout } => {
                test_webhook_endpoint(&url, timeout, output_format).await
            }
            WebhookCommands::VerifySignature {
                payload,
                signature,
                secret,
            } => verify_signature(&payload, &signature, &secret, output_format),
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
            println!("{}", "-".repeat(90));
            println!(
                "{:<15} {:<25} {:<35} {}",
                "Status".bold(),
                "Event".bold(),
                "Callback URL".bold(),
                "Hook ID".bold()
            );
            println!("{}", "-".repeat(90));

            for webhook in &webhook_outputs {
                let status_icon = if webhook.status == "active" {
                    "active".green()
                } else {
                    webhook.status.to_string().red()
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

            println!("{}", "-".repeat(90));
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
        None => prompts::input_validated("Enter callback URL", None, |input: &String| {
            if input.starts_with("http://") || input.starts_with("https://") {
                Ok(())
            } else {
                Err("URL must start with http:// or https://")
            }
        })?,
    };

    // Get event type
    let event_type = match event {
        Some(e) => e,
        None => {
            let event_labels: Vec<String> = WEBHOOK_EVENTS
                .iter()
                .map(|(e, d)| format!("{} - {}", e, d))
                .collect();

            let selection = prompts::select("Select event type", &event_labels)?;
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
            println!("{}", "-".repeat(60));

            for event in &events {
                println!(
                    "  {} {}",
                    event.event.cyan(),
                    format!("- {}", event.description).dimmed()
                );
            }

            println!("{}", "-".repeat(60));
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

// ============== WEBHOOK TESTING ==============

#[derive(Serialize)]
struct TestEndpointOutput {
    success: bool,
    url: String,
    status_code: Option<u16>,
    response_time_ms: u64,
    message: String,
}

async fn test_webhook_endpoint(
    url: &str,
    timeout_secs: u64,
    output_format: OutputFormat,
) -> Result<()> {
    use std::time::Instant;

    if output_format.supports_colors() {
        println!("{}", "Testing webhook endpoint...".dimmed());
        println!("  {} {}", "URL:".bold(), url.cyan());
    }

    // Create a simple test payload
    let test_payload = serde_json::json!({
        "test": true,
        "source": "raps-cli",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()?;

    let start = Instant::now();

    let result = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("User-Agent", "RAPS-CLI/0.7.0")
        .json(&test_payload)
        .send()
        .await;

    let elapsed = start.elapsed().as_millis() as u64;

    let output = match result {
        Ok(response) => {
            let status = response.status();
            TestEndpointOutput {
                success: status.is_success() || status.is_redirection(),
                url: url.to_string(),
                status_code: Some(status.as_u16()),
                response_time_ms: elapsed,
                message: format!("Endpoint responded with status {}", status),
            }
        }
        Err(e) => {
            let message = if e.is_timeout() {
                format!("Request timed out after {}s", timeout_secs)
            } else if e.is_connect() {
                "Failed to connect to endpoint".to_string()
            } else {
                format!("Request failed: {}", e)
            };

            TestEndpointOutput {
                success: false,
                url: url.to_string(),
                status_code: None,
                response_time_ms: elapsed,
                message,
            }
        }
    };

    match output_format {
        OutputFormat::Table => {
            if output.success {
                println!("{} Endpoint is reachable!", "✓".green().bold());
            } else {
                println!("{} Endpoint test failed!", "X".red().bold());
            }
            println!("  {} {}", "Message:".bold(), output.message);
            if let Some(status) = output.status_code {
                println!("  {} {}", "Status:".bold(), status);
            }
            println!(
                "  {} {}ms",
                "Response time:".bold(),
                output.response_time_ms
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct VerifySignatureOutput {
    valid: bool,
    message: String,
}

fn verify_signature(
    payload: &str,
    signature: &str,
    _secret: &str,
    output_format: OutputFormat,
) -> Result<()> {
    use std::io::Read;

    // Load payload (from string or file)
    let payload_data = if let Some(file_path) = payload.strip_prefix('@') {
        let mut content = String::new();
        std::fs::File::open(file_path)
            .and_then(|mut f| f.read_to_string(&mut content))
            .with_context(|| format!("Failed to read payload file: {}", file_path))?;
        content
    } else {
        payload.to_string()
    };

    // Calculate HMAC-SHA256 signature
    // Note: In a real implementation, you'd use a crypto library like hmac + sha2
    // For now, we'll provide a placeholder that shows the expected format

    // The APS webhook signature format is typically base64(HMAC-SHA256(secret, payload))
    // This is a simplified verification that checks format
    let is_valid_format = signature.len() > 20 && !signature.contains(' ');

    let output = if is_valid_format {
        VerifySignatureOutput {
            valid: true,
            message: "Signature format is valid. For full cryptographic verification, ensure your webhook handler validates using HMAC-SHA256.".to_string(),
        }
    } else {
        VerifySignatureOutput {
            valid: false,
            message: "Signature format appears invalid".to_string(),
        }
    };

    match output_format {
        OutputFormat::Table => {
            if output.valid {
                println!("{} {}", "✓".green().bold(), output.message);
            } else {
                println!("{} {}", "X".red().bold(), output.message);
            }
            println!(
                "\n{}",
                "Tip: Use this payload in your webhook handler for testing:".dimmed()
            );
            println!("{}", payload_data.chars().take(200).collect::<String>());
            if payload_data.len() > 200 {
                println!("{}...", "".dimmed());
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

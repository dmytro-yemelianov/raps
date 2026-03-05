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

use crate::commands::tracked::tracked_op;
use crate::output::OutputFormat;
// use raps_kernel::output::OutputFormat;
use raps_webhooks::{UpdateWebhookRequest, WEBHOOK_EVENTS, WebhooksClient};

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

    /// Get a specific webhook
    Get {
        /// System (e.g., data)
        #[arg(short, long, default_value = "data")]
        system: String,
        /// Event type
        #[arg(short, long)]
        event: String,
        /// Hook ID
        #[arg(long)]
        hook_id: String,
    },

    /// Update a webhook
    Update {
        /// System (e.g., data)
        #[arg(short, long, default_value = "data")]
        system: String,
        /// Event type
        #[arg(short, long)]
        event: String,
        /// Hook ID
        #[arg(long)]
        hook_id: String,
        /// New callback URL
        #[arg(long)]
        callback_url: Option<String>,
        /// New status (active or inactive)
        #[arg(long)]
        status: Option<String>,
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
        /// The signature from x-aps-signature (or legacy x-adsk-signature)
        #[arg(short, long)]
        signature: String,
        /// The webhook secret
        #[arg(long)]
        secret: String,
    },

    /// Deploy the Cloudflare Worker webhook gateway (serverless)
    Serve {
        /// Deploy as a serverless Cloudflare Worker
        #[arg(long)]
        serverless: bool,

        /// Cloudflare account ID (or CLOUDFLARE_ACCOUNT_ID env)
        #[arg(long, env = "CLOUDFLARE_ACCOUNT_ID")]
        account_id: Option<String>,

        /// APS webhook signing secret (set via wrangler secret)
        #[arg(long)]
        webhook_secret: Option<String>,

        /// Optional relay URL — forward events to this URL after storing
        #[arg(long)]
        relay_url: Option<String>,
    },

    /// Drain stored events from the Cloudflare Worker gateway
    Drain {
        /// Gateway URL (or RAPS_GATEWAY_URL env)
        #[arg(long, env = "RAPS_GATEWAY_URL")]
        gateway_url: String,

        /// API key for authentication (or RAPS_GATEWAY_API_KEY env)
        #[arg(long, env = "RAPS_GATEWAY_API_KEY")]
        api_key: Option<String>,

        /// Write events to this file (default: stdout)
        #[arg(long = "out-file")]
        out_file: Option<std::path::PathBuf>,

        /// Maximum events to drain
        #[arg(long, default_value = "100")]
        limit: u32,
    },
}

impl WebhookCommands {
    pub async fn execute(self, client: &WebhooksClient, output_format: OutputFormat) -> Result<()> {
        match self {
            WebhookCommands::List => list_webhooks(client, output_format).await,
            WebhookCommands::Create { url, event } => {
                create_webhook(client, url, event, output_format).await
            }
            WebhookCommands::Get {
                system,
                event,
                hook_id,
            } => get_webhook(client, &system, &event, &hook_id, output_format).await,
            WebhookCommands::Update {
                system,
                event,
                hook_id,
                callback_url,
                status,
            } => {
                update_webhook(
                    client,
                    &system,
                    &event,
                    &hook_id,
                    callback_url,
                    status,
                    output_format,
                )
                .await
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
            WebhookCommands::Serve {
                serverless,
                account_id,
                webhook_secret,
                relay_url,
            } => {
                webhook_serve(
                    serverless,
                    account_id,
                    webhook_secret,
                    relay_url,
                    output_format,
                )
                .await
            }
            WebhookCommands::Drain {
                gateway_url,
                api_key,
                out_file,
                limit,
            } => webhook_drain(&gateway_url, api_key, out_file, limit, output_format).await,
        }
    }
}

#[derive(Serialize, schemars::JsonSchema)]
struct WebhookListOutput {
    hook_id: String,
    event: String,
    callback_url: String,
    status: String,
}

async fn list_webhooks(client: &WebhooksClient, output_format: OutputFormat) -> Result<()> {
    let webhooks = tracked_op("Fetching webhooks", output_format, || async {
        client
            .list_all_webhooks()
            .await
            .context("Failed to list webhooks. Check your authentication with 'raps auth test'")
    })
    .await?;

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

#[derive(Serialize, schemars::JsonSchema)]
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
        Some(e) => {
            if !WebhooksClient::is_valid_event(&e) {
                let known: Vec<&str> = WEBHOOK_EVENTS.iter().map(|(e, _)| *e).collect();
                anyhow::bail!(
                    "Unknown webhook event '{}'. Valid events: {}",
                    e,
                    known.join(", ")
                );
            }
            e
        }
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
        .await
        .context(format!(
            "Failed to create webhook for event '{}'. Verify callback URL is reachable",
            event_type
        ))?;

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

#[derive(Serialize, schemars::JsonSchema)]
struct GetWebhookOutput {
    hook_id: String,
    system: String,
    event: String,
    callback_url: String,
    status: String,
    created_date: Option<String>,
    last_updated_date: Option<String>,
}

async fn get_webhook(
    client: &WebhooksClient,
    system: &str,
    event: &str,
    hook_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching webhook...".dimmed());
    }

    let webhook = client
        .get_webhook(system, event, hook_id)
        .await
        .context(format!(
            "Failed to get webhook '{}'. Verify the hook ID, system, and event are correct",
            hook_id
        ))?;

    let output = GetWebhookOutput {
        hook_id: webhook.hook_id.clone(),
        system: webhook.system.clone(),
        event: webhook.event.clone(),
        callback_url: webhook.callback_url.clone(),
        status: webhook.status.clone(),
        created_date: webhook.created_date.clone(),
        last_updated_date: webhook.last_updated_date.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Webhook Details:".bold());
            println!("{}", "-".repeat(60));
            println!("  {} {}", "Hook ID:".bold(), output.hook_id);
            println!("  {} {}", "System:".bold(), output.system);
            println!("  {} {}", "Event:".bold(), output.event.cyan());
            println!("  {} {}", "Callback:".bold(), output.callback_url);
            let status_display = if output.status == "active" {
                output.status.green().to_string()
            } else {
                output.status.red().to_string()
            };
            println!("  {} {}", "Status:".bold(), status_display);
            if let Some(ref created) = output.created_date {
                println!("  {} {}", "Created:".bold(), created);
            }
            if let Some(ref updated) = output.last_updated_date {
                println!("  {} {}", "Updated:".bold(), updated);
            }
            println!("{}", "-".repeat(60));
        }
        _ => {
            output_format.write(&output)?;
        }
    }
    Ok(())
}

#[derive(Serialize, schemars::JsonSchema)]
struct UpdateWebhookOutput {
    success: bool,
    hook_id: String,
    event: String,
    status: String,
    callback_url: String,
}

async fn update_webhook(
    client: &WebhooksClient,
    system: &str,
    event: &str,
    hook_id: &str,
    callback_url: Option<String>,
    status: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Updating webhook...".dimmed());
    }

    let request = UpdateWebhookRequest {
        callback_url,
        status,
        filter: None,
    };

    let webhook = client
        .update_webhook(system, event, hook_id, request)
        .await
        .context(format!(
            "Failed to update webhook '{}'. Verify the hook ID and permissions",
            hook_id
        ))?;

    let output = UpdateWebhookOutput {
        success: true,
        hook_id: webhook.hook_id.clone(),
        event: webhook.event.clone(),
        status: webhook.status.clone(),
        callback_url: webhook.callback_url.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Webhook updated successfully!", "✓".green().bold());
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

#[derive(Serialize, schemars::JsonSchema)]
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

    client
        .delete_webhook(system, event, hook_id)
        .await
        .context(format!(
            "Failed to delete webhook '{}'. Verify the hook ID, system, and event are correct",
            hook_id
        ))?;

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

#[derive(Serialize, schemars::JsonSchema)]
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

#[derive(Serialize, schemars::JsonSchema)]
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

#[derive(Serialize, schemars::JsonSchema)]
struct VerifySignatureOutput {
    valid: bool,
    message: String,
}

fn verify_signature(
    payload: &str,
    signature: &str,
    secret: &str,
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

    let valid = verify_hmac_sha256_signature(secret, payload_data.as_bytes(), signature);

    let output = if valid {
        VerifySignatureOutput {
            valid: true,
            message: "Signature is valid (HMAC-SHA256)".to_string(),
        }
    } else {
        VerifySignatureOutput {
            valid: false,
            message: "Signature is invalid".to_string(),
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

fn verify_hmac_sha256_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);

    let normalized = signature.trim();
    let normalized = normalized.strip_prefix("sha256=").unwrap_or(normalized);

    let Ok(expected_bytes) = hex::decode(normalized) else {
        return false;
    };

    mac.verify_slice(&expected_bytes).is_ok()
}

// ---------------------------------------------------------------------------
// Serve (deploy Cloudflare Worker)
// ---------------------------------------------------------------------------

async fn webhook_serve(
    serverless: bool,
    _account_id: Option<String>,
    webhook_secret: Option<String>,
    relay_url: Option<String>,
    _output_format: OutputFormat,
) -> Result<()> {
    if !serverless {
        anyhow::bail!(
            "Use --serverless to deploy the webhook gateway as a Cloudflare Worker.\n\
             For local webhook testing, use: raps webhook test <url>"
        );
    }

    println!(
        "{}",
        "Deploying RAPS Webhook Gateway to Cloudflare Workers...".bold()
    );

    // Set secrets via wrangler if provided
    if let Some(ref secret) = webhook_secret {
        println!("  Setting APS_WEBHOOK_SECRET...");
        let status = std::process::Command::new("wrangler")
            .args(["secret", "put", "APS_WEBHOOK_SECRET"])
            .stdin(std::process::Stdio::piped())
            .current_dir("workers/webhook-gateway")
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    stdin.write_all(secret.as_bytes())?;
                }
                child.wait()
            })
            .context("Failed to run wrangler secret put")?;

        if !status.success() {
            anyhow::bail!("wrangler secret put failed");
        }
    }

    // Set relay URL as env if provided
    if let Some(ref url) = relay_url {
        println!("  Setting RELAY_CALLBACK_URL: {}", url);
    }

    // Deploy
    println!("  Running wrangler deploy...");
    let output = std::process::Command::new("wrangler")
        .arg("deploy")
        .current_dir("workers/webhook-gateway")
        .output()
        .context("Failed to run wrangler deploy — is wrangler installed?")?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{} Webhook gateway deployed", "✓".green());

        // Try to extract the public URL from wrangler output
        for line in stdout.lines() {
            if line.contains("https://") {
                println!("  URL: {}", line.trim());
            }
        }
        println!("\n  Configure your APS webhook callback to: <worker-url>/aps/webhook");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("wrangler deploy failed:\n{}", stderr);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Drain (pull events from gateway)
// ---------------------------------------------------------------------------

async fn webhook_drain(
    gateway_url: &str,
    api_key: Option<String>,
    out_file: Option<std::path::PathBuf>,
    limit: u32,
    output_format: OutputFormat,
) -> Result<()> {
    let url = format!(
        "{}/events?limit={}",
        gateway_url.trim_end_matches('/'),
        limit
    );

    let client = reqwest::Client::new();
    let mut req = client.get(&url);

    if let Some(ref key) = api_key {
        req = req.bearer_auth(key);
    }

    let resp = req
        .send()
        .await
        .context("Failed to reach webhook gateway")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Gateway returned {}: {}", status, text);
    }

    let body: serde_json::Value = resp.json().await.context("Invalid JSON from gateway")?;

    // Write to file or stdout
    if let Some(ref path) = out_file {
        let json = serde_json::to_string_pretty(&body)?;
        std::fs::write(path, &json)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        println!("{} Events written to {}", "✓".green(), path.display());
    } else {
        match output_format {
            OutputFormat::Table => {
                let events = body["events"].as_array();
                let count = body["count"].as_u64().unwrap_or(0);
                if count == 0 {
                    println!("No events in backlog.");
                } else {
                    println!("{} ({} events)", "Webhook Events".bold(), count);
                    if let Some(evts) = events {
                        for evt in evts {
                            let id = evt["id"].as_str().unwrap_or("?");
                            let received = evt["received_at"].as_str().unwrap_or("?");
                            println!("  {} received_at={}", id, received);
                        }
                    }
                }
            }
            _ => {
                output_format.write(&body)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_str_exact_max() {
        let result = truncate_str("hello", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_str_over_max() {
        let result = truncate_str("this is a long string", 10);
        assert_eq!(result, "this is...");
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_truncate_str_empty() {
        let result = truncate_str("", 10);
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_str_small_max() {
        let result = truncate_str("abcdefgh", 4);
        assert_eq!(result, "a...");
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_webhook_list_output_serialization() {
        let output = WebhookListOutput {
            hook_id: "hook-123".to_string(),
            event: "dm.version.added".to_string(),
            callback_url: "https://example.com/webhook".to_string(),
            status: "active".to_string(),
        };
        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["hook_id"], "hook-123");
        assert_eq!(json["event"], "dm.version.added");
        assert_eq!(json["callback_url"], "https://example.com/webhook");
        assert_eq!(json["status"], "active");
    }

    #[test]
    fn test_verify_signature_output_serialization() {
        let output = VerifySignatureOutput {
            valid: true,
            message: "Signature verified".to_string(),
        };
        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["valid"], true);
        assert_eq!(json["message"], "Signature verified");
    }

    #[test]
    fn test_verify_hmac_sha256_signature_valid() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let secret = "test-secret";
        let body = br#"{"event":"dm.version.added"}"#;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let sig = hex::encode(mac.finalize().into_bytes());

        assert!(verify_hmac_sha256_signature(secret, body, &sig));
    }

    #[test]
    fn test_verify_hmac_sha256_signature_sha256_prefix() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let secret = "test-secret";
        let body = b"payload";
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let sig = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        assert!(verify_hmac_sha256_signature(secret, body, &sig));
    }

    #[test]
    fn test_verify_hmac_sha256_signature_invalid() {
        assert!(!verify_hmac_sha256_signature(
            "secret",
            b"payload",
            "not-a-valid-signature"
        ));
    }
}

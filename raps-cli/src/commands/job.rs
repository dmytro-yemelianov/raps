// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Serverless job management commands.
//!
//! Check status, list, and cancel Fly.io machine-based jobs dispatched
//! via `raps translate start --serverless`.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum JobCommands {
    /// Check the status of a serverless job (Fly.io machine)
    Status {
        /// Machine ID returned by dispatch
        id: String,

        /// Wait for the machine to reach a terminal state
        #[arg(long)]
        wait: bool,

        /// Polling interval in seconds (used with --wait)
        #[arg(long, default_value = "5")]
        poll_secs: u64,
    },

    /// List active and recent serverless machines
    List {
        /// Filter by machine state (e.g. started, stopped, destroyed)
        #[arg(long)]
        state: Option<String>,
    },

    /// Request cancellation of a running machine (stop + destroy)
    Cancel {
        /// Machine ID to cancel
        machine_id: String,
    },
}

#[derive(Serialize, schemars::JsonSchema)]
struct JobStatusOutput {
    id: String,
    state: String,
    region: String,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, schemars::JsonSchema)]
struct JobListOutput {
    machines: Vec<JobStatusOutput>,
    total: usize,
}

impl JobCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            JobCommands::Status {
                id,
                wait,
                poll_secs,
            } => job_status(&id, wait, poll_secs, output_format).await,
            JobCommands::List { state } => job_list(state, output_format).await,
            JobCommands::Cancel { machine_id } => job_cancel(&machine_id, output_format).await,
        }
    }
}

async fn job_status(
    id: &str,
    wait: bool,
    poll_secs: u64,
    output_format: OutputFormat,
) -> Result<()> {
    use raps_kernel::serverless::ServerlessDispatchAgent;

    let agent =
        ServerlessDispatchAgent::from_config().context("Failed to load serverless config")?;

    loop {
        let status = agent.machine_status(id).await?;

        let out = JobStatusOutput {
            id: status.id.clone(),
            state: status.state.clone(),
            region: status.region.clone(),
            created_at: status.created_at.clone(),
            updated_at: status.updated_at.clone(),
        };

        match output_format {
            OutputFormat::Table => {
                println!("{} {}", "Machine:".bold(), out.id);
                println!("  State:   {}", colorize_state(&out.state));
                println!("  Region:  {}", out.region);
                if !out.created_at.is_empty() {
                    println!("  Created: {}", out.created_at);
                }
                if !out.updated_at.is_empty() {
                    println!("  Updated: {}", out.updated_at);
                }
            }
            _ => {
                output_format.write(&out)?;
            }
        }

        if !wait {
            break;
        }

        match status.state.as_str() {
            "stopped" | "destroyed" | "failed" => break,
            _ => {
                tokio::time::sleep(std::time::Duration::from_secs(poll_secs)).await;
            }
        }
    }

    Ok(())
}

async fn job_list(state_filter: Option<String>, output_format: OutputFormat) -> Result<()> {
    use raps_kernel::serverless::ServerlessDispatchAgent;

    let agent =
        ServerlessDispatchAgent::from_config().context("Failed to load serverless config")?;

    let machines = agent.list_machines().await?;

    let filtered: Vec<JobStatusOutput> = machines
        .into_iter()
        .filter(|m| {
            if let Some(ref f) = state_filter {
                m.state.eq_ignore_ascii_case(f)
            } else {
                true
            }
        })
        .map(|m| JobStatusOutput {
            id: m.id,
            state: m.state,
            region: m.region,
            created_at: m.created_at,
            updated_at: m.updated_at,
        })
        .collect();

    let total = filtered.len();
    let out = JobListOutput {
        machines: filtered,
        total,
    };

    match output_format {
        OutputFormat::Table => {
            if out.machines.is_empty() {
                println!("No machines found.");
            } else {
                println!("{}", "Serverless Machines".bold());
                println!("  {:<26} {:<12} {:<8}", "Machine ID", "State", "Region");
                println!("  {}", "─".repeat(50));
                for m in &out.machines {
                    println!(
                        "  {:<26} {:<12} {:<8}",
                        m.id,
                        colorize_state(&m.state),
                        m.region,
                    );
                }
                println!("\n  Total: {}", out.total);
            }
        }
        _ => {
            output_format.write(&out)?;
        }
    }

    Ok(())
}

async fn job_cancel(machine_id: &str, _output_format: OutputFormat) -> Result<()> {
    use raps_kernel::serverless::ServerlessDispatchAgent;

    let agent =
        ServerlessDispatchAgent::from_config().context("Failed to load serverless config")?;

    // First check current state
    let status = agent.machine_status(machine_id).await?;
    match status.state.as_str() {
        "stopped" | "destroyed" => {
            println!(
                "{} Machine {} is already {}",
                "!".yellow(),
                machine_id,
                status.state
            );
            return Ok(());
        }
        _ => {}
    }

    // Stop the machine via Fly Machines API
    let config = raps_kernel::serverless::SwarmConfig::load()?;
    let client = reqwest::Client::new();
    let url = format!(
        "{}/v1/apps/{}/machines/{}/stop",
        config.serverless.api_url, config.serverless.fly_app, machine_id,
    );

    let resp = client
        .post(&url)
        .bearer_auth(&config.serverless.fly_token)
        .send()
        .await
        .context("Stop request failed")?;

    if resp.status().is_success() {
        println!("{} Machine {} stop requested", "✓".green(), machine_id);
    } else {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to stop machine: {}", text);
    }

    Ok(())
}

fn colorize_state(state: &str) -> String {
    match state {
        "started" | "running" => state.green().to_string(),
        "stopped" | "destroyed" => state.dimmed().to_string(),
        "failed" => state.red().to_string(),
        "created" => state.cyan().to_string(),
        _ => state.yellow().to_string(),
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Serverless dispatch agent for Fly.io Machines API.
//!
//! Reads configuration from `~/.config/raps/swarm.toml` `[serverless]` section.
//! Provides [`ServerlessDispatchAgent`] to create, monitor, and list
//! ephemeral Fly Machines for translation workloads.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Top-level swarm configuration file (`swarm.toml`).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmConfig {
    #[serde(default)]
    pub serverless: ServerlessConfig,

    #[serde(default)]
    pub redis: RedisConfig,

    #[serde(default)]
    pub worker: WorkerConfig,
}

/// Redis connection settings for distributed mode.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisConfig {
    #[serde(default = "default_redis_url")]
    pub url: String,
    #[serde(default = "default_pool_size")]
    pub pool_size: usize,
    #[serde(default = "default_namespace")]
    pub namespace: String,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: default_redis_url(),
            pool_size: default_pool_size(),
            namespace: default_namespace(),
        }
    }
}

fn default_redis_url() -> String {
    "redis://127.0.0.1:6379".into()
}
fn default_pool_size() -> usize {
    8
}
fn default_namespace() -> String {
    "raps".into()
}

/// Worker daemon settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WorkerConfig {
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_heartbeat_secs")]
    pub heartbeat_secs: u64,
    #[serde(default = "default_queues")]
    pub queues: Vec<String>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            concurrency: default_concurrency(),
            heartbeat_secs: default_heartbeat_secs(),
            queues: default_queues(),
        }
    }
}

fn default_concurrency() -> usize {
    4
}
fn default_heartbeat_secs() -> u64 {
    30
}
fn default_queues() -> Vec<String> {
    vec!["critical".into(), "normal".into(), "background".into()]
}

/// Serverless (Fly.io) dispatch settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerlessConfig {
    #[serde(default)]
    pub fly_app: String,
    #[serde(default)]
    pub fly_token: String,
    #[serde(default = "default_preferred_region")]
    pub preferred_region: String,
    #[serde(default = "default_machine_size")]
    pub machine_size: String,
    #[serde(default = "default_max_machines")]
    pub max_machines: u32,
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,
    #[serde(default = "default_fly_api_url")]
    pub api_url: String,
    #[serde(default)]
    pub notify_slack_url: Option<String>,
}

impl Default for ServerlessConfig {
    fn default() -> Self {
        Self {
            fly_app: String::new(),
            fly_token: String::new(),
            preferred_region: default_preferred_region(),
            machine_size: default_machine_size(),
            max_machines: default_max_machines(),
            idle_timeout_secs: default_idle_timeout(),
            api_url: default_fly_api_url(),
            notify_slack_url: None,
        }
    }
}

fn default_preferred_region() -> String {
    "iad".into()
}
fn default_machine_size() -> String {
    "shared-cpu-2x".into()
}
fn default_max_machines() -> u32 {
    10
}
fn default_idle_timeout() -> u64 {
    300
}
fn default_fly_api_url() -> String {
    "https://api.machines.dev".into()
}

impl SwarmConfig {
    /// Load from `~/.config/raps/swarm.toml`, falling back to defaults.
    pub fn load() -> Result<Self> {
        let path = Self::default_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            toml::from_str(&content).with_context(|| format!("Failed to parse {}", path.display()))
        } else {
            Ok(Self::default())
        }
    }

    /// Default config file path.
    pub fn default_path() -> PathBuf {
        directories::ProjectDirs::from("xyz", "rapscli", "raps")
            .map(|dirs| dirs.config_dir().join("swarm.toml"))
            .unwrap_or_else(|| PathBuf::from("swarm.toml"))
    }
}

// ---------------------------------------------------------------------------
// Dispatch types
// ---------------------------------------------------------------------------

/// Receipt returned after dispatching a job to a Fly Machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchReceipt {
    pub machine_id: String,
    pub region: String,
    pub state: String,
    pub app: String,
}

/// Status of a Fly Machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineStatus {
    pub id: String,
    pub state: String,
    pub region: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// ServerlessDispatchAgent
// ---------------------------------------------------------------------------

/// A translate job request for serverless dispatch (decoupled from Redis job_queue types).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateJobRequest {
    pub urn: String,
    pub output_format: String,
    pub root_filename: Option<String>,
    pub region: Option<String>,
    pub force: bool,
}

/// Dispatches translation jobs to Fly.io ephemeral machines.
pub struct ServerlessDispatchAgent {
    client: reqwest::Client,
    config: ServerlessConfig,
}

impl ServerlessDispatchAgent {
    pub fn new(config: ServerlessConfig) -> Result<Self> {
        anyhow::ensure!(
            !config.fly_app.is_empty(),
            "fly_app must be set in swarm.toml [serverless]"
        );
        anyhow::ensure!(
            !config.fly_token.is_empty(),
            "fly_token must be set (swarm.toml or FLY_API_TOKEN env)"
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self { client, config })
    }

    /// Load config from swarm.toml, with env var overrides.
    pub fn from_config() -> Result<Self> {
        let mut swarm = SwarmConfig::load()?;

        // Env overrides: FLY_API_TOKEN > swarm.toml fly_token
        if let Ok(token) = std::env::var("FLY_API_TOKEN") {
            swarm.serverless.fly_token = token;
        }
        if let Ok(app) = std::env::var("FLY_APP") {
            swarm.serverless.fly_app = app;
        }

        Self::new(swarm.serverless)
    }

    /// Dispatch a translate job as an ephemeral Fly Machine.
    pub async fn dispatch_translate(&self, job: &TranslateJobRequest) -> Result<DispatchReceipt> {
        let url = format!(
            "{}/v1/apps/{}/machines",
            self.config.api_url, self.config.fly_app
        );

        let env_vars = serde_json::json!({
            "JOB_URN": job.urn,
            "JOB_FORMAT": job.output_format,
            "JOB_REGION": job.region.as_deref().unwrap_or("US"),
            "JOB_FORCE": job.force.to_string(),
        });

        let body = serde_json::json!({
            "region": self.config.preferred_region,
            "config": {
                "image": format!("rapscli/raps-worker:latest"),
                "size": self.config.machine_size,
                "env": env_vars,
                "auto_destroy": true,
                "restart": { "policy": "no" },
                "stop_config": {
                    "timeout": format!("{}s", self.config.idle_timeout_secs),
                },
            },
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.config.fly_token)
            .json(&body)
            .send()
            .await
            .context("Fly Machines API request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Fly API returned {}: {}", status, text);
        }

        let machine: serde_json::Value = resp.json().await.context("Invalid Fly API response")?;
        Ok(DispatchReceipt {
            machine_id: machine["id"].as_str().unwrap_or("unknown").to_string(),
            region: machine["region"]
                .as_str()
                .unwrap_or(&self.config.preferred_region)
                .to_string(),
            state: machine["state"].as_str().unwrap_or("created").to_string(),
            app: self.config.fly_app.clone(),
        })
    }

    /// Check machine status.
    pub async fn machine_status(&self, machine_id: &str) -> Result<MachineStatus> {
        let url = format!(
            "{}/v1/apps/{}/machines/{}",
            self.config.api_url, self.config.fly_app, machine_id,
        );

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.config.fly_token)
            .send()
            .await
            .context("Fly Machines API request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Fly API returned {}: {}", status, text);
        }

        resp.json::<MachineStatus>()
            .await
            .context("Failed to parse machine status")
    }

    /// List all machines for the configured app.
    pub async fn list_machines(&self) -> Result<Vec<MachineStatus>> {
        let url = format!(
            "{}/v1/apps/{}/machines",
            self.config.api_url, self.config.fly_app,
        );

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.config.fly_token)
            .send()
            .await
            .context("Fly Machines API request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Fly API returned {}: {}", status, text);
        }

        resp.json::<Vec<MachineStatus>>()
            .await
            .context("Failed to parse machines list")
    }

    /// Send a notification to a Slack webhook (if configured).
    pub async fn notify_slack(&self, message: &str) -> Result<()> {
        let Some(ref url) = self.config.notify_slack_url else {
            return Ok(());
        };

        self.client
            .post(url)
            .json(&serde_json::json!({ "text": message }))
            .send()
            .await
            .context("Slack notification failed")?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SwarmConfig::default();
        assert_eq!(config.serverless.preferred_region, "iad");
        assert_eq!(config.serverless.max_machines, 10);
        assert_eq!(config.redis.pool_size, 8);
        assert_eq!(config.worker.concurrency, 4);
    }

    #[test]
    fn test_parse_swarm_toml() {
        let toml_str = r#"
[serverless]
fly_app = "my-app"
fly_token = "test-token"
preferred_region = "lhr"
max_machines = 5

[redis]
url = "redis://myhost:6380"
pool_size = 16

[worker]
concurrency = 8
heartbeat_secs = 15
queues = ["critical", "normal"]
"#;
        let config: SwarmConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.serverless.fly_app, "my-app");
        assert_eq!(config.serverless.preferred_region, "lhr");
        assert_eq!(config.serverless.max_machines, 5);
        assert_eq!(config.redis.url, "redis://myhost:6380");
        assert_eq!(config.redis.pool_size, 16);
        assert_eq!(config.worker.concurrency, 8);
        assert_eq!(config.worker.queues.len(), 2);
    }
}

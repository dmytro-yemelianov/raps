// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CloudConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub master_encryption_key: String,
    #[serde(default = "default_jwt_expiry")]
    pub jwt_expiry_seconds: u64,
    #[serde(default = "default_refresh_expiry")]
    pub refresh_expiry_seconds: u64,
}

fn default_port() -> u16 {
    8080
}
fn default_jwt_expiry() -> u64 {
    900 // 15 minutes
}
fn default_refresh_expiry() -> u64 {
    604800 // 7 days
}

impl CloudConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();
        envy::from_env::<Self>().map_err(|e| anyhow::anyhow!("Config error: {e}"))
    }
}

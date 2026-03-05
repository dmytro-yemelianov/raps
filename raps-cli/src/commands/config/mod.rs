// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Configuration management commands
//!
//! Commands for managing profiles and configuration settings.

mod config_ops;
mod context;
mod profiles;

use anyhow::{Context, Result};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum ConfigCommands {
    /// Manage profiles (create, list, use, delete)
    #[command(subcommand)]
    Profile(ProfileCommands),

    /// Get a configuration value
    Get {
        /// Configuration key (e.g., client_id, base_url)
        key: String,
    },

    /// Set a configuration value for the active profile
    Set {
        /// Configuration key (e.g., client_id, base_url)
        key: String,
        /// Configuration value
        value: String,
    },

    /// Set or show the current working context (hub, project, account)
    #[command(subcommand)]
    Context(ContextCommands),

    /// Migrate tokens from plaintext file to OS keychain
    MigrateTokens,
}

#[derive(Debug, Subcommand)]
pub enum ContextCommands {
    /// Show current context settings
    Show,

    /// Set context value
    Set {
        /// Key to set (hub_id, project_id, account_id)
        key: String,
        /// Value to set (use "clear" to remove)
        value: String,
    },

    /// Clear all context values
    Clear,
}

#[derive(Debug, Subcommand)]
pub enum ProfileCommands {
    /// Create a new profile
    Create {
        /// Profile name
        name: String,
    },

    /// List all profiles
    List,

    /// Set the active profile
    Use {
        /// Profile name
        name: String,
    },

    /// Delete a profile
    Delete {
        /// Profile name
        name: String,
    },

    /// Show current active profile
    Current,

    /// Export profiles to a file
    Export {
        /// Output file path
        #[arg(long = "out-file", default_value = "profiles-export.json")]
        out_file: std::path::PathBuf,

        /// Include secrets (client_id, client_secret) - use with caution
        #[arg(long)]
        include_secrets: bool,

        /// Export specific profile (default: all)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Import profiles from a file
    Import {
        /// Input file path
        file: std::path::PathBuf,

        /// Overwrite existing profiles with same name
        #[arg(long)]
        overwrite: bool,
    },

    /// Compare two profiles
    Diff {
        /// First profile name
        profile1: String,
        /// Second profile name
        profile2: String,
    },
}

impl ConfigCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            ConfigCommands::Profile(cmd) => cmd.execute(output_format).await,
            ConfigCommands::Get { key } => config_ops::get_config(&key, output_format).await,
            ConfigCommands::Set { key, value } => {
                config_ops::set_config(&key, &value, output_format).await
            }
            ConfigCommands::Context(cmd) => cmd.execute(output_format).await,
            ConfigCommands::MigrateTokens => {
                raps_kernel::storage::TokenStorage::migrate_to_keychain()
            }
        }
    }
}

impl ContextCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            ContextCommands::Show => context::show_context(output_format).await,
            ContextCommands::Set { key, value } => {
                context::set_context(&key, &value, output_format).await
            }
            ContextCommands::Clear => context::clear_context(output_format).await,
        }
    }
}

impl ProfileCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            ProfileCommands::Create { name } => {
                profiles::create_profile(&name, output_format).await
            }
            ProfileCommands::List => profiles::list_profiles(output_format).await,
            ProfileCommands::Use { name } => profiles::use_profile(&name, output_format).await,
            ProfileCommands::Delete { name } => {
                profiles::delete_profile(&name, output_format).await
            }
            ProfileCommands::Current => profiles::show_current_profile(output_format).await,
            ProfileCommands::Export {
                out_file,
                include_secrets,
                name,
            } => profiles::export_profiles(&out_file, include_secrets, name, output_format).await,
            ProfileCommands::Import { file, overwrite } => {
                profiles::import_profiles(&file, overwrite, output_format).await
            }
            ProfileCommands::Diff { profile1, profile2 } => {
                profiles::diff_profiles(&profile1, &profile2, output_format).await
            }
        }
    }
}

/// Profile configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ProfileConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub base_url: Option<String>,
    pub callback_url: Option<String>,
    pub da_nickname: Option<String>,
    pub use_keychain: Option<bool>,
    /// Sticky context: default hub ID for commands that need it
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_hub_id: Option<String>,
    /// Sticky context: default project ID for commands that need it
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_project_id: Option<String>,
    /// Sticky context: default account ID for admin commands
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_account_id: Option<String>,
}

/// Profiles storage structure
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub(crate) struct ProfilesData {
    pub active_profile: Option<String>,
    pub profiles: HashMap<String, ProfileConfig>,
}

async fn profiles_path() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
        .context("Failed to get project directories")?;
    let config_dir = proj_dirs.config_dir();
    tokio::fs::create_dir_all(config_dir).await?;
    Ok(config_dir.join("profiles.json"))
}

pub(crate) async fn load_profiles() -> Result<ProfilesData> {
    let path = profiles_path().await?;
    if !path.exists() {
        return Ok(ProfilesData {
            active_profile: None,
            profiles: HashMap::new(),
        });
    }

    let content = tokio::fs::read_to_string(&path).await?;
    let data: ProfilesData =
        serde_json::from_str(&content).context("Failed to parse profiles.json")?;
    Ok(data)
}

pub(crate) async fn save_profiles(data: &ProfilesData) -> Result<()> {
    let path = profiles_path().await?;
    let content = serde_json::to_string_pretty(data)?;
    tokio::fs::write(&path, content).await?;
    Ok(())
}

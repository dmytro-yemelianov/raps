//! Configuration management commands
//!
//! Commands for managing profiles and configuration settings.

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
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
}

impl ConfigCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            ConfigCommands::Profile(cmd) => cmd.execute(output_format).await,
            ConfigCommands::Get { key } => get_config(&key, output_format).await,
            ConfigCommands::Set { key, value } => set_config(&key, &value, output_format).await,
        }
    }
}

impl ProfileCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            ProfileCommands::Create { name } => create_profile(&name, output_format).await,
            ProfileCommands::List => list_profiles(output_format).await,
            ProfileCommands::Use { name } => use_profile(&name, output_format).await,
            ProfileCommands::Delete { name } => delete_profile(&name, output_format).await,
            ProfileCommands::Current => show_current_profile(output_format).await,
        }
    }
}

/// Profile configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub base_url: Option<String>,
    pub callback_url: Option<String>,
    pub da_nickname: Option<String>,
}

/// Profiles storage structure
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ProfilesData {
    pub active_profile: Option<String>,
    pub profiles: HashMap<String, ProfileConfig>,
}

fn profiles_path() -> Result<PathBuf> {
    let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
        .context("Failed to get project directories")?;
    let config_dir = proj_dirs.config_dir();
    std::fs::create_dir_all(config_dir)?;
    Ok(config_dir.join("profiles.json"))
}

pub(crate) fn load_profiles() -> Result<ProfilesData> {
    let path = profiles_path()?;
    if !path.exists() {
        return Ok(ProfilesData {
            active_profile: None,
            profiles: HashMap::new(),
        });
    }

    let content = std::fs::read_to_string(&path)?;
    let data: ProfilesData =
        serde_json::from_str(&content).context("Failed to parse profiles.json")?;
    Ok(data)
}

fn save_profiles(data: &ProfilesData) -> Result<()> {
    let path = profiles_path()?;
    let content = serde_json::to_string_pretty(data)?;
    std::fs::write(&path, content)?;
    Ok(())
}

async fn create_profile(name: &str, output_format: OutputFormat) -> Result<()> {
    let mut data = load_profiles()?;

    if data.profiles.contains_key(name) {
        let msg = format!("Profile '{}' already exists", name);
        match output_format {
            OutputFormat::Table => {
                eprintln!("{}", msg.yellow());
            }
            _ => {
                #[derive(Serialize)]
                struct ErrorOutput {
                    error: String,
                }
                output_format.write(&ErrorOutput { error: msg })?;
            }
        }
        return Ok(());
    }

    data.profiles.insert(
        name.to_string(),
        ProfileConfig {
            client_id: None,
            client_secret: None,
            base_url: None,
            callback_url: None,
            da_nickname: None,
        },
    );

    save_profiles(&data)?;

    #[derive(Serialize)]
    struct CreateProfileOutput {
        success: bool,
        profile: String,
        message: String,
    }

    let output = CreateProfileOutput {
        success: true,
        profile: name.to_string(),
        message: format!("Profile '{}' created successfully", name),
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

async fn list_profiles(output_format: OutputFormat) -> Result<()> {
    let data = load_profiles()?;

    #[derive(Serialize)]
    struct ProfileInfo {
        name: String,
        active: bool,
    }

    let mut profiles: Vec<ProfileInfo> = data
        .profiles
        .keys()
        .map(|name| ProfileInfo {
            name: name.clone(),
            active: data.active_profile.as_ref() == Some(name),
        })
        .collect();

    profiles.sort_by(|a, b| match (a.active, b.active) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    match output_format {
        OutputFormat::Table => {
            if profiles.is_empty() {
                println!("No profiles found. Create one with 'raps config profile create <name>'");
            } else {
                println!("{}", "Profiles:".bold());
                for profile in &profiles {
                    let marker = if profile.active { "→" } else { " " };
                    let name = if profile.active {
                        profile.name.cyan().bold()
                    } else {
                        profile.name.normal()
                    };
                    println!("  {} {}", marker.green(), name);
                }
            }
        }
        _ => {
            output_format.write(&profiles)?;
        }
    }

    Ok(())
}

async fn use_profile(name: &str, output_format: OutputFormat) -> Result<()> {
    let mut data = load_profiles()?;

    if !data.profiles.contains_key(name) {
        anyhow::bail!(
            "Profile '{}' does not exist. Create it first with 'raps config profile create {}'",
            name,
            name
        );
    }

    data.active_profile = Some(name.to_string());
    save_profiles(&data)?;

    #[derive(Serialize)]
    struct UseProfileOutput {
        success: bool,
        profile: String,
        message: String,
    }

    let output = UseProfileOutput {
        success: true,
        profile: name.to_string(),
        message: format!("Switched to profile '{}'", name),
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

async fn delete_profile(name: &str, output_format: OutputFormat) -> Result<()> {
    let mut data = load_profiles()?;

    if !data.profiles.contains_key(name) {
        anyhow::bail!("Profile '{}' does not exist", name);
    }

    // If deleting active profile, clear it
    if data.active_profile.as_ref().is_some_and(|active| active == name) {
        data.active_profile = None;
    }

    data.profiles.remove(name);
    save_profiles(&data)?;

    #[derive(Serialize)]
    struct DeleteProfileOutput {
        success: bool,
        profile: String,
        message: String,
    }

    let output = DeleteProfileOutput {
        success: true,
        profile: name.to_string(),
        message: format!("Profile '{}' deleted successfully", name),
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

async fn show_current_profile(output_format: OutputFormat) -> Result<()> {
    let data = load_profiles()?;

    #[derive(Serialize)]
    struct CurrentProfileOutput {
        active_profile: Option<String>,
    }

    let output = CurrentProfileOutput {
        active_profile: data.active_profile.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            if let Some(profile) = &data.active_profile {
                println!("Active profile: {}", profile.cyan().bold());
            } else {
                println!("No active profile. Using environment variables or defaults.");
                println!("Set one with 'raps config profile use <name>'");
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

async fn get_config(key: &str, output_format: OutputFormat) -> Result<()> {
    let data = load_profiles()?;

    let value = if let Some(profile_name) = &data.active_profile {
        if let Some(profile) = data.profiles.get(profile_name) {
            match key {
                "client_id" => profile.client_id.as_ref(),
                "client_secret" => profile.client_secret.as_ref(),
                "base_url" => profile.base_url.as_ref(),
                "callback_url" => profile.callback_url.as_ref(),
                "da_nickname" => profile.da_nickname.as_ref(),
                _ => {
                    anyhow::bail!("Unknown configuration key: {}. Valid keys: client_id, client_secret, base_url, callback_url, da_nickname", key);
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    #[derive(Serialize)]
    struct GetConfigOutput {
        key: String,
        value: Option<String>,
        source: String,
    }

    let source = if data.active_profile.is_some() {
        format!("profile:{}", data.active_profile.as_ref().unwrap())
    } else {
        "environment".to_string()
    };

    let output = GetConfigOutput {
        key: key.to_string(),
        value: value.cloned(),
        source: source.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            if let Some(v) = value {
                println!("{} = {}", key.bold(), v);
                println!("  (from {})", source.dimmed());
            } else {
                println!("{} = (not set)", key.bold());
                println!("  (from {})", source.dimmed());
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

async fn set_config(key: &str, value: &str, output_format: OutputFormat) -> Result<()> {
    let mut data = load_profiles()?;

    let profile_name = data.active_profile.clone()
        .ok_or_else(|| anyhow::anyhow!("No active profile. Create and activate one first with 'raps config profile create <name>' and 'raps config profile use <name>'"))?;

    let profile = data
        .profiles
        .get_mut(&profile_name)
        .ok_or_else(|| anyhow::anyhow!("Active profile '{}' not found", profile_name))?;

    match key {
        "client_id" => profile.client_id = Some(value.to_string()),
        "client_secret" => profile.client_secret = Some(value.to_string()),
        "base_url" => profile.base_url = Some(value.to_string()),
        "callback_url" => profile.callback_url = Some(value.to_string()),
        "da_nickname" => profile.da_nickname = Some(value.to_string()),
        _ => {
            anyhow::bail!("Unknown configuration key: {}. Valid keys: client_id, client_secret, base_url, callback_url, da_nickname", key);
        }
    }

    save_profiles(&data)?;

    #[derive(Serialize)]
    struct SetConfigOutput {
        success: bool,
        key: String,
        value: String,
        profile: String,
    }

    let output = SetConfigOutput {
        success: true,
        key: key.to_string(),
        value: value.to_string(),
        profile: profile_name.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Set {} = {} in profile '{}'",
                "✓".green().bold(),
                key.bold(),
                value,
                profile_name
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

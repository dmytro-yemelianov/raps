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

    /// Export profiles to a file
    Export {
        /// Output file path
        #[arg(short, long, default_value = "profiles-export.json")]
        output: std::path::PathBuf,

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
            ProfileCommands::Export {
                output,
                include_secrets,
                name,
            } => export_profiles(&output, include_secrets, name, output_format).await,
            ProfileCommands::Import { file, overwrite } => {
                import_profiles(&file, overwrite, output_format).await
            }
            ProfileCommands::Diff { profile1, profile2 } => {
                diff_profiles(&profile1, &profile2, output_format).await
            }
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    if data
        .active_profile
        .as_ref()
        .is_some_and(|active| active == name)
    {
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

async fn export_profiles(
    output_path: &std::path::Path,
    include_secrets: bool,
    name_filter: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let data = load_profiles()?;

    // Filter profiles if name specified
    let mut export_data = if let Some(ref name) = name_filter {
        let mut filtered = ProfilesData {
            active_profile: data.active_profile.clone(),
            profiles: std::collections::HashMap::new(),
        };
        if let Some(profile) = data.profiles.get(name) {
            filtered.profiles.insert(name.clone(), profile.clone());
        } else {
            anyhow::bail!("Profile '{}' not found", name);
        }
        filtered
    } else {
        data.clone()
    };

    // Redact secrets if not including them
    if !include_secrets {
        for profile in export_data.profiles.values_mut() {
            if profile.client_id.is_some() {
                profile.client_id = Some("***REDACTED***".to_string());
            }
            if profile.client_secret.is_some() {
                profile.client_secret = Some("***REDACTED***".to_string());
            }
        }
    }

    let content = serde_json::to_string_pretty(&export_data)?;
    std::fs::write(output_path, &content)?;

    #[derive(Serialize)]
    struct ExportOutput {
        success: bool,
        path: String,
        profiles_count: usize,
        include_secrets: bool,
    }

    let output = ExportOutput {
        success: true,
        path: output_path.display().to_string(),
        profiles_count: export_data.profiles.len(),
        include_secrets,
    };

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Exported {} profile(s) to {}",
                "✓".green().bold(),
                output.profiles_count,
                output.path.cyan()
            );
            if !include_secrets {
                println!(
                    "  {} Secrets were redacted. Use --include-secrets to export credentials.",
                    "!".yellow()
                );
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

async fn import_profiles(
    file_path: &std::path::Path,
    overwrite: bool,
    output_format: OutputFormat,
) -> Result<()> {
    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read import file: {}", file_path.display()))?;

    let import_data: ProfilesData = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse import file: {}", file_path.display()))?;

    let mut data = load_profiles()?;
    let mut imported = 0;
    let mut skipped = 0;

    for (name, profile) in import_data.profiles {
        if data.profiles.contains_key(&name) && !overwrite {
            if output_format.supports_colors() {
                println!(
                    "  {} Profile '{}' already exists, skipping",
                    "→".yellow(),
                    name
                );
            }
            skipped += 1;
            continue;
        }
        data.profiles.insert(name, profile);
        imported += 1;
    }

    save_profiles(&data)?;

    #[derive(Serialize)]
    struct ImportOutput {
        success: bool,
        imported: usize,
        skipped: usize,
    }

    let output = ImportOutput {
        success: true,
        imported,
        skipped,
    };

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Imported {} profile(s)",
                "✓".green().bold(),
                output.imported
            );
            if skipped > 0 {
                println!(
                    "  {} {} profile(s) skipped (use --overwrite to replace)",
                    "→".yellow(),
                    skipped
                );
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

async fn diff_profiles(profile1: &str, profile2: &str, output_format: OutputFormat) -> Result<()> {
    let data = load_profiles()?;

    let p1 = data
        .profiles
        .get(profile1)
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", profile1))?;
    let p2 = data
        .profiles
        .get(profile2)
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", profile2))?;

    #[derive(Serialize)]
    struct DiffItem {
        key: String,
        value1: Option<String>,
        value2: Option<String>,
        different: bool,
    }

    let mut diffs = Vec::new();

    // Compare fields
    let fields = [
        ("client_id", p1.client_id.as_ref(), p2.client_id.as_ref()),
        ("base_url", p1.base_url.as_ref(), p2.base_url.as_ref()),
    ];

    for (key, v1, v2) in fields {
        let redact = |v: Option<&String>| {
            v.map(|s| {
                if key == "client_id" {
                    format!("{}...", &s[..8.min(s.len())])
                } else {
                    s.clone()
                }
            })
        };

        diffs.push(DiffItem {
            key: key.to_string(),
            value1: redact(v1),
            value2: redact(v2),
            different: v1 != v2,
        });
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Profile Comparison:".bold());
            println!("{}", "─".repeat(70));
            println!(
                "{:<15} {:<25} {:<25} {}",
                "Key".bold(),
                profile1.cyan().bold(),
                profile2.cyan().bold(),
                "".bold()
            );
            println!("{}", "─".repeat(70));

            for diff in &diffs {
                let v1 = diff.value1.as_deref().unwrap_or("-");
                let v2 = diff.value2.as_deref().unwrap_or("-");
                let marker = if diff.different {
                    "≠".red().to_string()
                } else {
                    "=".green().to_string()
                };
                println!("{:<15} {:<25} {:<25} {}", diff.key, v1, v2, marker);
            }

            println!("{}", "─".repeat(70));
        }
        _ => {
            output_format.write(&diffs)?;
        }
    }

    Ok(())
}

async fn get_config(key: &str, output_format: OutputFormat) -> Result<()> {
    let data = load_profiles()?;

    // Handle use_keychain separately since it's an environment variable, not a profile setting
    let (value, source) = if key == "use_keychain" {
        let env_value = std::env::var("RAPS_USE_KEYCHAIN")
            .ok()
            .filter(|v| v.to_lowercase() == "true" || v == "1" || v.to_lowercase() == "yes");
        (
            env_value.as_ref().map(|_| "true"),
            "environment".to_string(),
        )
    } else {
        let value = if let Some(profile_name) = &data.active_profile {
            if let Some(profile) = data.profiles.get(profile_name) {
                match key {
                    "client_id" => profile.client_id.as_deref(),
                    "client_secret" => profile.client_secret.as_deref(),
                    "base_url" => profile.base_url.as_deref(),
                    "callback_url" => profile.callback_url.as_deref(),
                    "da_nickname" => profile.da_nickname.as_deref(),
                    _ => {
                        anyhow::bail!("Unknown configuration key: {}. Valid keys: client_id, client_secret, base_url, callback_url, da_nickname, use_keychain", key);
                    }
                }
            } else {
                None
            }
        } else {
            None
        };
        let source = if data.active_profile.is_some() {
            format!("profile:{}", data.active_profile.as_ref().unwrap())
        } else {
            "environment".to_string()
        };
        (value, source)
    };

    #[derive(Serialize)]
    struct GetConfigOutput {
        key: String,
        value: Option<String>,
        source: String,
    }

    let output = GetConfigOutput {
        key: key.to_string(),
        value: value.map(|s| s.to_string()),
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
        "use_keychain" => {
            // Set environment variable for keychain usage
            if value.to_lowercase() == "true" || value == "1" || value.to_lowercase() == "yes" {
                std::env::set_var("RAPS_USE_KEYCHAIN", "true");
            } else {
                std::env::remove_var("RAPS_USE_KEYCHAIN");
            }
        }
        _ => {
            anyhow::bail!("Unknown configuration key: {}. Valid keys: client_id, client_secret, base_url, callback_url, da_nickname, use_keychain", key);
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

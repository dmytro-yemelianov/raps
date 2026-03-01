// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Profile management command implementations

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;

use super::{ProfileConfig, ProfilesData, load_profiles, save_profiles};

pub(super) async fn create_profile(name: &str, output_format: OutputFormat) -> Result<()> {
    let mut data = load_profiles().await?;

    if data.profiles.contains_key(name) {
        let msg = format!("Profile '{}' already exists", name);
        match output_format {
            OutputFormat::Table => {
                eprintln!("{}", msg.yellow());
            }
            _ => {
                #[derive(Serialize, schemars::JsonSchema)]
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
            use_keychain: None,
            context_hub_id: None,
            context_project_id: None,
            context_account_id: None,
        },
    );

    save_profiles(&data).await?;

    #[derive(Serialize, schemars::JsonSchema)]
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
            println!("{} {}", "\u{2713}".green().bold(), output.message);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

pub(super) async fn list_profiles(output_format: OutputFormat) -> Result<()> {
    let data = load_profiles().await?;

    #[derive(Serialize, schemars::JsonSchema)]
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
                    let marker = if profile.active { "\u{2192}" } else { " " };
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

pub(super) async fn use_profile(name: &str, output_format: OutputFormat) -> Result<()> {
    let mut data = load_profiles().await?;

    if !data.profiles.contains_key(name) {
        anyhow::bail!(
            "Profile '{}' does not exist. Create it first with 'raps config profile create {}'",
            name,
            name
        );
    }

    data.active_profile = Some(name.to_string());
    save_profiles(&data).await?;

    #[derive(Serialize, schemars::JsonSchema)]
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
            println!("{} {}", "\u{2713}".green().bold(), output.message);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

pub(super) async fn delete_profile(name: &str, output_format: OutputFormat) -> Result<()> {
    let mut data = load_profiles().await?;

    if !data.profiles.contains_key(name) {
        anyhow::bail!("Profile '{name}' does not exist");
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
    save_profiles(&data).await?;

    #[derive(Serialize, schemars::JsonSchema)]
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
            println!("{} {}", "\u{2713}".green().bold(), output.message);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

pub(super) async fn show_current_profile(output_format: OutputFormat) -> Result<()> {
    let data = load_profiles().await?;

    #[derive(Serialize, schemars::JsonSchema)]
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

pub(super) async fn export_profiles(
    output_path: &std::path::Path,
    include_secrets: bool,
    name_filter: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let data = load_profiles().await?;

    // Filter profiles if name specified
    let mut export_data = if let Some(ref name) = name_filter {
        let mut filtered = ProfilesData {
            active_profile: data.active_profile.clone(),
            profiles: std::collections::HashMap::new(),
        };
        if let Some(profile) = data.profiles.get(name) {
            filtered.profiles.insert(name.clone(), profile.clone());
        } else {
            anyhow::bail!("Profile '{name}' not found");
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

    #[derive(Serialize, schemars::JsonSchema)]
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
                "\u{2713}".green().bold(),
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

pub(super) async fn import_profiles(
    file_path: &std::path::Path,
    overwrite: bool,
    output_format: OutputFormat,
) -> Result<()> {
    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read import file: {}", file_path.display()))?;

    let import_data: ProfilesData = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse import file: {}", file_path.display()))?;

    let mut data = load_profiles().await?;
    let mut imported = 0;
    let mut skipped = 0;

    for (name, profile) in import_data.profiles {
        if data.profiles.contains_key(&name) && !overwrite {
            if output_format.supports_colors() {
                println!(
                    "  {} Profile '{}' already exists, skipping",
                    "\u{2192}".yellow(),
                    name
                );
            }
            skipped += 1;
            continue;
        }
        data.profiles.insert(name, profile);
        imported += 1;
    }

    save_profiles(&data).await?;

    #[derive(Serialize, schemars::JsonSchema)]
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
                "\u{2713}".green().bold(),
                output.imported
            );
            if skipped > 0 {
                println!(
                    "  {} {} profile(s) skipped (use --overwrite to replace)",
                    "\u{2192}".yellow(),
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

pub(super) async fn diff_profiles(
    profile1: &str,
    profile2: &str,
    output_format: OutputFormat,
) -> Result<()> {
    let data = load_profiles().await?;

    let p1 = data
        .profiles
        .get(profile1)
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", profile1))?;
    let p2 = data
        .profiles
        .get(profile2)
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", profile2))?;

    #[derive(Serialize, schemars::JsonSchema)]
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
            println!("{}", "\u{2500}".repeat(70));
            println!(
                "{:<15} {:<25} {:<25} {}",
                "Key".bold(),
                profile1.cyan().bold(),
                profile2.cyan().bold(),
                "".bold()
            );
            println!("{}", "\u{2500}".repeat(70));

            for diff in &diffs {
                let v1 = diff.value1.as_deref().unwrap_or("-");
                let v2 = diff.value2.as_deref().unwrap_or("-");
                let marker = if diff.different {
                    "\u{2260}".red().to_string()
                } else {
                    "=".green().to_string()
                };
                println!("{:<15} {:<25} {:<25} {}", diff.key, v1, v2, marker);
            }

            println!("{}", "\u{2500}".repeat(70));
        }
        _ => {
            output_format.write(&diffs)?;
        }
    }

    Ok(())
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Get/set configuration value command implementations

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;

use super::{load_profiles, save_profiles};

pub(super) async fn get_config(key: &str, output_format: OutputFormat) -> Result<()> {
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
                        anyhow::bail!(
                            "Unknown configuration key: {}. Valid keys: client_id, client_secret, base_url, callback_url, da_nickname, use_keychain",
                            key
                        );
                    }
                }
            } else {
                None
            }
        } else {
            None
        };
        let source = if let Some(ref profile) = data.active_profile {
            format!("profile:{}", profile)
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

pub(super) async fn set_config(
    key: &str,
    value: &str,
    output_format: OutputFormat,
) -> Result<()> {
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
            // Store keychain preference in profile configuration
            let use_keychain = matches!(value.to_lowercase().as_str(), "true" | "1" | "yes" | "on");
            profile.use_keychain = Some(use_keychain);

            // Provide immediate feedback about security implications
            if !use_keychain {
                eprintln!(
                    "\u{26a0}\u{fe0f}  WARNING: Disabling keychain storage will store tokens in plaintext."
                );
                eprintln!("\u{26a0}\u{fe0f}  This is less secure than using the OS keychain.");
            } else {
                println!("\u{2705} Keychain storage enabled for secure token management.");
            }
        }
        _ => {
            anyhow::bail!(
                "Unknown configuration key: {}. Valid keys: client_id, client_secret, base_url, callback_url, da_nickname, use_keychain",
                key
            );
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
                "\u{2713}".green().bold(),
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

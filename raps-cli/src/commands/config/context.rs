// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Context management command implementations

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;

use super::{load_profiles, save_profiles};

pub(super) async fn show_context(output_format: OutputFormat) -> Result<()> {
    let data = load_profiles()?;

    #[derive(Serialize)]
    struct ContextItem {
        key: String,
        value: Option<String>,
        source: String,
    }

    let mut items = Vec::new();

    // Check hub_id
    let env_hub = std::env::var("APS_HUB_ID").ok();
    let profile_hub = data
        .active_profile
        .as_ref()
        .and_then(|name| data.profiles.get(name))
        .and_then(|p| p.context_hub_id.clone());
    items.push(ContextItem {
        key: "hub_id".to_string(),
        value: env_hub.clone().or(profile_hub.clone()),
        source: if env_hub.is_some() {
            "env:APS_HUB_ID".to_string()
        } else if profile_hub.is_some() {
            format!(
                "profile:{}",
                data.active_profile.as_deref().unwrap_or("unknown")
            )
        } else {
            "(not set)".to_string()
        },
    });

    // Check project_id
    let env_proj = std::env::var("APS_PROJECT_ID").ok();
    let profile_proj = data
        .active_profile
        .as_ref()
        .and_then(|name| data.profiles.get(name))
        .and_then(|p| p.context_project_id.clone());
    items.push(ContextItem {
        key: "project_id".to_string(),
        value: env_proj.clone().or(profile_proj.clone()),
        source: if env_proj.is_some() {
            "env:APS_PROJECT_ID".to_string()
        } else if profile_proj.is_some() {
            format!(
                "profile:{}",
                data.active_profile.as_deref().unwrap_or("unknown")
            )
        } else {
            "(not set)".to_string()
        },
    });

    // Check account_id
    let env_acc = std::env::var("APS_ACCOUNT_ID").ok();
    let profile_acc = data
        .active_profile
        .as_ref()
        .and_then(|name| data.profiles.get(name))
        .and_then(|p| p.context_account_id.clone());
    items.push(ContextItem {
        key: "account_id".to_string(),
        value: env_acc.clone().or(profile_acc.clone()),
        source: if env_acc.is_some() {
            "env:APS_ACCOUNT_ID".to_string()
        } else if profile_acc.is_some() {
            format!(
                "profile:{}",
                data.active_profile.as_deref().unwrap_or("unknown")
            )
        } else {
            "(not set)".to_string()
        },
    });

    match output_format {
        OutputFormat::Table => {
            println!("{}", "Current Context:".bold());
            println!("{}", "-".repeat(60));
            for item in &items {
                let val = item.value.as_deref().unwrap_or("(not set)").to_string();
                let display_val = if item.value.is_some() {
                    val.cyan().to_string()
                } else {
                    val.dimmed().to_string()
                };
                println!(
                    "  {:<15} {:<30} {}",
                    item.key.bold(),
                    display_val,
                    item.source.dimmed()
                );
            }
        }
        _ => {
            output_format.write(&items)?;
        }
    }

    Ok(())
}

pub(super) async fn set_context(
    key: &str,
    value: &str,
    output_format: OutputFormat,
) -> Result<()> {
    let mut data = load_profiles()?;

    let profile_name = data.active_profile.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "No active profile. Create and activate one first with 'raps config profile create <name>' and 'raps config profile use <name>'"
        )
    })?;

    let profile = data
        .profiles
        .get_mut(&profile_name)
        .ok_or_else(|| anyhow::anyhow!("Active profile '{}' not found", profile_name))?;

    let clear = value == "clear";

    match key {
        "hub_id" => {
            profile.context_hub_id = if clear { None } else { Some(value.to_string()) };
        }
        "project_id" => {
            profile.context_project_id = if clear { None } else { Some(value.to_string()) };
        }
        "account_id" => {
            profile.context_account_id = if clear { None } else { Some(value.to_string()) };
        }
        _ => {
            anyhow::bail!(
                "Unknown context key: {}. Valid keys: hub_id, project_id, account_id",
                key
            );
        }
    }

    save_profiles(&data)?;

    #[derive(Serialize)]
    struct SetContextOutput {
        success: bool,
        key: String,
        value: Option<String>,
        profile: String,
    }

    let output = SetContextOutput {
        success: true,
        key: key.to_string(),
        value: if clear { None } else { Some(value.to_string()) },
        profile: profile_name.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            if clear {
                println!(
                    "{} Cleared context {} in profile '{}'",
                    "\u{2713}".green().bold(),
                    key.bold(),
                    profile_name
                );
            } else {
                println!(
                    "{} Set context {} = {} in profile '{}'",
                    "\u{2713}".green().bold(),
                    key.bold(),
                    value,
                    profile_name
                );
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

pub(super) async fn clear_context(output_format: OutputFormat) -> Result<()> {
    let mut data = load_profiles()?;

    let profile_name = data.active_profile.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "No active profile. Create and activate one first with 'raps config profile create <name>' and 'raps config profile use <name>'"
        )
    })?;

    let profile = data
        .profiles
        .get_mut(&profile_name)
        .ok_or_else(|| anyhow::anyhow!("Active profile '{}' not found", profile_name))?;

    profile.context_hub_id = None;
    profile.context_project_id = None;
    profile.context_account_id = None;

    save_profiles(&data)?;

    #[derive(Serialize)]
    struct ClearContextOutput {
        success: bool,
        profile: String,
        message: String,
    }

    let output = ClearContextOutput {
        success: true,
        profile: profile_name.clone(),
        message: format!("All context values cleared in profile '{}'", profile_name),
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

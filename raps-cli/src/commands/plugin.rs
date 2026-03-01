// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Plugin Management Commands
//!
//! Commands for managing RAPS plugins, hooks, and aliases.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use crate::plugins::{PluginConfig, PluginEntry, PluginManager};
// use raps_kernel::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum PluginCommands {
    /// List discovered and configured plugins
    List,

    /// Enable a plugin
    Enable {
        /// Plugin name
        name: String,
    },

    /// Disable a plugin
    Disable {
        /// Plugin name
        name: String,
    },

    /// Trust a plugin by recording its current SHA-256 hash
    Trust {
        /// Plugin name
        name: String,
    },

    /// Verify a plugin's integrity (hash and optional signature)
    Verify {
        /// Plugin name
        name: String,
    },

    /// Show detailed info about a plugin
    Info {
        /// Plugin name
        name: String,
    },

    /// Manage command aliases
    #[command(subcommand)]
    Alias(AliasCommands),
}

#[derive(Debug, Subcommand)]
pub enum AliasCommands {
    /// List all configured aliases
    List,

    /// Add a new alias
    Add {
        /// Alias name
        name: String,
        /// Command to alias (e.g., "object upload --resume")
        command: String,
    },

    /// Remove an alias
    Remove {
        /// Alias name
        name: String,
    },
}

impl PluginCommands {
    pub fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            PluginCommands::List => list_plugins(output_format),
            PluginCommands::Enable { name } => enable_plugin(&name, output_format),
            PluginCommands::Disable { name } => disable_plugin(&name, output_format),
            PluginCommands::Info { name } => plugin_info(&name, output_format),
            PluginCommands::Trust { name } => trust_plugin(&name, output_format),
            PluginCommands::Verify { name } => verify_plugin(&name, output_format),
            PluginCommands::Alias(cmd) => cmd.execute(output_format),
        }
    }
}

impl AliasCommands {
    pub fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            AliasCommands::List => list_aliases(output_format),
            AliasCommands::Add { name, command } => add_alias(&name, &command, output_format),
            AliasCommands::Remove { name } => remove_alias(&name, output_format),
        }
    }
}

#[derive(Serialize, schemars::JsonSchema)]
struct PluginOutput {
    name: String,
    path: String,
    enabled: bool,
    description: Option<String>,
}

fn list_plugins(output_format: OutputFormat) -> Result<()> {
    let manager = PluginManager::default();
    let plugins = manager.list_plugins();

    let outputs: Vec<PluginOutput> = plugins
        .iter()
        .map(|p| PluginOutput {
            name: p.name.clone(),
            path: p.path.to_string_lossy().to_string(),
            enabled: p.enabled,
            description: None,
        })
        .collect();

    match output_format {
        OutputFormat::Table => {
            if outputs.is_empty() {
                println!("{}", "No plugins discovered.".yellow());
                println!("\n{}", "To create a plugin:".dimmed());
                println!(
                    "  1. Create an executable named {} in your PATH",
                    "raps-<name>".cyan()
                );
                println!("  2. Run {} to see it listed", "raps plugin list".cyan());
            } else {
                println!("\n{}", "Discovered Plugins:".bold());
                println!("{}", "─".repeat(80));
                println!(
                    "  {:<20} {:<45} {}",
                    "Name".bold(),
                    "Path".bold(),
                    "Status".bold()
                );
                println!("{}", "─".repeat(80));

                for plugin in &outputs {
                    let status = if plugin.enabled {
                        "✓ enabled".green().to_string()
                    } else {
                        "✗ disabled".red().to_string()
                    };
                    println!(
                        "  {:<20} {:<45} {}",
                        plugin.name.cyan(),
                        truncate_str(&plugin.path, 45),
                        status
                    );
                }

                println!("{}", "─".repeat(80));
                println!("{} {} plugin(s) found", "→".cyan(), outputs.len());
            }
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

fn enable_plugin(name: &str, output_format: OutputFormat) -> Result<()> {
    let mut config = PluginConfig::load()?;

    // Update or create the plugin entry
    if let Some(entry) = config.plugins.get_mut(name) {
        entry.enabled = true;
    } else {
        config.plugins.insert(
            name.to_string(),
            PluginEntry {
                enabled: true,
                path: None,
                description: None,
                sha256: None,
                public_key: None,
                signature: None,
                trusted: false,
            },
        );
    }

    config.save()?;

    match output_format {
        OutputFormat::Table => {
            println!("{} Plugin '{}' enabled", "✓".green().bold(), name.cyan());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "plugin": name,
                "enabled": true
            }))?;
        }
    }

    Ok(())
}

fn disable_plugin(name: &str, output_format: OutputFormat) -> Result<()> {
    let mut config = PluginConfig::load()?;

    // Update or create the plugin entry
    if let Some(entry) = config.plugins.get_mut(name) {
        entry.enabled = false;
    } else {
        config.plugins.insert(
            name.to_string(),
            PluginEntry {
                enabled: false,
                path: None,
                description: None,
                sha256: None,
                public_key: None,
                signature: None,
                trusted: false,
            },
        );
    }

    config.save()?;

    match output_format {
        OutputFormat::Table => {
            println!("{} Plugin '{}' disabled", "✓".green().bold(), name.cyan());
        }
        _ => {
            output_format.write(&serde_json::json!({
                "plugin": name,
                "enabled": false
            }))?;
        }
    }

    Ok(())
}

#[derive(Serialize, schemars::JsonSchema)]
struct AliasOutput {
    name: String,
    command: String,
}

fn list_aliases(output_format: OutputFormat) -> Result<()> {
    let config = PluginConfig::load()?;

    let outputs: Vec<AliasOutput> = config
        .aliases
        .iter()
        .map(|(name, cmd)| AliasOutput {
            name: name.clone(),
            command: cmd.clone(),
        })
        .collect();

    match output_format {
        OutputFormat::Table => {
            if outputs.is_empty() {
                println!("{}", "No aliases configured.".yellow());
                println!("\n{}", "To add an alias:".dimmed());
                println!("  {}", "raps plugin alias add <name> \"<command>\"".cyan());
                println!("\n{}", "Example:".dimmed());
                println!(
                    "  {}",
                    "raps plugin alias add up \"object upload --resume\"".cyan()
                );
            } else {
                println!("\n{}", "Configured Aliases:".bold());
                println!("{}", "─".repeat(70));
                println!("  {:<15} {}", "Alias".bold(), "Command".bold());
                println!("{}", "─".repeat(70));

                for alias in &outputs {
                    println!("  {:<15} {}", alias.name.cyan(), alias.command);
                }

                println!("{}", "─".repeat(70));
                println!("{} {} alias(es) configured", "→".cyan(), outputs.len());
            }
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

fn add_alias(name: &str, command: &str, output_format: OutputFormat) -> Result<()> {
    let mut config = PluginConfig::load()?;
    config.aliases.insert(name.to_string(), command.to_string());
    config.save()?;

    match output_format {
        OutputFormat::Table => {
            println!("{} Alias '{}' added", "✓".green().bold(), name.cyan());
            println!(
                "  {} {} → {}",
                "Usage:".dimmed(),
                format!("raps {}", name).cyan(),
                command
            );
        }
        _ => {
            output_format.write(&serde_json::json!({
                "alias": name,
                "command": command
            }))?;
        }
    }

    Ok(())
}

fn remove_alias(name: &str, output_format: OutputFormat) -> Result<()> {
    let mut config = PluginConfig::load()?;

    if config.aliases.remove(name).is_some() {
        config.save()?;

        match output_format {
            OutputFormat::Table => {
                println!("{} Alias '{}' removed", "✓".green().bold(), name.cyan());
            }
            _ => {
                output_format.write(&serde_json::json!({
                    "alias": name,
                    "removed": true
                }))?;
            }
        }
    } else {
        match output_format {
            OutputFormat::Table => {
                println!("{} Alias '{}' not found", "!".yellow().bold(), name);
            }
            _ => {
                output_format.write(&serde_json::json!({
                    "alias": name,
                    "error": "not found"
                }))?;
            }
        }
    }

    Ok(())
}

fn plugin_info(name: &str, output_format: OutputFormat) -> Result<()> {
    let manager = PluginManager::default();
    let plugins = manager.list_plugins();

    let plugin = plugins.iter().find(|p| p.name == name);
    let config = PluginConfig::load().unwrap_or_default();
    let entry = config.plugins.get(name);

    match plugin {
        Some(p) => {
            let file_size = std::fs::metadata(&p.path).map(|m| m.len()).unwrap_or(0);

            #[derive(Serialize, schemars::JsonSchema)]
            struct PluginInfoOutput {
                name: String,
                path: String,
                enabled: bool,
                description: Option<String>,
                trusted: bool,
                has_sha256: bool,
                has_signature: bool,
                file_size: u64,
            }

            let info = PluginInfoOutput {
                name: p.name.clone(),
                path: p.path.to_string_lossy().to_string(),
                enabled: p.enabled,
                description: entry.and_then(|e| e.description.clone()),
                trusted: entry.is_some_and(|e| e.trusted),
                has_sha256: entry.and_then(|e| e.sha256.as_ref()).is_some(),
                has_signature: entry.and_then(|e| e.signature.as_ref()).is_some(),
                file_size,
            };

            match output_format {
                OutputFormat::Table => {
                    println!("\n{} {}", "Plugin:".bold(), info.name.cyan().bold());
                    println!("{}", "─".repeat(60));
                    println!("  {:<16} {}", "Path:".bold(), info.path);
                    println!(
                        "  {:<16} {}",
                        "Status:".bold(),
                        if info.enabled {
                            "enabled".green().to_string()
                        } else {
                            "disabled".red().to_string()
                        }
                    );
                    if let Some(ref desc) = info.description {
                        println!("  {:<16} {}", "Description:".bold(), desc);
                    }
                    println!(
                        "  {:<16} {}",
                        "Size:".bold(),
                        super::object::format_size(file_size)
                    );
                    println!(
                        "  {:<16} {}",
                        "Trusted:".bold(),
                        if info.trusted {
                            "yes".green().to_string()
                        } else {
                            "no".yellow().to_string()
                        }
                    );
                    if info.has_sha256 {
                        println!("  {:<16} {}", "SHA-256:".bold(), "recorded".green());
                    }
                    if info.has_signature {
                        println!("  {:<16} {}", "Signature:".bold(), "present".green());
                    }
                    println!("{}", "─".repeat(60));
                }
                _ => {
                    output_format.write(&info)?;
                }
            }
        }
        None => {
            anyhow::bail!(
                "Plugin '{}' not found. Use 'raps plugin list' to see available plugins.",
                name
            );
        }
    }

    Ok(())
}

fn trust_plugin(name: &str, output_format: OutputFormat) -> Result<()> {
    let manager = PluginManager::default();
    let hash = manager.trust_plugin(name)?;

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Plugin '{}' trusted with SHA-256: {}",
                "✓".green().bold(),
                name.cyan(),
                &hash[..16]
            );
        }
        _ => {
            output_format.write(&serde_json::json!({
                "plugin": name,
                "trusted": true,
                "sha256": hash
            }))?;
        }
    }

    Ok(())
}

fn verify_plugin(name: &str, output_format: OutputFormat) -> Result<()> {
    let manager = PluginManager::default();
    let result = manager.verify_plugin(name)?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} {}", "Plugin:".bold(), result.name.cyan());
            println!("  {:<16} {}", "Path:".dimmed(), result.path.display());
            println!("  {:<16} {}", "SHA-256:".dimmed(), result.current_hash);

            if let Some(ref recorded) = result.recorded_hash {
                let status = if result.hash_match {
                    "match".green().to_string()
                } else {
                    "MISMATCH".red().bold().to_string()
                };
                println!("  {:<16} {} ({})", "Recorded:".dimmed(), recorded, status);
            } else {
                println!(
                    "  {:<16} {}",
                    "Recorded:".dimmed(),
                    "none (not yet trusted)".yellow()
                );
            }

            if result.has_signature {
                let sig_status = if result.signature_valid {
                    "valid".green().bold().to_string()
                } else {
                    "INVALID".red().bold().to_string()
                };
                println!("  {:<16} {}", "Signature:".dimmed(), sig_status);
            }

            let trust_status = if result.has_signature && result.signature_valid {
                "signed + verified".green().to_string()
            } else if result.hash_match && result.trusted {
                "TOFU hash verified".green().to_string()
            } else if !result.hash_match && result.recorded_hash.is_some() {
                "UNTRUSTED — hash changed".red().bold().to_string()
            } else {
                "not yet trusted".yellow().to_string()
            };
            println!("  {:<16} {}", "Trust:".dimmed(), trust_status);
        }
        _ => {
            output_format.write(&serde_json::json!({
                "plugin": result.name,
                "path": result.path.to_string_lossy(),
                "sha256": result.current_hash,
                "recorded_sha256": result.recorded_hash,
                "hash_match": result.hash_match,
                "has_signature": result.has_signature,
                "signature_valid": result.signature_valid,
                "trusted": result.trusted,
            }))?;
        }
    }

    Ok(())
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

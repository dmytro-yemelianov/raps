// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Skill commands — discover, install, and manage Claude Code skills.

use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use crate::skill::installer;
use crate::skill::registry::BundledRegistry;

/// Skill management commands
#[derive(Debug, Subcommand)]
pub enum SkillCommands {
    /// List available and installed skills
    List(ListArgs),

    /// Install a skill to ~/.claude/skills/
    Install(InstallArgs),

    /// Uninstall a skill
    Uninstall(UninstallArgs),

    /// Show detailed info about a skill
    Info(InfoArgs),

    /// Print the skills installation directory path
    Path,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Show only installed skills
    #[arg(long)]
    pub installed: bool,
}

#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Skill name
    pub name: String,

    /// Overwrite existing installation
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct UninstallArgs {
    /// Skill name
    pub name: String,
}

#[derive(Debug, Args)]
pub struct InfoArgs {
    /// Skill name
    pub name: String,
}

#[derive(Debug, Serialize)]
struct SkillRow {
    name: String,
    version: String,
    status: String,
    description: String,
}

impl SkillCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            SkillCommands::List(args) => list(args, output_format),
            SkillCommands::Install(args) => install(args),
            SkillCommands::Uninstall(args) => uninstall(args),
            SkillCommands::Info(args) => info(args, output_format),
            SkillCommands::Path => path(),
        }
    }
}

fn list(args: ListArgs, output_format: OutputFormat) -> Result<()> {
    let registry = BundledRegistry::load();
    let installed = installer::list_installed();

    let mut rows: Vec<SkillRow> = if args.installed {
        registry
            .skills
            .iter()
            .filter(|s| installed.contains(&s.name))
            .map(|s| SkillRow {
                name: s.name.clone(),
                version: s.version.clone(),
                status: "installed".to_string(),
                description: truncate(&s.description, 60),
            })
            .collect()
    } else {
        registry
            .skills
            .iter()
            .map(|s| {
                let status = if installed.contains(&s.name) {
                    "installed"
                } else {
                    "available"
                };
                SkillRow {
                    name: s.name.clone(),
                    version: s.version.clone(),
                    status: status.to_string(),
                    description: truncate(&s.description, 60),
                }
            })
            .collect()
    };

    // Add installed-but-not-bundled (custom/community) skills
    for name in &installed {
        if registry.get(name).is_none() {
            rows.push(SkillRow {
                name: name.clone(),
                version: "-".to_string(),
                status: "custom".to_string(),
                description: "Locally installed skill".to_string(),
            });
        }
    }

    match output_format {
        OutputFormat::Table => {
            if rows.is_empty() {
                println!("No skills found.");
                return Ok(());
            }
            println!(
                "\n  {:<25} {:<10} {:<12} {}",
                "Name".bold(),
                "Version".bold(),
                "Status".bold(),
                "Description".bold()
            );
            for row in &rows {
                let status_colored = match row.status.as_str() {
                    "installed" => row.status.green().to_string(),
                    _ => row.status.yellow().to_string(),
                };
                println!(
                    "  {:<25} {:<10} {:<12} {}",
                    row.name.cyan(),
                    row.version,
                    status_colored,
                    row.description
                );
            }
            println!();
        }
        _ => {
            output_format.write(&rows)?;
        }
    }
    Ok(())
}

fn install(args: InstallArgs) -> Result<()> {
    match installer::install_skill(&args.name, args.force) {
        Ok(msg) => {
            println!("{}", msg);
            Ok(())
        }
        Err(msg) => {
            eprintln!("{} {}", "Error:".red().bold(), msg);
            std::process::exit(1);
        }
    }
}

fn uninstall(args: UninstallArgs) -> Result<()> {
    match installer::uninstall_skill(&args.name) {
        Ok(msg) => {
            println!("{}", msg);
            Ok(())
        }
        Err(msg) => {
            eprintln!("{} {}", "Error:".red().bold(), msg);
            std::process::exit(1);
        }
    }
}

fn info(args: InfoArgs, output_format: OutputFormat) -> Result<()> {
    let registry = BundledRegistry::load();
    let installed = installer::list_installed();
    let skills_path = installer::skills_dir();

    let entry = registry.get(&args.name);
    let is_installed = installed.contains(&args.name);

    match entry {
        Some(entry) => match output_format {
            OutputFormat::Table => {
                println!();
                println!("  {:<14} {}", "Name:".bold(), entry.name.cyan());
                println!("  {:<14} {}", "Version:".bold(), entry.version);
                println!("  {:<14} {}", "Description:".bold(), entry.description);
                if is_installed {
                    println!("  {:<14} {}", "Status:".bold(), "installed".green());
                    println!(
                        "  {:<14} {}",
                        "Location:".bold(),
                        skills_path.join(&entry.name).join("SKILL.md").display()
                    );
                } else {
                    println!(
                        "  {:<14} {}",
                        "Status:".bold(),
                        "available (not installed)".yellow()
                    );
                }
                println!("  {:<14} bundled", "Source:".bold());

                if let Some(content) = registry.get_content(&args.name) {
                    println!();
                    println!("  {}", "Preview:".bold());
                    for line in content.lines().take(20) {
                        println!("  {}", line.dimmed());
                    }
                    println!("  {}", "...".dimmed());
                }
                println!();
            }
            _ => {
                #[derive(Serialize)]
                struct SkillInfo {
                    name: String,
                    version: String,
                    description: String,
                    status: String,
                    source: String,
                    installed_path: Option<String>,
                }
                let info = SkillInfo {
                    name: entry.name.clone(),
                    version: entry.version.clone(),
                    description: entry.description.clone(),
                    status: if is_installed {
                        "installed".to_string()
                    } else {
                        "available".to_string()
                    },
                    source: "bundled".to_string(),
                    installed_path: if is_installed {
                        Some(
                            skills_path
                                .join(&entry.name)
                                .join("SKILL.md")
                                .to_string_lossy()
                                .to_string(),
                        )
                    } else {
                        None
                    },
                };
                output_format.write(&info)?;
            }
        },
        None => {
            eprintln!(
                "{} Unknown skill '{}'. Run 'raps skill list' to see available skills.",
                "Error:".red().bold(),
                args.name
            );
            std::process::exit(1);
        }
    }

    Ok(())
}

fn path() -> Result<()> {
    println!("{}", installer::skills_dir().display());
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{truncated}...")
    }
}

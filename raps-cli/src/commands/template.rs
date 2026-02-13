// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Project template management commands (v4.5+)
//!
//! Commands for managing project templates in ACC accounts.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use serde::Serialize;

use raps_acc::admin::AccountAdminClient;

use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum TemplateCommands {
    /// List project templates in an account
    List {
        /// Account ID (defaults to APS_ACCOUNT_ID env var)
        #[arg(short, long)]
        account: Option<String>,

        /// Maximum templates to return
        #[arg(long)]
        limit: Option<usize>,
    },

    /// Get details of a specific template
    Info {
        /// Account ID
        #[arg(short, long)]
        account: Option<String>,

        /// Template project ID
        template_id: String,
    },

    /// Create a new project template
    Create {
        /// Account ID
        #[arg(short, long)]
        account: Option<String>,

        /// Template name
        #[arg(long)]
        name: String,
    },

    /// Update an existing template
    Update {
        /// Account ID
        #[arg(short, long)]
        account: Option<String>,

        /// Template project ID
        template_id: String,

        /// New template name
        #[arg(long)]
        name: Option<String>,
    },

    /// Archive a template
    Archive {
        /// Account ID
        #[arg(short, long)]
        account: Option<String>,

        /// Template project ID
        template_id: String,
    },
}

#[derive(Serialize)]
struct TemplateOutput {
    id: String,
    name: String,
    status: String,
}

impl TemplateCommands {
    pub async fn execute(
        self,
        admin_client: &AccountAdminClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            TemplateCommands::List { account, limit } => {
                let account_id = get_account_id(account)?;
                let limit = limit.unwrap_or(100);

                if output_format.supports_colors() {
                    println!("{}", "Fetching templates...".dimmed());
                }

                let templates = admin_client.list_all_templates(&account_id).await?;
                let templates: Vec<_> = templates.into_iter().take(limit).collect();

                let outputs: Vec<TemplateOutput> = templates
                    .iter()
                    .map(|t| TemplateOutput {
                        id: t.id.clone(),
                        name: t.name.clone(),
                        status: t.status.clone().unwrap_or_else(|| "unknown".to_string()),
                    })
                    .collect();

                if outputs.is_empty() {
                    match output_format {
                        OutputFormat::Table => {
                            println!("{}", "No templates found.".yellow());
                        }
                        _ => output_format.write(&Vec::<TemplateOutput>::new())?,
                    }
                    return Ok(());
                }

                match output_format {
                    OutputFormat::Table => {
                        println!("\n{}", "Templates:".bold());
                        println!("{}", "─".repeat(80));
                        for t in &outputs {
                            println!(
                                "{:<40} {:<30} {}",
                                t.id.cyan(),
                                t.name,
                                t.status.green()
                            );
                        }
                        println!("{}", "─".repeat(80));
                        println!("{} {} template(s)", "→".cyan(), outputs.len());
                    }
                    _ => output_format.write(&outputs)?,
                }

                Ok(())
            }

            TemplateCommands::Info {
                account,
                template_id,
            } => {
                let account_id = get_account_id(account)?;
                let project = admin_client.get_project(&account_id, &template_id).await?;

                let output = TemplateOutput {
                    id: project.id.clone(),
                    name: project.name.clone(),
                    status: project
                        .status
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                };

                match output_format {
                    OutputFormat::Table => {
                        println!("\n{}", "Template Details:".bold());
                        println!("{}", "─".repeat(60));
                        println!("{:<15} {}", "ID:".bold(), output.id.cyan());
                        println!("{:<15} {}", "Name:".bold(), output.name);
                        println!("{:<15} {}", "Status:".bold(), output.status);
                        println!("{}", "─".repeat(60));
                    }
                    _ => output_format.write(&output)?,
                }

                Ok(())
            }

            TemplateCommands::Create { account, name } => {
                let account_id = get_account_id(account)?;

                let request = raps_acc::admin::CreateProjectRequest {
                    name,
                    classification: Some(raps_acc::types::ProjectClassification::Template),
                    ..Default::default()
                };

                let project = admin_client.create_project(&account_id, request).await?;

                match output_format {
                    OutputFormat::Table => {
                        println!(
                            "\n{} Template created: {} ({})",
                            "✓".green().bold(),
                            project.name,
                            project.id.cyan()
                        );
                    }
                    _ => {
                        let output = TemplateOutput {
                            id: project.id,
                            name: project.name,
                            status: project
                                .status
                                .unwrap_or_else(|| "pending".to_string()),
                        };
                        output_format.write(&output)?;
                    }
                }

                Ok(())
            }

            TemplateCommands::Update {
                account,
                template_id,
                name,
            } => {
                let account_id = get_account_id(account)?;

                let request = raps_acc::admin::UpdateProjectRequest {
                    name,
                    ..Default::default()
                };

                let project = admin_client
                    .update_project(&account_id, &template_id, request)
                    .await?;

                match output_format {
                    OutputFormat::Table => {
                        println!(
                            "\n{} Template updated: {} ({})",
                            "✓".green().bold(),
                            project.name,
                            project.id.cyan()
                        );
                    }
                    _ => {
                        let output = TemplateOutput {
                            id: project.id,
                            name: project.name,
                            status: project
                                .status
                                .unwrap_or_else(|| "unknown".to_string()),
                        };
                        output_format.write(&output)?;
                    }
                }

                Ok(())
            }

            TemplateCommands::Archive {
                account,
                template_id,
            } => {
                let account_id = get_account_id(account)?;
                admin_client
                    .archive_project(&account_id, &template_id)
                    .await?;

                match output_format {
                    OutputFormat::Table => {
                        println!(
                            "\n{} Template archived: {}",
                            "✓".green().bold(),
                            template_id.cyan()
                        );
                    }
                    _ => {
                        let output = TemplateOutput {
                            id: template_id,
                            name: String::new(),
                            status: "archived".to_string(),
                        };
                        output_format.write(&output)?;
                    }
                }

                Ok(())
            }
        }
    }
}

fn get_account_id(account: Option<String>) -> Result<String> {
    let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());
    match account_id {
        Some(id) if !id.is_empty() => Ok(id),
        _ => {
            anyhow::bail!(
                "Account ID is required. Use --account or set APS_ACCOUNT_ID environment variable."
            );
        }
    }
}

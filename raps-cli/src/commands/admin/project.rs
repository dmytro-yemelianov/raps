// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Project management and company listing command implementations

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use raps_acc::admin::{AccountAdminClient, CreateProjectRequest, UpdateProjectRequest};
use raps_acc::types::ProjectClassification;
use raps_admin::ProjectFilter;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

use crate::output::OutputFormat;

use super::{AdminProjectCommands, get_account_id};

#[derive(Serialize, schemars::JsonSchema)]
struct ProjectListOutput {
    id: String,
    name: String,
    status: String,
    platform: String,
    created_at: Option<String>,
}

pub(crate) fn format_project_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "active" => status.green().to_string(),
        "inactive" => status.yellow().to_string(),
        "archived" => status.dimmed().to_string(),
        _ => status.to_string(),
    }
}

#[derive(Serialize, schemars::JsonSchema)]
struct CompanyListOutput {
    id: String,
    name: String,
    trade: Option<String>,
    city: Option<String>,
    country: Option<String>,
    member_count: Option<usize>,
}

/// Execute company listing for an account
pub(crate) async fn execute_company_list(
    config: &Config,
    auth_client: &AuthClient,
    account: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = get_account_id(account)?;

    if output_format.supports_colors() {
        println!(
            "\n{} List companies in account {}",
            "\u{2192}".cyan(),
            account_id.cyan()
        );
        println!();
    }

    let http_config = HttpClientConfig::default();
    let admin_client =
        AccountAdminClient::new_with_http_config(config.clone(), auth_client.clone(), http_config);

    let companies = admin_client.list_companies(&account_id).await?;

    let outputs: Vec<CompanyListOutput> = companies
        .iter()
        .map(|c| CompanyListOutput {
            id: c.id.clone(),
            name: c.name.clone(),
            trade: c.trade.clone(),
            city: c.city.clone(),
            country: c.country.clone(),
            member_count: c.member_count,
        })
        .collect();

    match output_format {
        OutputFormat::Table => {
            if outputs.is_empty() {
                println!("{}", "No companies found.".yellow());
            } else {
                println!("{}", "Companies:".bold());
                println!("{}", "\u{2500}".repeat(110));
                println!(
                    "{:<38} {:<25} {:<15} {:<15} {:<10} {}",
                    "ID".bold(),
                    "Name".bold(),
                    "Trade".bold(),
                    "City".bold(),
                    "Country".bold(),
                    "Members".bold()
                );
                println!("{}", "\u{2500}".repeat(110));

                for c in &outputs {
                    let name_truncated = if c.name.len() > 23 {
                        format!("{}...", &c.name[..20])
                    } else {
                        c.name.clone()
                    };
                    let trade_display = c.trade.as_deref().unwrap_or("-");
                    let trade_truncated = if trade_display.len() > 13 {
                        format!("{}...", &trade_display[..10])
                    } else {
                        trade_display.to_string()
                    };
                    let city_display = c.city.as_deref().unwrap_or("-");
                    let country_display = c.country.as_deref().unwrap_or("-");
                    let members_display = c
                        .member_count
                        .map(|m| m.to_string())
                        .unwrap_or_else(|| "-".to_string());

                    println!(
                        "{:<38} {:<25} {:<15} {:<15} {:<10} {}",
                        c.id.cyan(),
                        name_truncated,
                        trade_truncated,
                        city_display,
                        country_display,
                        members_display.dimmed()
                    );
                }

                println!("{}", "\u{2500}".repeat(110));
                println!("{} {} company(ies) found", "\u{2192}".cyan(), outputs.len());
            }
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

impl AdminProjectCommands {
    pub async fn execute(
        self,
        config: &Config,
        auth_client: &AuthClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            AdminProjectCommands::List {
                account,
                filter,
                status,
                platform,
                limit,
            } => {
                let account_id = get_account_id(account)?;

                // Build filter expression from individual flags
                let mut filter_parts = Vec::new();
                if let Some(f) = &filter {
                    filter_parts.push(f.clone());
                }
                if let Some(s) = &status {
                    filter_parts.push(format!("status:{}", s));
                }
                if platform != "all" {
                    filter_parts.push(format!("platform:{}", platform));
                }

                let filter_expr = if filter_parts.is_empty() {
                    None
                } else {
                    Some(filter_parts.join(","))
                };

                let project_filter = if let Some(ref expr) = filter_expr {
                    ProjectFilter::from_expression(expr)?
                } else {
                    ProjectFilter::new()
                };

                if output_format.supports_colors() {
                    println!(
                        "\n{} List projects in account {}",
                        "\u{2192}".cyan(),
                        account_id.cyan()
                    );
                    if let Some(ref expr) = filter_expr {
                        println!("  Filter: {}", expr);
                    }
                    if let Some(l) = limit {
                        println!("  Limit: {}", l);
                    }
                    println!();
                }

                // Create admin client
                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                );

                // List all projects
                let all_projects = admin_client.list_all_projects(&account_id).await?;

                // Apply filter
                let mut filtered_projects = project_filter.apply(all_projects);

                // Apply limit
                if let Some(l) = limit {
                    filtered_projects.truncate(l);
                }

                // Build output
                let outputs: Vec<ProjectListOutput> = filtered_projects
                    .iter()
                    .map(|p| ProjectListOutput {
                        id: p.id.clone(),
                        name: p.name.clone(),
                        status: p.status.clone().unwrap_or_else(|| "unknown".to_string()),
                        platform: if p.is_acc() {
                            "acc".to_string()
                        } else if p.is_bim360() {
                            "bim360".to_string()
                        } else {
                            "unknown".to_string()
                        },
                        created_at: p.created_at.map(|d| d.to_rfc3339()),
                    })
                    .collect();

                match output_format {
                    OutputFormat::Table => {
                        if outputs.is_empty() {
                            println!("{}", "No projects found matching the filter.".yellow());
                        } else {
                            println!("{}", "Projects:".bold());
                            println!("{}", "\u{2500}".repeat(100));
                            println!(
                                "{:<38} {:<30} {:<10} {:<10} {}",
                                "ID".bold(),
                                "Name".bold(),
                                "Status".bold(),
                                "Platform".bold(),
                                "Created".bold()
                            );
                            println!("{}", "\u{2500}".repeat(100));

                            for p in &outputs {
                                let created = p.created_at.as_deref().unwrap_or("-");
                                let name_truncated = if p.name.len() > 28 {
                                    format!("{}...", &p.name[..25])
                                } else {
                                    p.name.clone()
                                };
                                println!(
                                    "{:<38} {:<30} {:<10} {:<10} {}",
                                    p.id.cyan(),
                                    name_truncated,
                                    format_project_status(&p.status),
                                    p.platform,
                                    created.dimmed()
                                );
                            }

                            println!("{}", "\u{2500}".repeat(100));
                            println!("{} {} project(s) found", "\u{2192}".cyan(), outputs.len());
                        }
                    }
                    _ => {
                        output_format.write(&outputs)?;
                    }
                }

                Ok(())
            }
            AdminProjectCommands::Create {
                account,
                name,
                r#type,
                classification,
                start_date,
                end_date,
                timezone,
            } => {
                let account_id = get_account_id(account)?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} Creating project '{}' in account {}",
                        "\u{2192}".cyan(),
                        name.cyan(),
                        account_id.cyan()
                    );
                }

                let parsed_classification = if let Some(ref cls) = classification {
                    Some(match cls.to_lowercase().as_str() {
                        "production" => ProjectClassification::Production,
                        "template" => ProjectClassification::Template,
                        "component" => ProjectClassification::Component,
                        "sample" => ProjectClassification::Sample,
                        _ => anyhow::bail!(
                            "Invalid classification '{}'. Valid values: production, template, component, sample",
                            cls
                        ),
                    })
                } else {
                    None
                };

                let request = CreateProjectRequest {
                    name: name.clone(),
                    r#type,
                    classification: parsed_classification,
                    start_date,
                    end_date,
                    timezone,
                    ..Default::default()
                };

                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                );

                let project = admin_client.create_project(&account_id, request).await?;

                match output_format {
                    OutputFormat::Table => {
                        println!(
                            "\n{} Project created successfully!",
                            "\u{2713}".green().bold()
                        );
                        println!("{:<15} {}", "ID:".bold(), project.id.cyan());
                        println!("{:<15} {}", "Name:".bold(), project.name);
                        println!(
                            "{:<15} {}",
                            "Status:".bold(),
                            project.status.as_deref().unwrap_or("pending")
                        );
                    }
                    _ => {
                        output_format.write(&serde_json::json!({
                            "id": project.id,
                            "name": project.name,
                            "status": project.status,
                            "created": true
                        }))?;
                    }
                }

                Ok(())
            }
            AdminProjectCommands::Update {
                account,
                project,
                name,
                status,
                start_date,
                end_date,
            } => {
                let account_id = get_account_id(account)?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} Updating project {} in account {}",
                        "\u{2192}".cyan(),
                        project.cyan(),
                        account_id.cyan()
                    );
                }

                let request = UpdateProjectRequest {
                    name,
                    status,
                    start_date,
                    end_date,
                    ..Default::default()
                };

                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                );

                let updated = admin_client
                    .update_project(&account_id, &project, request)
                    .await?;

                match output_format {
                    OutputFormat::Table => {
                        println!(
                            "\n{} Project updated successfully!",
                            "\u{2713}".green().bold()
                        );
                        println!("{:<15} {}", "ID:".bold(), updated.id.cyan());
                        println!("{:<15} {}", "Name:".bold(), updated.name);
                        println!(
                            "{:<15} {}",
                            "Status:".bold(),
                            updated.status.as_deref().unwrap_or("-")
                        );
                    }
                    _ => {
                        output_format.write(&serde_json::json!({
                            "id": updated.id,
                            "name": updated.name,
                            "status": updated.status,
                            "updated": true
                        }))?;
                    }
                }

                Ok(())
            }
            AdminProjectCommands::Archive { account, project } => {
                let account_id = get_account_id(account)?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} Archiving project {} in account {}",
                        "\u{2192}".cyan(),
                        project.cyan(),
                        account_id.cyan()
                    );
                }

                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                );

                admin_client.archive_project(&account_id, &project).await?;

                match output_format {
                    OutputFormat::Table => {
                        println!(
                            "\n{} Project archived successfully!",
                            "\u{2713}".green().bold()
                        );
                        println!("{:<15} {}", "ID:".bold(), project.cyan());
                    }
                    _ => {
                        output_format.write(&serde_json::json!({
                            "id": project,
                            "archived": true
                        }))?;
                    }
                }

                Ok(())
            }
        }
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Project management and company listing command implementations

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use raps_acc::admin::{
    AccountAdminClient, CreateCompanyRequest, CreateProjectRequest, UpdateCompanyRequest,
    UpdateProjectRequest,
};
use raps_acc::extended::AccClient;
use raps_acc::types::ProjectClassification;
use raps_admin::ProjectFilter;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

use raps_dm::DataManagementClient;

use crate::output::OutputFormat;

use super::{AdminProjectCommands, resolve_account_id};

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct ProjectListOutput {
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
pub(crate) struct CompanyListOutput {
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
    dm_client: &DataManagementClient,
    account: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = resolve_account_id(account, dm_client).await?;

    if output_format.supports_colors() {
        println!(
            "\n{} List companies in account {}",
            "→".cyan(),
            account_id.cyan()
        );
        println!();
    }

    let http_config = HttpClientConfig::default();
    let admin_client =
        AccountAdminClient::new_with_http_config(config.clone(), auth_client.clone(), http_config)?;

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
                println!("{}", "─".repeat(110));
                println!(
                    "{:<38} {:<25} {:<15} {:<15} {:<10} {}",
                    "ID".bold(),
                    "Name".bold(),
                    "Trade".bold(),
                    "City".bold(),
                    "Country".bold(),
                    "Members".bold()
                );
                println!("{}", "─".repeat(110));

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

                println!("{}", "─".repeat(110));
                println!("{} {} company(ies) found", "→".cyan(), outputs.len());
            }
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

/// Execute company get by ID
pub(crate) async fn execute_company_get(
    config: &Config,
    auth_client: &AuthClient,
    dm_client: &DataManagementClient,
    company_id: String,
    account: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = resolve_account_id(account, dm_client).await?;

    let http_config = HttpClientConfig::default();
    let admin_client =
        AccountAdminClient::new_with_http_config(config.clone(), auth_client.clone(), http_config)?;

    let company = admin_client.get_company(&account_id, &company_id).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Company Details:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "ID:".bold(), company.id.cyan());
            println!("{:<15} {}", "Name:".bold(), company.name);
            println!(
                "{:<15} {}",
                "Trade:".bold(),
                company.trade.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "City:".bold(),
                company.city.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Country:".bold(),
                company.country.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "State:".bold(),
                company.state_or_province.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Address:".bold(),
                company.address_line1.as_deref().unwrap_or("-")
            );
            if let Some(members) = company.member_count {
                println!("{:<15} {}", "Members:".bold(), members);
            }
        }
        _ => {
            output_format.write(&CompanyListOutput {
                id: company.id,
                name: company.name,
                trade: company.trade,
                city: company.city,
                country: company.country,
                member_count: company.member_count,
            })?;
        }
    }
    Ok(())
}

/// Execute company search by name
pub(crate) async fn execute_company_search(
    config: &Config,
    auth_client: &AuthClient,
    dm_client: &DataManagementClient,
    name: String,
    account: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = resolve_account_id(account, dm_client).await?;

    if output_format.supports_colors() {
        println!(
            "\n{} Search companies matching '{}' in account {}",
            "→".cyan(),
            name.cyan(),
            account_id.cyan()
        );
        println!();
    }

    let http_config = HttpClientConfig::default();
    let admin_client =
        AccountAdminClient::new_with_http_config(config.clone(), auth_client.clone(), http_config)?;

    let companies = admin_client.search_companies(&account_id, &name).await?;

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
                println!("{}", "─".repeat(110));
                println!(
                    "{:<38} {:<25} {:<15} {:<15} {:<10} {}",
                    "ID".bold(),
                    "Name".bold(),
                    "Trade".bold(),
                    "City".bold(),
                    "Country".bold(),
                    "Members".bold()
                );
                println!("{}", "─".repeat(110));

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

                println!("{}", "─".repeat(110));
                println!("{} {} company(ies) found", "→".cyan(), outputs.len());
            }
        }
        _ => {
            output_format.write(&outputs)?;
        }
    }

    Ok(())
}

/// Execute company creation
pub(crate) async fn execute_company_create(
    config: &Config,
    auth_client: &AuthClient,
    dm_client: &DataManagementClient,
    name: String,
    trade: Option<String>,
    city: Option<String>,
    country: Option<String>,
    account: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = resolve_account_id(account, dm_client).await?;

    if output_format.supports_colors() {
        println!(
            "\n{} Creating company '{}' in account {}",
            "→".cyan(),
            name.cyan(),
            account_id.cyan()
        );
    }

    let http_config = HttpClientConfig::default();
    let admin_client =
        AccountAdminClient::new_with_http_config(config.clone(), auth_client.clone(), http_config)?;

    let request = CreateCompanyRequest {
        name,
        trade,
        city,
        country,
        address_line_1: None,
        state_or_province: None,
    };

    let company = admin_client.create_company(&account_id, request).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Company created!", "✓".green().bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "ID:".bold(), company.id.cyan());
            println!("{:<15} {}", "Name:".bold(), company.name);
            println!(
                "{:<15} {}",
                "Trade:".bold(),
                company.trade.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "City:".bold(),
                company.city.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Country:".bold(),
                company.country.as_deref().unwrap_or("-")
            );
        }
        _ => {
            output_format.write(&CompanyListOutput {
                id: company.id,
                name: company.name,
                trade: company.trade,
                city: company.city,
                country: company.country,
                member_count: company.member_count,
            })?;
        }
    }
    Ok(())
}

/// Execute company update
pub(crate) async fn execute_company_update(
    config: &Config,
    auth_client: &AuthClient,
    dm_client: &DataManagementClient,
    company_id: String,
    name: Option<String>,
    trade: Option<String>,
    city: Option<String>,
    country: Option<String>,
    account: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = resolve_account_id(account, dm_client).await?;

    if output_format.supports_colors() {
        println!(
            "\n{} Updating company {} in account {}",
            "→".cyan(),
            company_id.cyan(),
            account_id.cyan()
        );
    }

    let http_config = HttpClientConfig::default();
    let admin_client =
        AccountAdminClient::new_with_http_config(config.clone(), auth_client.clone(), http_config)?;

    let request = UpdateCompanyRequest {
        name,
        trade,
        city,
        country,
        ..Default::default()
    };

    let company = admin_client
        .update_company(&account_id, &company_id, request)
        .await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Company updated!", "✓".green().bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "ID:".bold(), company.id.cyan());
            println!("{:<15} {}", "Name:".bold(), company.name);
            println!(
                "{:<15} {}",
                "Trade:".bold(),
                company.trade.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "City:".bold(),
                company.city.as_deref().unwrap_or("-")
            );
            println!(
                "{:<15} {}",
                "Country:".bold(),
                company.country.as_deref().unwrap_or("-")
            );
        }
        _ => {
            output_format.write(&CompanyListOutput {
                id: company.id,
                name: company.name,
                trade: company.trade,
                city: company.city,
                country: company.country,
                member_count: company.member_count,
            })?;
        }
    }
    Ok(())
}

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct RoleListOutput {
    id: String,
    name: String,
}

/// Execute role listing for an account
pub(crate) async fn execute_role_list(
    config: &Config,
    auth_client: &AuthClient,
    dm_client: &DataManagementClient,
    account: Option<String>,
    project: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = resolve_account_id(account, dm_client).await?;

    if output_format.supports_colors() {
        println!(
            "\n{} List roles in account {}",
            "→".cyan(),
            account_id.cyan()
        );
        println!();
    }

    let http_config = HttpClientConfig::default();
    let admin_client =
        AccountAdminClient::new_with_http_config(config.clone(), auth_client.clone(), http_config)?;

    let roles = admin_client
        .list_roles_with_project(&account_id, project.as_deref())
        .await?;

    if roles.is_empty() {
        // ACC hub — show built-in product-based roles
        let outputs = vec![
            RoleListOutput {
                id: "(product-based)".to_string(),
                name: "Project Admin".to_string(),
            },
            RoleListOutput {
                id: "(product-based)".to_string(),
                name: "Project Member".to_string(),
            },
            RoleListOutput {
                id: "(product-based)".to_string(),
                name: "Project Editor".to_string(),
            },
            RoleListOutput {
                id: "(product-based)".to_string(),
                name: "Project Viewer".to_string(),
            },
        ];

        match output_format {
            OutputFormat::Table => {
                println!(
                    "{} ACC hub detected — roles are product-based, not UUID-based:",
                    "ℹ".cyan()
                );
                println!("{}", "─".repeat(50));
                for r in &outputs {
                    println!("  {}", r.name.green());
                }
                println!("{}", "─".repeat(50));
                println!(
                    "Use these names with {}: e.g. {}",
                    "--role".bold(),
                    "--role admin".cyan()
                );
            }
            _ => {
                output_format.write(&outputs)?;
            }
        }
    } else {
        let outputs: Vec<RoleListOutput> = roles
            .iter()
            .map(|r| RoleListOutput {
                id: r.id.clone(),
                name: r.name.clone(),
            })
            .collect();

        match output_format {
            OutputFormat::Table => {
                println!("{}", "Roles:".bold());
                println!("{}", "─".repeat(75));
                println!("{:<38} {}", "ID".bold(), "Name".bold(),);
                println!("{}", "─".repeat(75));

                for r in &outputs {
                    println!("{:<38} {}", r.id.cyan(), r.name);
                }

                println!("{}", "─".repeat(75));
                println!("{} {} role(s) found", "→".cyan(), outputs.len());
            }
            _ => {
                output_format.write(&outputs)?;
            }
        }
    }

    Ok(())
}

impl AdminProjectCommands {
    pub async fn execute(
        self,
        config: &Config,
        auth_client: &AuthClient,
        dm_client: &DataManagementClient,
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
                let account_id = resolve_account_id(account, dm_client).await?;

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
                        "→".cyan(),
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
                )?;

                // Determine actual hub platform from hub extension_type.
                // The ACC admin API returns platform:"acc" for all projects
                // regardless of whether the account is ACC or BIM 360.
                let hub_platform = match dm_client.list_hubs().await {
                    Ok(hubs) => {
                        let hub_id = format!("b.{}", account_id);
                        hubs.iter()
                            .find(|h| h.id == hub_id)
                            .and_then(|h| {
                                h.attributes
                                    .extension
                                    .as_ref()?
                                    .extension_type
                                    .as_deref()
                                    .map(|ext| {
                                        if ext.contains("bim360") {
                                            "bim360"
                                        } else if ext.contains("accproject") {
                                            "acc"
                                        } else {
                                            "acc"
                                        }
                                    })
                            })
                            .unwrap_or("acc")
                    }
                    Err(_) => "acc",
                };

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
                        platform: hub_platform.to_string(),
                        created_at: p.created_at.map(|d| d.to_rfc3339()),
                    })
                    .collect();

                match output_format {
                    OutputFormat::Table => {
                        if outputs.is_empty() {
                            println!("{}", "No projects found matching the filter.".yellow());
                        } else {
                            println!("{}", "Projects:".bold());
                            println!("{}", "─".repeat(100));
                            println!(
                                "{:<38} {:<30} {:<10} {:<10} {}",
                                "ID".bold(),
                                "Name".bold(),
                                "Status".bold(),
                                "Platform".bold(),
                                "Created".bold()
                            );
                            println!("{}", "─".repeat(100));

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

                            println!("{}", "─".repeat(100));
                            println!("{} {} project(s) found", "→".cyan(), outputs.len());
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
                let account_id = resolve_account_id(account, dm_client).await?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} Creating project '{}' in account {}",
                        "→".cyan(),
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
                )?;

                let project = admin_client.create_project(&account_id, request).await?;

                match output_format {
                    OutputFormat::Table => {
                        println!("\n{} Project created successfully!", "✓".green().bold());
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
                let account_id = resolve_account_id(account, dm_client).await?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} Updating project {} in account {}",
                        "→".cyan(),
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
                )?;

                let updated = admin_client
                    .update_project(&account_id, &project, request)
                    .await?;

                match output_format {
                    OutputFormat::Table => {
                        println!("\n{} Project updated successfully!", "✓".green().bold());
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
            AdminProjectCommands::CreateBatch {
                account,
                prefix,
                count,
                start,
                concurrency,
                no_wait,
            } => {
                let account_id = resolve_account_id(account, dm_client).await?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} Creating {} projects with prefix '{}' in account {}",
                        "→".cyan(),
                        count.to_string().cyan(),
                        prefix.cyan(),
                        account_id.cyan()
                    );
                    println!(
                        "  Concurrency: {}, Start: {}, Wait: {}",
                        concurrency,
                        start,
                        if no_wait { "no" } else { "yes" }
                    );
                    println!();
                }

                let http_config = HttpClientConfig::default();
                let acc_client = Arc::new(AccClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                )?);

                let created = Arc::new(AtomicUsize::new(0));
                let failed = Arc::new(AtomicUsize::new(0));
                let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));

                let mut handles = Vec::new();

                for i in start..(start + count) {
                    let client = Arc::clone(&acc_client);
                    let acct = account_id.clone();
                    let name = format!("{}-{:03}", prefix, i);
                    let sem = Arc::clone(&semaphore);
                    let created = Arc::clone(&created);
                    let failed = Arc::clone(&failed);
                    let wait = !no_wait;

                    handles.push(tokio::spawn(async move {
                        let _permit = sem.acquire().await.unwrap();
                        let request = raps_acc::CreateProjectRequest {
                            name: name.clone(),
                            template_project_id: None,
                            products: None,
                            project_type: Some("ACC".to_string()),
                        };

                        match client.create_project(&acct, request).await {
                            Ok(job) => {
                                let project_id =
                                    job.project_id.as_deref().unwrap_or("unknown").to_string();

                                if wait {
                                    // Wait for activation (up to 60s)
                                    match client
                                        .wait_for_project_activation(
                                            &acct,
                                            &project_id,
                                            Some(60),
                                            Some(2000),
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            let n = created.fetch_add(1, Ordering::Relaxed) + 1;
                                            eprintln!(
                                                "  {} [{}/{}] {} ({})",
                                                "✓".green(),
                                                n,
                                                n + failed.load(Ordering::Relaxed),
                                                name,
                                                project_id
                                            );
                                        }
                                        Err(e) => {
                                            let n = created.fetch_add(1, Ordering::Relaxed) + 1;
                                            eprintln!(
                                                "  {} [{}/{}] {} ({}) - activation: {}",
                                                "~".yellow(),
                                                n,
                                                n + failed.load(Ordering::Relaxed),
                                                name,
                                                project_id,
                                                e
                                            );
                                        }
                                    }
                                } else {
                                    let n = created.fetch_add(1, Ordering::Relaxed) + 1;
                                    eprintln!(
                                        "  {} [{}/{}] {} ({})",
                                        "✓".green(),
                                        n,
                                        n + failed.load(Ordering::Relaxed),
                                        name,
                                        project_id
                                    );
                                }
                            }
                            Err(e) => {
                                failed.fetch_add(1, Ordering::Relaxed);
                                eprintln!("  {} {} - {}", "✗".red(), name, e);
                            }
                        }
                    }));
                }

                // Wait for all tasks to complete
                for handle in handles {
                    let _ = handle.await;
                }

                let total_created = created.load(Ordering::Relaxed);
                let total_failed = failed.load(Ordering::Relaxed);

                if output_format.supports_colors() {
                    println!();
                    println!(
                        "{} Batch complete: {} created, {} failed",
                        "→".cyan(),
                        total_created.to_string().green(),
                        if total_failed > 0 {
                            total_failed.to_string().red().to_string()
                        } else {
                            "0".to_string()
                        }
                    );
                } else {
                    output_format.write(&serde_json::json!({
                        "created": total_created,
                        "failed": total_failed,
                        "total": count,
                    }))?;
                }

                Ok(())
            }
            AdminProjectCommands::Archive {
                account,
                project,
                export_dir,
                concurrency,
                yes,
            } => {
                let account_id = resolve_account_id(account, dm_client).await?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} Archiving project {} in account {}",
                        "→".cyan(),
                        project.cyan(),
                        account_id.cyan()
                    );
                    if let Some(ref dir) = export_dir {
                        println!("  Export directory: {}", dir.display().to_string().cyan());
                    }
                }

                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                )?;

                // If --export-dir is set, download all files first
                if let Some(ref dir) = export_dir {
                    export_project_files(
                        config,
                        auth_client,
                        dm_client,
                        &account_id,
                        &project,
                        dir,
                        concurrency,
                        output_format,
                    )
                    .await?;
                }

                // Confirmation before archive
                if !yes && output_format.supports_colors() {
                    print!("\nProceed with archiving? [y/N] ");
                    use std::io::{self, Write};
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    if !input.trim().eq_ignore_ascii_case("y") {
                        println!("Archive cancelled. Files were exported.");
                        return Ok(());
                    }
                }

                admin_client.archive_project(&account_id, &project).await?;

                match output_format {
                    OutputFormat::Table => {
                        println!("\n{} Project archived successfully!", "✓".green().bold());
                        println!("{:<15} {}", "ID:".bold(), project.cyan());
                        if let Some(ref dir) = export_dir {
                            println!(
                                "{:<15} {}",
                                "Exported to:".bold(),
                                dir.display().to_string().cyan()
                            );
                        }
                    }
                    _ => {
                        output_format.write(&serde_json::json!({
                            "id": project,
                            "archived": true,
                            "export_dir": export_dir.as_ref().map(|d| d.display().to_string()),
                        }))?;
                    }
                }

                Ok(())
            }
        }
    }
}

/// Export all project files to a local directory, preserving folder structure.
///
/// Recursively walks the project's folder tree, downloads the latest version
/// of each file, and writes a manifest.json summarising the export.
#[allow(clippy::too_many_arguments)]
async fn export_project_files(
    _config: &Config,
    _auth_client: &raps_kernel::auth::AuthClient,
    dm_client: &raps_dm::DataManagementClient,
    account_id: &str,
    project_id: &str,
    export_dir: &std::path::Path,
    concurrency: usize,
    output_format: OutputFormat,
) -> Result<()> {
    use futures_util::stream::{self, StreamExt};
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Resolve hub_id from account_id (needed for get_top_folders)
    let hub_id = format!("b.{}", account_id.trim_start_matches("b."));

    // Create export directory
    std::fs::create_dir_all(export_dir)?;

    if output_format.supports_colors() {
        eprintln!("Discovering project folder structure...");
    }

    // Get top-level folders
    let top_folders = dm_client.get_top_folders(&hub_id, project_id).await?;

    // Recursively collect all files
    let mut file_entries: Vec<FileEntry> = Vec::new();

    for folder in &top_folders {
        let folder_name = folder
            .attributes
            .display_name
            .as_deref()
            .unwrap_or(&folder.attributes.name);
        collect_files_recursive(
            dm_client,
            project_id,
            &folder.id,
            &std::path::PathBuf::from(sanitize_filename(folder_name)),
            &mut file_entries,
        )
        .await?;
    }

    if file_entries.is_empty() {
        if output_format.supports_colors() {
            eprintln!("No files found in project.");
        }
        return Ok(());
    }

    if output_format.supports_colors() {
        eprintln!("Found {} file(s). Downloading...", file_entries.len());
    }

    // Download files concurrently
    let downloaded = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    let total = file_entries.len();

    let results: Vec<DownloadResult> = stream::iter(file_entries)
        .map(|entry| {
            let dm_client = dm_client;
            let export_dir = export_dir;
            let downloaded = &downloaded;
            let failed = &failed;
            async move {
                let local_path = export_dir.join(&entry.relative_path);

                // Create parent directories
                if let Some(parent) = local_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }

                // Get download URL
                let url = match dm_client
                    .get_item_download_url(&entry.project_id, &entry.item_id)
                    .await
                {
                    Ok(Some(u)) => u,
                    Ok(None) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                        return DownloadResult {
                            path: entry.relative_path,
                            status: "no_download_url".to_string(),
                            error: Some("No storage URL available".to_string()),
                        };
                    }
                    Err(e) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                        return DownloadResult {
                            path: entry.relative_path,
                            status: "failed".to_string(),
                            error: Some(e.to_string()),
                        };
                    }
                };

                // Download the file using the signed URL
                match download_url_to_file(&url, &local_path).await {
                    Ok(_) => {
                        downloaded.fetch_add(1, Ordering::Relaxed);
                        DownloadResult {
                            path: entry.relative_path,
                            status: "ok".to_string(),
                            error: None,
                        }
                    }
                    Err(e) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                        DownloadResult {
                            path: entry.relative_path,
                            status: "failed".to_string(),
                            error: Some(e.to_string()),
                        }
                    }
                }
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let ok = downloaded.load(Ordering::Relaxed);
    let fail = failed.load(Ordering::Relaxed);

    // Write manifest
    let manifest = serde_json::json!({
        "project_id": project_id,
        "account_id": account_id,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "total_files": total,
        "downloaded": ok,
        "failed": fail,
        "files": results,
    });
    let manifest_path = export_dir.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    if output_format.supports_colors() {
        eprintln!(
            "Export complete: {} downloaded, {} failed (of {})",
            ok, fail, total
        );
        eprintln!("Manifest: {}", manifest_path.display());
    }

    if fail > 0 {
        anyhow::bail!("{fail} file(s) failed to download");
    }

    Ok(())
}

struct FileEntry {
    project_id: String,
    item_id: String,
    relative_path: std::path::PathBuf,
}

#[derive(serde::Serialize)]
struct DownloadResult {
    path: std::path::PathBuf,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Recursively collect all files from a folder tree.
fn collect_files_recursive<'a>(
    dm_client: &'a raps_dm::DataManagementClient,
    project_id: &'a str,
    folder_id: &'a str,
    current_path: &'a std::path::Path,
    entries: &'a mut Vec<FileEntry>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        let contents = dm_client
            .list_folder_contents(project_id, folder_id)
            .await?;

        for item in contents {
            let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");

            let display_name = item
                .pointer("/attributes/displayName")
                .or_else(|| item.pointer("/attributes/name"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");

            if item_type == "folders" {
                let sub_path = current_path.join(sanitize_filename(display_name));
                collect_files_recursive(dm_client, project_id, id, &sub_path, entries).await?;
            } else if item_type == "items" {
                entries.push(FileEntry {
                    project_id: project_id.to_string(),
                    item_id: id.to_string(),
                    relative_path: current_path.join(sanitize_filename(display_name)),
                });
            }
        }

        Ok(())
    })
}

/// Download a URL to a local file using reqwest.
async fn download_url_to_file(url: &str, path: &std::path::Path) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Download failed: HTTP {}", response.status());
    }

    let bytes = response.bytes().await?;
    tokio::fs::write(path, &bytes).await?;

    Ok(())
}

/// Sanitize a filename by removing characters invalid on common filesystems.
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

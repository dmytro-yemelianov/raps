// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! User management command implementations

use std::sync::Arc;

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use raps_acc::admin::AccountAdminClient;
use raps_acc::users::ProjectUsersClient;
use raps_admin::{BulkConfig, bulk_add_user};
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

use crate::output::OutputFormat;

use super::csv_ops::{execute_csv_import, execute_csv_update};
use super::operations::display_bulk_result;
use super::{
    UserCommands, create_bulk_progress_bar, get_account_id, make_progress_callback,
    parse_filter_with_ids,
};

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct UserListOutput {
    pub(crate) id: String,
    pub(crate) email: String,
    pub(crate) name: String,
    pub(crate) role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) company: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) status: Option<String>,
}

pub(crate) fn display_user_list(
    users: &Vec<UserListOutput>,
    output_format: OutputFormat,
) -> Result<()> {
    if users.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No users found.".yellow()),
            _ => output_format.write(&Vec::<UserListOutput>::new())?,
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("{}", "Users:".bold());
            println!("{}", "─".repeat(110));
            println!(
                "{:<30} {:<25} {:<18} {:<18} {}",
                "Email".bold(),
                "Name".bold(),
                "Role".bold(),
                "Status".bold(),
                "Company".bold()
            );
            println!("{}", "─".repeat(110));

            for u in users {
                let email_truncated = if u.email.len() > 28 {
                    format!("{}...", &u.email[..25])
                } else {
                    u.email.clone()
                };
                let name_truncated = if u.name.len() > 23 {
                    format!("{}...", &u.name[..20])
                } else {
                    u.name.clone()
                };
                let role_display = if u.role.is_empty() {
                    "-".to_string()
                } else if u.role.len() > 16 {
                    format!("{}...", &u.role[..13])
                } else {
                    u.role.clone()
                };
                let status_display = u.status.as_deref().unwrap_or("-");
                let company_display = u.company.as_deref().unwrap_or("-");

                println!(
                    "{:<30} {:<25} {:<18} {:<18} {}",
                    email_truncated.cyan(),
                    name_truncated,
                    role_display,
                    format_user_status(status_display),
                    company_display.dimmed()
                );
            }

            println!("{}", "─".repeat(110));
            println!("{} {} user(s) found", "→".cyan(), users.len());
        }
        _ => {
            output_format.write(users)?;
        }
    }

    Ok(())
}

pub(crate) fn format_user_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "active" => status.green().to_string(),
        "inactive" | "not_invited" => status.yellow().to_string(),
        "disabled" => status.red().to_string(),
        _ => status.to_string(),
    }
}

impl UserCommands {
    pub async fn execute(
        self,
        config: &Config,
        auth_client: &AuthClient,
        output_format: OutputFormat,
        global_concurrency: usize,
    ) -> Result<()> {
        match self {
            UserCommands::List {
                account,
                project,
                role,
                status,
                search,
            } => {
                let account_id = get_account_id(account)?;
                let http_config = HttpClientConfig::default();

                if let Some(project_id) = project {
                    // Project-level user listing
                    if output_format.supports_colors() {
                        println!(
                            "\n{} List users in project {}",
                            "→".cyan(),
                            project_id.cyan()
                        );
                        println!();
                    }

                    let users_client = ProjectUsersClient::new_with_http_config(
                        config.clone(),
                        auth_client.clone(),
                        http_config,
                    );

                    let all_users = users_client.list_all_project_users(&project_id).await?;

                    // Apply filters
                    let filtered: Vec<_> = all_users
                        .into_iter()
                        .filter(|u| {
                            if let Some(ref r) = role {
                                if let Some(ref role_name) = u.role_name {
                                    if !role_name.to_lowercase().contains(&r.to_lowercase()) {
                                        return false;
                                    }
                                } else {
                                    return false;
                                }
                            }
                            if let Some(ref s) = search {
                                let s_lower = s.to_lowercase();
                                let email_match = u
                                    .email
                                    .as_ref()
                                    .map(|e| e.to_lowercase().contains(&s_lower))
                                    .unwrap_or(false);
                                let name_match = u
                                    .name
                                    .as_ref()
                                    .map(|n| n.to_lowercase().contains(&s_lower))
                                    .unwrap_or(false);
                                if !email_match && !name_match {
                                    return false;
                                }
                            }
                            true
                        })
                        .collect();

                    let outputs: Vec<UserListOutput> = filtered
                        .iter()
                        .map(|u| UserListOutput {
                            id: u.id.clone(),
                            email: u.email.clone().unwrap_or_default(),
                            name: u.name.clone().unwrap_or_default(),
                            role: u.role_name.clone().unwrap_or_default(),
                            company: None,
                            status: None,
                        })
                        .collect();

                    display_user_list(&outputs, output_format)?;
                } else {
                    // Account-level user listing
                    if output_format.supports_colors() {
                        println!(
                            "\n{} List users in account {}",
                            "→".cyan(),
                            account_id.cyan()
                        );
                        println!();
                    }

                    let admin_client = AccountAdminClient::new_with_http_config(
                        config.clone(),
                        auth_client.clone(),
                        http_config,
                    );

                    let all_users = admin_client.list_all_users(&account_id).await?;

                    // Apply filters
                    let filtered: Vec<_> = all_users
                        .into_iter()
                        .filter(|u| {
                            if let Some(ref s) = status {
                                if let Some(ref user_status) = u.status {
                                    if !user_status.to_lowercase().eq(&s.to_lowercase()) {
                                        return false;
                                    }
                                } else {
                                    return false;
                                }
                            }
                            if let Some(ref s) = search {
                                let s_lower = s.to_lowercase();
                                let email_match = u.email.to_lowercase().contains(&s_lower);
                                let name_match = u
                                    .name
                                    .as_ref()
                                    .map(|n| n.to_lowercase().contains(&s_lower))
                                    .unwrap_or(false);
                                let first_match = u
                                    .first_name
                                    .as_ref()
                                    .map(|n| n.to_lowercase().contains(&s_lower))
                                    .unwrap_or(false);
                                let last_match = u
                                    .last_name
                                    .as_ref()
                                    .map(|n| n.to_lowercase().contains(&s_lower))
                                    .unwrap_or(false);
                                if !email_match && !name_match && !first_match && !last_match {
                                    return false;
                                }
                            }
                            true
                        })
                        .collect();

                    let outputs: Vec<UserListOutput> = filtered
                        .iter()
                        .map(|u| {
                            let display_name = match (&u.first_name, &u.last_name) {
                                (Some(f), Some(l)) => format!("{} {}", f, l),
                                (Some(f), None) => f.clone(),
                                (None, Some(l)) => l.clone(),
                                (None, None) => u.name.clone().unwrap_or_default(),
                            };
                            UserListOutput {
                                id: u.id.clone(),
                                email: u.email.clone(),
                                name: display_name,
                                role: String::new(),
                                company: u.company_id.clone(),
                                status: u.status.clone(),
                            }
                        })
                        .collect();

                    display_user_list(&outputs, output_format)?;
                }

                Ok(())
            }

            UserCommands::Add {
                email,
                account,
                role,
                filter,
                project_ids,
                concurrency,
                dry_run,
                yes: _,
            } => {
                let concurrency = concurrency.unwrap_or(global_concurrency);
                let account_id = get_account_id(account)?;
                let project_filter = parse_filter_with_ids(&filter, &project_ids)?;

                let bulk_config = BulkConfig {
                    concurrency: concurrency.min(50),
                    dry_run,
                    ..Default::default()
                };

                if output_format.supports_colors() {
                    println!(
                        "\n{} Bulk add user: {} to account {}",
                        "→".cyan(),
                        email.green(),
                        account_id.cyan()
                    );
                    if let Some(r) = &role {
                        println!("  Role: {}", r.yellow());
                    }
                    if let Some(f) = &filter {
                        println!("  Filter: {}", f);
                    }
                    println!("  Concurrency: {}", concurrency.min(50));
                    if dry_run {
                        println!("  {} Dry-run mode enabled", "⚠".yellow());
                    }
                    println!();
                }

                // Create API clients
                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                );
                let users_client = Arc::new(ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                ));

                let progress_bar = create_bulk_progress_bar(output_format);
                let on_progress = make_progress_callback(progress_bar.clone());

                let result = bulk_add_user(
                    &admin_client,
                    users_client,
                    &account_id,
                    &email,
                    role.as_deref(),
                    &project_filter,
                    bulk_config,
                    on_progress,
                )
                .await?;

                // Finish progress bar
                if let Some(pb) = progress_bar {
                    pb.finish_and_clear();
                }

                // Display results
                display_bulk_result(&result, output_format)?;

                // Exit with appropriate code
                if result.failed > 0 {
                    anyhow::bail!(
                        "Bulk operation partially failed: {} items failed",
                        result.failed
                    );
                }

                Ok(())
            }

            UserCommands::Remove {
                email,
                account,
                filter,
                project_ids,
                concurrency,
                dry_run,
                yes: _,
            } => {
                let concurrency = concurrency.unwrap_or(global_concurrency);
                let account_id = get_account_id(account)?;
                let project_filter = parse_filter_with_ids(&filter, &project_ids)?;

                let bulk_config = BulkConfig {
                    concurrency: concurrency.min(50),
                    dry_run,
                    ..Default::default()
                };

                if output_format.supports_colors() {
                    println!(
                        "\n{} Bulk remove user: {} from account {}",
                        "→".cyan(),
                        email.red(),
                        account_id.cyan()
                    );
                    if let Some(f) = &filter {
                        println!("  Filter: {}", f);
                    }
                    println!("  Concurrency: {}", concurrency.min(50));
                    if dry_run {
                        println!("  {} Dry-run mode enabled", "⚠".yellow());
                    }
                    println!();
                }

                // Create API clients
                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                );
                let users_client = Arc::new(ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                ));

                let progress_bar = create_bulk_progress_bar(output_format);
                let on_progress = make_progress_callback(progress_bar.clone());

                let result = raps_admin::bulk_remove_user(
                    &admin_client,
                    users_client,
                    &account_id,
                    &email,
                    &project_filter,
                    bulk_config,
                    on_progress,
                )
                .await?;

                // Finish progress bar
                if let Some(pb) = progress_bar {
                    pb.finish_and_clear();
                }

                // Display results
                display_bulk_result(&result, output_format)?;

                // Exit with appropriate code
                if result.failed > 0 {
                    anyhow::bail!(
                        "Bulk operation partially failed: {} items failed",
                        result.failed
                    );
                }

                Ok(())
            }

            UserCommands::Update {
                email,
                account,
                role,
                company,
                from_role,
                filter,
                project_ids,
                from_csv,
                concurrency,
                dry_run,
                yes: _,
            } => {
                let concurrency = concurrency.unwrap_or(global_concurrency);
                // Handle --from-csv mode
                if let Some(csv_path) = from_csv {
                    return execute_csv_update(
                        config,
                        auth_client,
                        account.clone(),
                        filter.clone(),
                        project_ids.clone(),
                        &csv_path,
                        concurrency,
                        dry_run,
                        output_format,
                    )
                    .await;
                }

                // Validate: at least --role or --company must be provided
                if role.is_none() && company.is_none() {
                    anyhow::bail!("At least one of --role or --company must be provided.");
                }

                let account_id = get_account_id(account)?;

                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                );

                // Handle company update at account level
                if let Some(ref company_name) = company {
                    if output_format.supports_colors() {
                        println!(
                            "\n{} Update company for user: {} to: {}",
                            "→".cyan(),
                            email.green(),
                            company_name.yellow()
                        );
                        if dry_run {
                            println!("  {} Dry-run mode enabled", "⚠".yellow());
                        }
                    }

                    if !dry_run {
                        // Look up user by email to get user ID
                        let user = admin_client
                            .find_user_by_email(&account_id, &email)
                            .await?
                            .ok_or_else(|| anyhow::anyhow!("User not found: {}", email))?;

                        let update_req = raps_acc::admin::UpdateAccountUserRequest {
                            company_id: None,
                            company_name: Some(company_name.clone()),
                        };

                        admin_client
                            .update_user(&account_id, &user.id, update_req)
                            .await?;

                        if output_format.supports_colors() {
                            println!(
                                "{} Company updated for {} to '{}'",
                                "✓".green().bold(),
                                email,
                                company_name
                            );
                        }
                    } else if output_format.supports_colors() {
                        println!(
                            "  {} Would update company for {} to '{}'",
                            "→".dimmed(),
                            email,
                            company_name
                        );
                    }
                }

                // Handle role update across projects (if --role is provided)
                if let Some(ref role_value) = role {
                    let project_filter = parse_filter_with_ids(&filter, &project_ids)?;

                    let bulk_config = BulkConfig {
                        concurrency: concurrency.min(50),
                        dry_run,
                        ..Default::default()
                    };

                    if output_format.supports_colors() {
                        println!(
                            "\n{} Bulk update user: {} to role: {}",
                            "→".cyan(),
                            email.green(),
                            role_value.yellow()
                        );
                        if let Some(fr) = &from_role {
                            println!("  From role: {}", fr);
                        }
                        if let Some(f) = &filter {
                            println!("  Filter: {}", f);
                        }
                        println!("  Concurrency: {}", concurrency.min(50));
                        if dry_run {
                            println!("  {} Dry-run mode enabled", "⚠".yellow());
                        }
                        println!();
                    }

                    let users_client = Arc::new(ProjectUsersClient::new_with_http_config(
                        config.clone(),
                        auth_client.clone(),
                        http_config,
                    ));

                    let progress_bar = create_bulk_progress_bar(output_format);
                    let on_progress = make_progress_callback(progress_bar.clone());

                    let result = raps_admin::bulk_update_role(
                        &admin_client,
                        users_client,
                        &account_id,
                        &email,
                        role_value,
                        from_role.as_deref(),
                        &project_filter,
                        bulk_config,
                        on_progress,
                    )
                    .await?;

                    // Finish progress bar
                    if let Some(pb) = progress_bar {
                        pb.finish_and_clear();
                    }

                    // Display results
                    display_bulk_result(&result, output_format)?;

                    // Exit with appropriate code
                    if result.failed > 0 {
                        anyhow::bail!(
                            "Bulk operation partially failed: {} items failed",
                            result.failed
                        );
                    }
                }

                Ok(())
            }

            UserCommands::AddToProject {
                project,
                email,
                role_id,
            } => {
                let http_config = HttpClientConfig::default();
                let users_client = ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                );

                if output_format.supports_colors() {
                    println!(
                        "\n{} Adding user {} to project {}",
                        "→".cyan(),
                        email.cyan(),
                        project.cyan()
                    );
                }

                let request = raps_acc::users::AddProjectUserRequest {
                    email: email.clone(),
                    role_id: role_id.clone(),
                    products: vec![],
                };

                let user = users_client.add_user(&project, request).await?;

                #[derive(Serialize, schemars::JsonSchema)]
                struct AddResult {
                    user_id: String,
                    email: String,
                    role: Option<String>,
                    project: String,
                }

                let result = AddResult {
                    user_id: user.id,
                    email: user.email.unwrap_or(email),
                    role: user.role_name,
                    project,
                };

                output_format.write(&result)?;

                if output_format.supports_colors() {
                    println!("\n{} User added successfully", "✓".green());
                }

                Ok(())
            }

            UserCommands::RemoveFromProject {
                project,
                user_id,
                yes,
            } => {
                let http_config = HttpClientConfig::default();
                let users_client = ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                );

                if !yes && output_format.supports_colors() {
                    println!(
                        "\n{} Remove user {} from project {}?",
                        "⚠".yellow(),
                        user_id.cyan(),
                        project.cyan()
                    );
                    print!("Continue? [y/N] ");
                    use std::io::{self, Write};
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    if !input.trim().eq_ignore_ascii_case("y") {
                        println!("Cancelled.");
                        return Ok(());
                    }
                }

                users_client.remove_user(&project, &user_id).await?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} User {} removed from project {}",
                        "✓".green(),
                        user_id.cyan(),
                        project.cyan()
                    );
                } else {
                    println!("User {} removed from project {}", user_id, project);
                }

                Ok(())
            }

            UserCommands::UpdateInProject {
                project,
                user_id,
                role_id,
            } => {
                let http_config = HttpClientConfig::default();
                let users_client = ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                );

                if output_format.supports_colors() {
                    println!(
                        "\n{} Updating user {} in project {}",
                        "→".cyan(),
                        user_id.cyan(),
                        project.cyan()
                    );
                }

                let request = raps_acc::users::UpdateProjectUserRequest {
                    role_id: role_id.clone(),
                    products: None,
                };

                let user = users_client
                    .update_user(&project, &user_id, request)
                    .await?;

                #[derive(Serialize, schemars::JsonSchema)]
                struct UpdateResult {
                    user_id: String,
                    email: Option<String>,
                    role: Option<String>,
                    project: String,
                }

                let result = UpdateResult {
                    user_id: user.id,
                    email: user.email,
                    role: user.role_name,
                    project,
                };

                output_format.write(&result)?;

                if output_format.supports_colors() {
                    println!("\n{} User updated successfully", "✓".green());
                }

                Ok(())
            }

            UserCommands::Import { project, from_csv } => {
                execute_csv_import(config, auth_client, &project, &from_csv, output_format).await
            }
        }
    }
}

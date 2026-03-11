// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! User management command implementations

use std::sync::Arc;

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use raps_acc::admin::AccountAdminClient;
use raps_acc::admin::ResolvedRole;
use raps_acc::users::ProjectUsersClient;
use raps_admin::{BulkConfig, bulk_add_user};
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

use crate::output::OutputFormat;

use super::csv_ops::{execute_csv_import, execute_csv_update};
use super::operations::display_bulk_result;
use raps_dm::DataManagementClient;

use super::{
    UserCommands, create_bulk_progress_bar, make_progress_callback, parse_filter_with_ids,
    resolve_account_id,
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
        dm_client: &DataManagementClient,
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
                let account_id = resolve_account_id(account, dm_client).await?;
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
                    )?;

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
                    )?;

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
                delay_ms,
                dry_run,
                yes: _,
            } => {
                let concurrency = concurrency.unwrap_or(global_concurrency);
                let account_id = resolve_account_id(account, dm_client).await?;
                let project_filter = parse_filter_with_ids(&filter, &project_ids)?;

                let bulk_config = BulkConfig {
                    concurrency: concurrency.min(50),
                    dry_run,
                    delay_ms: delay_ms.unwrap_or(0),
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
                    if let Some(d) = delay_ms {
                        println!("  Delay: {}ms between requests", d);
                    }
                    if dry_run {
                        println!("  {} Dry-run mode enabled", "⚠".yellow());
                    }
                    println!();
                }

                // Create API clients — scale connection pool with concurrency
                let effective_concurrency = concurrency.min(50);
                let http_config = HttpClientConfig {
                    pool_max_idle_per_host: effective_concurrency.max(10),
                    ..HttpClientConfig::default()
                };
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                )?;
                let mut users_client = ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                )?;
                users_client.account_id = Some(account_id.clone());
                let users_client = Arc::new(users_client);

                // Resolve role name to either a BIM 360 UUID or ACC product access list
                let (resolved_role_id, resolved_products): (Option<String>, Vec<raps_acc::types::ProductAccess>) = if let Some(ref role_name) = role {
                    match admin_client.resolve_role(&account_id, role_name).await? {
                        ResolvedRole::Uuid(id) => (Some(id), vec![]),
                        ResolvedRole::Products(products) => (None, products),
                    }
                } else {
                    (None, vec![])
                };

                let progress_bar = create_bulk_progress_bar(output_format);
                let on_progress = make_progress_callback(progress_bar.clone());

                let result = bulk_add_user(
                    &admin_client,
                    users_client,
                    &account_id,
                    &email,
                    resolved_role_id.as_deref(),
                    resolved_products,
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
                delay_ms,
                dry_run,
                yes: _,
            } => {
                let concurrency = concurrency.unwrap_or(global_concurrency);
                let account_id = resolve_account_id(account, dm_client).await?;
                let project_filter = parse_filter_with_ids(&filter, &project_ids)?;

                let bulk_config = BulkConfig {
                    concurrency: concurrency.min(50),
                    dry_run,
                    delay_ms: delay_ms.unwrap_or(0),
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
                    if let Some(d) = delay_ms {
                        println!("  Delay: {}ms between requests", d);
                    }
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
                )?;
                let users_client = Arc::new(ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                )?);

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
                delay_ms,
                dry_run,
                yes: _,
            } => {
                let concurrency = concurrency.unwrap_or(global_concurrency);
                // Handle --from-csv mode
                if let Some(csv_path) = from_csv {
                    return execute_csv_update(
                        config,
                        auth_client,
                        dm_client,
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

                let account_id = resolve_account_id(account, dm_client).await?;

                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                )?;

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
                        delay_ms: delay_ms.unwrap_or(0),
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
                    )?);

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
                role,
                account,
            } => {
                let account_id = account.or_else(|| std::env::var("APS_ACCOUNT_ID").ok());
                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                )?;
                let mut users_client = ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                )?;
                users_client.account_id = account_id.clone();

                if output_format.supports_colors() {
                    println!(
                        "\n{} Adding user {} to project {}",
                        "→".cyan(),
                        email.cyan(),
                        project.cyan()
                    );
                }

                // Resolve role → either BIM 360 UUID (role_ids) or ACC products.
                // Default to "member" when no role is given, so the ACC endpoint always
                // receives a non-empty products array (empty array causes HTTP 500).
                let role_name = role.as_deref().unwrap_or("member");
                let (resolved_role_ids, resolved_products) = if let Some(ref aid) = account_id {
                    match admin_client.resolve_role(aid, role_name).await? {
                        ResolvedRole::Uuid(id) => (vec![id], vec![]),
                        ResolvedRole::Products(products) => (vec![], products),
                    }
                } else {
                    // No account_id → assume ACC hub, map by name directly
                    match admin_client.resolve_role("", role_name).await {
                        Ok(ResolvedRole::Products(products)) => (vec![], products),
                        _ => {
                            anyhow::bail!(
                                "Could not resolve role {:?}. \
                                 Known ACC roles: admin, member, editor, viewer. \
                                 For BIM 360 UUIDs, also provide --account.",
                                role_name
                            )
                        }
                    }
                };

                let request = raps_acc::users::AddProjectUserRequest {
                    email: email.clone(),
                    role_ids: resolved_role_ids,
                    products: resolved_products,
                    suppress_administrative_emails: false,
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
                )?;

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
                )?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} Updating user {} in project {}",
                        "→".cyan(),
                        user_id.cyan(),
                        project.cyan()
                    );
                }

                let request = raps_acc::users::UpdateProjectUserRequest {
                    role_ids: role_id.clone().map(|s| vec![s]).unwrap_or_default(),
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

            UserCommands::ExportPermissions {
                email,
                account,
                folders,
                output,
                concurrency,
            } => {
                let account_id = resolve_account_id(account, dm_client).await?;
                export_permissions(
                    config.clone(),
                    auth_client.clone(),
                    &account_id,
                    &email,
                    folders,
                    output.as_deref(),
                    concurrency,
                    output_format,
                )
                .await
            }

            UserCommands::ClonePermissions {
                from,
                to,
                account,
                folders,
                concurrency,
                dry_run,
                yes,
            } => {
                let account_id = resolve_account_id(account, dm_client).await?;
                clone_permissions(
                    config.clone(),
                    auth_client.clone(),
                    &account_id,
                    &from,
                    &to,
                    folders,
                    concurrency,
                    dry_run,
                    yes,
                    output_format,
                )
                .await
            }

            UserCommands::AddToAllProjects {
                email,
                account,
                role,
                concurrency,
                dry_run,
            } => {
                let concurrency = concurrency.unwrap_or(global_concurrency).min(50);
                let account_id = resolve_account_id(account, dm_client).await?;
                let http_config = HttpClientConfig::default();

                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                )?;
                let users_client = ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config,
                )?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} Add user {} to all active projects in account {}",
                        "→".cyan(),
                        email.green(),
                        account_id.cyan()
                    );
                    if let Some(ref r) = role {
                        println!("  Role: {}", r.yellow());
                    }
                    println!("  Concurrency: {}", concurrency);
                    if dry_run {
                        println!("  {} Dry-run mode enabled", "⚠".yellow());
                    }
                    println!();
                }

                // Fetch all projects and filter to active
                let all_projects = admin_client.list_all_projects(&account_id).await?;
                let active_projects: Vec<_> = all_projects
                    .into_iter()
                    .filter(|p| {
                        p.status
                            .as_deref()
                            .map(|s| s.eq_ignore_ascii_case("active"))
                            .unwrap_or(false)
                    })
                    .collect();

                if active_projects.is_empty() {
                    if output_format.supports_colors() {
                        println!("{}", "No active projects found.".yellow());
                    }
                    return Ok(());
                }

                if output_format.supports_colors() {
                    println!(
                        "  Found {} active project(s)",
                        active_projects.len().to_string().cyan()
                    );
                    println!();
                }

                if dry_run {
                    #[derive(Serialize, schemars::JsonSchema)]
                    struct DryRunOutput {
                        email: String,
                        #[serde(skip_serializing_if = "Option::is_none")]
                        role: Option<String>,
                        projects: Vec<DryRunProject>,
                    }
                    #[derive(Serialize, schemars::JsonSchema)]
                    struct DryRunProject {
                        id: String,
                        name: String,
                    }

                    let projects: Vec<DryRunProject> = active_projects
                        .iter()
                        .map(|p| DryRunProject {
                            id: p.id.clone(),
                            name: p.name.clone(),
                        })
                        .collect();

                    match output_format {
                        OutputFormat::Table => {
                            for p in &projects {
                                println!(
                                    "  {} Would add to: {} ({})",
                                    "→".dimmed(),
                                    p.name.cyan(),
                                    p.id.dimmed()
                                );
                            }
                            println!();
                            println!(
                                "{} Dry run: {} project(s) would be affected",
                                "✓".green().bold(),
                                projects.len()
                            );
                        }
                        _ => {
                            output_format.write(&DryRunOutput {
                                email: email.clone(),
                                role: role.clone(),
                                projects,
                            })?;
                        }
                    }
                    return Ok(());
                }

                // Execute concurrently
                use futures_util::stream::{self, StreamExt};
                use std::sync::atomic::{AtomicUsize, Ordering};

                let succeeded = AtomicUsize::new(0);
                let failed = AtomicUsize::new(0);
                let skipped = AtomicUsize::new(0);

                #[derive(Serialize, schemars::JsonSchema)]
                struct ProjectResult {
                    project_id: String,
                    project_name: String,
                    status: String,
                    #[serde(skip_serializing_if = "Option::is_none")]
                    error: Option<String>,
                }

                let results: Vec<ProjectResult> = stream::iter(active_projects)
                    .map(|project| {
                        let users_client = &users_client;
                        let email = &email;
                        let role = &role;
                        let succeeded = &succeeded;
                        let failed = &failed;
                        let skipped = &skipped;
                        async move {
                            let request = raps_acc::users::AddProjectUserRequest {
                                email: email.clone(),
                                role_ids: role.clone().map(|s| vec![s]).unwrap_or_default(),
                                products: vec![],
                                suppress_administrative_emails: false,
                            };
                            match users_client.add_user(&project.id, request).await {
                                Ok(_) => {
                                    succeeded.fetch_add(1, Ordering::Relaxed);
                                    ProjectResult {
                                        project_id: project.id,
                                        project_name: project.name,
                                        status: "added".to_string(),
                                        error: None,
                                    }
                                }
                                Err(e) => {
                                    let err_str = e.to_string();
                                    if err_str.contains("409")
                                        || err_str.to_lowercase().contains("already")
                                        || err_str.to_lowercase().contains("exists")
                                    {
                                        skipped.fetch_add(1, Ordering::Relaxed);
                                        ProjectResult {
                                            project_id: project.id,
                                            project_name: project.name,
                                            status: "already_exists".to_string(),
                                            error: None,
                                        }
                                    } else {
                                        failed.fetch_add(1, Ordering::Relaxed);
                                        ProjectResult {
                                            project_id: project.id,
                                            project_name: project.name,
                                            status: "failed".to_string(),
                                            error: Some(err_str),
                                        }
                                    }
                                }
                            }
                        }
                    })
                    .buffer_unordered(concurrency)
                    .collect()
                    .await;

                let total = results.len();
                let ok = succeeded.load(Ordering::Relaxed);
                let skip = skipped.load(Ordering::Relaxed);
                let fail = failed.load(Ordering::Relaxed);

                match output_format {
                    OutputFormat::Table => {
                        for r in &results {
                            let icon = match r.status.as_str() {
                                "added" => "✓".green().to_string(),
                                "already_exists" => "○".yellow().to_string(),
                                _ => "✗".red().to_string(),
                            };
                            print!(
                                "  {} {} ({})",
                                icon,
                                r.project_name.cyan(),
                                r.project_id.dimmed()
                            );
                            if let Some(ref e) = r.error {
                                print!(" — {}", e.red());
                            }
                            println!();
                        }

                        println!();
                        println!("{}", "─".repeat(60));
                        println!(
                            "  Total: {}  Added: {}  Already existed: {}  Failed: {}",
                            total,
                            ok.to_string().green(),
                            skip.to_string().yellow(),
                            fail.to_string().red()
                        );
                    }
                    _ => {
                        #[derive(Serialize, schemars::JsonSchema)]
                        struct AddToAllOutput {
                            email: String,
                            total: usize,
                            added: usize,
                            already_existed: usize,
                            failed: usize,
                            results: Vec<ProjectResult>,
                        }
                        output_format.write(&AddToAllOutput {
                            email: email.clone(),
                            total,
                            added: ok,
                            already_existed: skip,
                            failed: fail,
                            results,
                        })?;
                    }
                }

                if fail > 0 {
                    anyhow::bail!("{} project(s) failed", fail);
                }

                Ok(())
            }
        }
    }
}

async fn export_permissions(
    config: Config,
    auth_client: AuthClient,
    account_id: &str,
    email: &str,
    include_folders: bool,
    output_path: Option<&str>,
    concurrency: usize,
    output_format: OutputFormat,
) -> Result<()> {
    use futures_util::stream::{self, StreamExt};
    use raps_acc::permissions::FolderPermissionsClient;

    let http_config = HttpClientConfig {
        pool_max_idle_per_host: concurrency.max(10),
        ..HttpClientConfig::default()
    };
    let admin_client = AccountAdminClient::new_with_http_config(
        config.clone(),
        auth_client.clone(),
        http_config.clone(),
    )?;
    let users_client = ProjectUsersClient::new_with_http_config(
        config.clone(),
        auth_client.clone(),
        http_config.clone(),
    )?;

    // Look up user to validate email
    let account_user = admin_client
        .find_user_by_email(account_id, email)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User not found in account: {email}"))?;
    let user_id = account_user.id.clone();

    // List all active projects
    let all_projects = admin_client.list_all_projects(account_id).await?;
    let total = all_projects.len();

    if output_format.supports_colors() {
        eprintln!(
            "Scanning {} projects for user {}...",
            total, email
        );
    }

    // For each project, check if user is a member (using filter[email])
    #[derive(serde::Serialize)]
    struct PermissionRow {
        project_id: String,
        project_name: String,
        role: String,
        products: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        folder_path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        permission_level: Option<String>,
    }

    let rows: Vec<Option<PermissionRow>> = stream::iter(all_projects)
        .map(|project| {
            let users_client = &users_client;
            let email = email;
            let include_folders = include_folders;
            async move {
                match users_client
                    .find_project_user_by_email(&project.id, email)
                    .await
                {
                    Ok(Some(project_user)) => {
                        let role = project_user
                            .role_name
                            .unwrap_or_else(|| "unknown".to_string());
                        let products = project_user
                            .products
                            .map(|ps| {
                                ps.iter()
                                    .map(|p| p.key.clone())
                                    .collect::<Vec<_>>()
                                    .join("; ")
                            })
                            .unwrap_or_default();
                        Some(PermissionRow {
                            project_id: project.id,
                            project_name: project.name,
                            role,
                            products,
                            folder_path: if include_folders {
                                Some(String::new())
                            } else {
                                None
                            },
                            permission_level: if include_folders {
                                Some(String::new())
                            } else {
                                None
                            },
                        })
                    }
                    Ok(None) => None,
                    Err(_) => None, // Skip errors (e.g. 403/404)
                }
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let mut rows: Vec<PermissionRow> = rows.into_iter().flatten().collect();

    // If --folders, fetch folder permissions for projects where user exists
    if include_folders {
        let perms_client = FolderPermissionsClient::new_with_http_config(
            config.clone(),
            auth_client.clone(),
            http_config,
        )?;

        // Collect project IDs that need folder permission lookups
        let project_ids: Vec<String> = rows.iter().map(|r| r.project_id.clone()).collect();

        // Fetch folder permissions concurrently
        let folder_results: Vec<(String, Option<Vec<raps_acc::permissions::FolderPermission>>)> =
            stream::iter(project_ids)
                .map(|pid| {
                    let perms_client = &perms_client;
                    async move {
                        let perms = match perms_client.get_project_files_folder_id(&pid).await {
                            Ok(folder_id) => perms_client
                                .get_permissions(&pid, &folder_id)
                                .await
                                .ok(),
                            Err(_) => None,
                        };
                        (pid, perms)
                    }
                })
                .buffer_unordered(concurrency)
                .collect()
                .await;

        // Match folder permissions back to rows
        for (project_id, perms) in folder_results {
            if let Some(perms) = perms {
                for perm in &perms {
                    if perm.subject_id == user_id && perm.subject_type == "USER" {
                        let level = actions_to_level(&perm.actions);
                        if let Some(row) = rows.iter_mut().find(|r| r.project_id == project_id) {
                            row.folder_path = Some("Project Files".to_string());
                            row.permission_level = Some(level);
                        }
                    }
                }
            }
        }
    }

    // Sort by project name
    rows.sort_by(|a, b| a.project_name.cmp(&b.project_name));

    if output_format.supports_colors() {
        eprintln!("Found user in {} of {} projects", rows.len(), total);
    }

    // Output
    if let Some(path) = output_path {
        let mut wtr = csv::Writer::from_path(path)?;
        for row in &rows {
            wtr.serialize(row)?;
        }
        wtr.flush()?;
        if output_format.supports_colors() {
            eprintln!("Written to {path}");
        }
    } else {
        output_format.write(&rows)?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn clone_permissions(
    config: Config,
    auth_client: AuthClient,
    account_id: &str,
    source_email: &str,
    target_email: &str,
    include_folders: bool,
    concurrency: usize,
    dry_run: bool,
    yes: bool,
    output_format: OutputFormat,
) -> Result<()> {
    use futures_util::stream::{self, StreamExt};
    use raps_acc::permissions::FolderPermissionsClient;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let http_config = HttpClientConfig {
        pool_max_idle_per_host: concurrency.max(10),
        ..HttpClientConfig::default()
    };
    let admin_client = AccountAdminClient::new_with_http_config(
        config.clone(),
        auth_client.clone(),
        http_config.clone(),
    )?;
    let users_client = ProjectUsersClient::new_with_http_config(
        config.clone(),
        auth_client.clone(),
        http_config.clone(),
    )?;

    // Validate both users exist in the account
    let source_user = admin_client
        .find_user_by_email(account_id, source_email)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Source user not found in account: {source_email}"))?;
    let target_user = admin_client
        .find_user_by_email(account_id, target_email)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Target user not found in account: {target_email}"))?;

    // List all projects
    let all_projects = admin_client.list_all_projects(account_id).await?;
    let total_projects = all_projects.len();

    if output_format.supports_colors() {
        eprintln!(
            "Scanning {} projects for source user {}...",
            total_projects, source_email
        );
    }

    // Scan: find projects where source user is a member, capture their role/products
    #[derive(Clone)]
    struct SourceMembership {
        project_id: String,
        project_name: String,
        role_ids: Vec<String>,
        products: Vec<raps_acc::types::ProductAccess>,
    }

    let memberships: Vec<Option<SourceMembership>> = stream::iter(all_projects)
        .map(|project| {
            let users_client = &users_client;
            let source_email = source_email;
            async move {
                match users_client
                    .find_project_user_by_email(&project.id, source_email)
                    .await
                {
                    Ok(Some(pu)) => Some(SourceMembership {
                        project_id: project.id,
                        project_name: project.name,
                        role_ids: pu.role_ids.clone(),
                        products: pu
                            .products
                            .clone()
                            .unwrap_or_default(),
                    }),
                    _ => None,
                }
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    let memberships: Vec<SourceMembership> = memberships.into_iter().flatten().collect();

    if memberships.is_empty() {
        if output_format.supports_colors() {
            eprintln!("Source user {source_email} is not a member of any project.");
        }
        return Ok(());
    }

    if output_format.supports_colors() {
        eprintln!(
            "Source user is a member of {} project(s) out of {}",
            memberships.len(),
            total_projects
        );
    }

    // Dry run: show what would happen
    if dry_run {
        #[derive(Serialize, schemars::JsonSchema)]
        struct DryRunOutput {
            source: String,
            target: String,
            projects: Vec<DryRunProject>,
        }
        #[derive(Serialize, schemars::JsonSchema)]
        struct DryRunProject {
            id: String,
            name: String,
            role_ids: Vec<String>,
        }

        let projects: Vec<DryRunProject> = memberships
            .iter()
            .map(|m| DryRunProject {
                id: m.project_id.clone(),
                name: m.project_name.clone(),
                role_ids: m.role_ids.clone(),
            })
            .collect();

        match output_format {
            OutputFormat::Table => {
                for p in &projects {
                    println!(
                        "  {} {} ({}) — roles: [{}]",
                        "→".dimmed(),
                        p.name.cyan(),
                        p.id.dimmed(),
                        p.role_ids.join(", ")
                    );
                }
                println!();
                println!(
                    "{} Dry run: would clone permissions to {} in {} project(s)",
                    "✓".green().bold(),
                    target_email.green(),
                    projects.len()
                );
                if include_folders {
                    println!("  Including folder-level permissions");
                }
            }
            _ => {
                output_format.write(&DryRunOutput {
                    source: source_email.to_string(),
                    target: target_email.to_string(),
                    projects,
                })?;
            }
        }
        return Ok(());
    }

    // Confirmation prompt
    if !yes && output_format.supports_colors() {
        println!(
            "\n{} Clone permissions from {} to {} across {} project(s)?",
            "⚠".yellow(),
            source_email.cyan(),
            target_email.green(),
            memberships.len()
        );
        if include_folders {
            println!("  Including folder-level permissions");
        }
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

    // Execute: add/update target user in each project with source's role/products
    let succeeded = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);
    let skipped = AtomicUsize::new(0);

    #[derive(Serialize, schemars::JsonSchema)]
    struct CloneResult {
        project_id: String,
        project_name: String,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    }

    let results: Vec<CloneResult> = stream::iter(memberships.clone())
        .map(|membership| {
            let users_client = &users_client;
            let target_email = target_email;
            let succeeded = &succeeded;
            let failed = &failed;
            let skipped = &skipped;
            async move {
                // Try to add user first
                let request = raps_acc::users::AddProjectUserRequest {
                    email: target_email.to_string(),
                    role_ids: membership.role_ids.clone(),
                    products: membership.products.clone(),
                    suppress_administrative_emails: false,
                };
                match users_client.add_user(&membership.project_id, request).await {
                    Ok(_) => {
                        succeeded.fetch_add(1, Ordering::Relaxed);
                        CloneResult {
                            project_id: membership.project_id,
                            project_name: membership.project_name,
                            status: "added".to_string(),
                            error: None,
                        }
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        // User already exists — try updating their role instead
                        if err_str.contains("409")
                            || err_str.to_lowercase().contains("already")
                            || err_str.to_lowercase().contains("exists")
                        {
                            match users_client
                                .find_project_user_by_email(
                                    &membership.project_id,
                                    target_email,
                                )
                                .await
                            {
                                Ok(Some(existing)) => {
                                    // Check if roles already match
                                    if existing.role_ids == membership.role_ids {
                                        skipped.fetch_add(1, Ordering::Relaxed);
                                        return CloneResult {
                                            project_id: membership.project_id,
                                            project_name: membership.project_name,
                                            status: "already_matching".to_string(),
                                            error: None,
                                        };
                                    }
                                    // Update role
                                    let update_req =
                                        raps_acc::users::UpdateProjectUserRequest {
                                            role_ids: membership.role_ids.clone(),
                                            products: Some(membership.products.clone()),
                                        };
                                    match users_client
                                        .update_user(
                                            &membership.project_id,
                                            &existing.id,
                                            update_req,
                                        )
                                        .await
                                    {
                                        Ok(_) => {
                                            succeeded.fetch_add(1, Ordering::Relaxed);
                                            CloneResult {
                                                project_id: membership.project_id,
                                                project_name: membership.project_name,
                                                status: "updated".to_string(),
                                                error: None,
                                            }
                                        }
                                        Err(ue) => {
                                            failed.fetch_add(1, Ordering::Relaxed);
                                            CloneResult {
                                                project_id: membership.project_id,
                                                project_name: membership.project_name,
                                                status: "failed".to_string(),
                                                error: Some(ue.to_string()),
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    skipped.fetch_add(1, Ordering::Relaxed);
                                    CloneResult {
                                        project_id: membership.project_id,
                                        project_name: membership.project_name,
                                        status: "skipped".to_string(),
                                        error: Some(
                                            "user exists but could not look up".to_string(),
                                        ),
                                    }
                                }
                            }
                        } else {
                            failed.fetch_add(1, Ordering::Relaxed);
                            CloneResult {
                                project_id: membership.project_id,
                                project_name: membership.project_name,
                                status: "failed".to_string(),
                                error: Some(err_str),
                            }
                        }
                    }
                }
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    // Clone folder permissions if requested
    let mut folder_cloned = 0usize;
    let mut folder_failed = 0usize;
    if include_folders {
        let perms_client = FolderPermissionsClient::new_with_http_config(
            config.clone(),
            auth_client.clone(),
            http_config,
        )?;

        let source_id = &source_user.id;
        let target_id = &target_user.id;

        // Only clone folders for projects where the target was successfully added/updated
        let ok_project_ids: Vec<String> = results
            .iter()
            .filter(|r| r.status == "added" || r.status == "updated" || r.status == "already_matching")
            .map(|r| r.project_id.clone())
            .collect();

        let folder_results: Vec<bool> = stream::iter(ok_project_ids)
            .map(|pid| {
                let perms_client = &perms_client;
                let source_id = source_id;
                let target_id = target_id;
                async move {
                    // Get the Project Files folder
                    let folder_id = match perms_client.get_project_files_folder_id(&pid).await {
                        Ok(fid) => fid,
                        Err(_) => return false,
                    };

                    // Get current permissions
                    let perms = match perms_client.get_permissions(&pid, &folder_id).await {
                        Ok(p) => p,
                        Err(_) => return false,
                    };

                    // Find source user's permissions
                    let source_perm = perms.iter().find(|p| {
                        p.subject_id == *source_id && p.subject_type == "USER"
                    });

                    let Some(source_perm) = source_perm else {
                        return true; // No folder perms to clone
                    };

                    // Apply to target user
                    let request = raps_acc::permissions::BatchUpdatePermissionsRequest {
                        permissions: vec![raps_acc::permissions::UpdatePermissionRequest {
                            subject_id: target_id.to_string(),
                            subject_type: "USER".to_string(),
                            actions: source_perm.actions.clone(),
                        }],
                    };

                    perms_client
                        .update_permissions(&pid, &folder_id, request)
                        .await
                        .is_ok()
                }
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

        for ok in &folder_results {
            if *ok {
                folder_cloned += 1;
            } else {
                folder_failed += 1;
            }
        }
    }

    // Display results
    let total = results.len();
    let ok = succeeded.load(Ordering::Relaxed);
    let skip = skipped.load(Ordering::Relaxed);
    let fail = failed.load(Ordering::Relaxed);

    match output_format {
        OutputFormat::Table => {
            for r in &results {
                let icon = match r.status.as_str() {
                    "added" | "updated" => "✓".green().to_string(),
                    "already_matching" | "skipped" => "○".yellow().to_string(),
                    _ => "✗".red().to_string(),
                };
                print!(
                    "  {} {} ({}) [{}]",
                    icon,
                    r.project_name.cyan(),
                    r.project_id.dimmed(),
                    r.status
                );
                if let Some(ref e) = r.error {
                    print!(" — {}", e.red());
                }
                println!();
            }

            println!();
            println!("{}", "─".repeat(60));
            println!(
                "  Projects: {}  Cloned: {}  Already matched: {}  Failed: {}",
                total,
                ok.to_string().green(),
                skip.to_string().yellow(),
                fail.to_string().red()
            );
            if include_folders {
                println!(
                    "  Folder permissions: {} cloned, {} failed",
                    folder_cloned.to_string().green(),
                    folder_failed.to_string().red()
                );
            }
        }
        _ => {
            #[derive(Serialize, schemars::JsonSchema)]
            struct CloneOutput {
                source: String,
                target: String,
                total: usize,
                cloned: usize,
                already_matched: usize,
                failed: usize,
                #[serde(skip_serializing_if = "Option::is_none")]
                folder_cloned: Option<usize>,
                #[serde(skip_serializing_if = "Option::is_none")]
                folder_failed: Option<usize>,
                results: Vec<CloneResult>,
            }
            output_format.write(&CloneOutput {
                source: source_email.to_string(),
                target: target_email.to_string(),
                total,
                cloned: ok,
                already_matched: skip,
                failed: fail,
                folder_cloned: if include_folders { Some(folder_cloned) } else { None },
                folder_failed: if include_folders { Some(folder_failed) } else { None },
                results,
            })?;
        }
    }

    if fail > 0 {
        anyhow::bail!("{} project(s) failed during permission cloning", fail);
    }

    Ok(())
}

fn actions_to_level(actions: &[String]) -> String {
    let has = |a: &str| actions.iter().any(|x| x == a);
    if has("CONTROL") {
        "Folder Control".to_string()
    } else if has("EDIT") {
        "View+Download+Upload+Edit".to_string()
    } else if has("PUBLISH") && has("VIEW") {
        "View+Download+Upload".to_string()
    } else if has("PUBLISH") {
        "Upload Only".to_string()
    } else if has("DOWNLOAD") {
        "View+Download".to_string()
    } else if has("VIEW") {
        "View Only".to_string()
    } else {
        actions.join(", ")
    }
}

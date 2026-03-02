// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! CSV-based bulk update and import operations

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

use raps_acc::admin::AccountAdminClient;
use raps_acc::users::{ImportUserRequest, ProjectUsersClient};
use raps_admin::{BulkConfig, ProgressUpdate};
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

use crate::output::OutputFormat;

use super::{create_bulk_progress_bar, get_account_id, parse_filter_with_ids};

// ============================================================================
// CSV UPDATE
// ============================================================================

/// A single row from the CSV update file
#[derive(Debug, serde::Deserialize)]
pub(crate) struct CsvUpdateRow {
    pub(crate) email: String,
    #[serde(default)]
    pub(crate) role: Option<String>,
    #[serde(default)]
    pub(crate) company: Option<String>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct CsvUpdateResultOutput {
    pub(crate) total: usize,
    pub(crate) updated: usize,
    pub(crate) skipped: usize,
    pub(crate) failed: usize,
    pub(crate) errors: Vec<CsvUpdateErrorOutput>,
}

#[derive(Serialize, schemars::JsonSchema)]
pub(crate) struct CsvUpdateErrorOutput {
    pub(crate) email: String,
    pub(crate) error: String,
}

/// Execute bulk user updates from a CSV file
///
/// Expected columns: email (required), role (optional), company (optional)
#[allow(clippy::too_many_arguments)]
pub(crate) async fn execute_csv_update(
    config: &Config,
    auth_client: &AuthClient,
    account: Option<String>,
    filter: Option<String>,
    project_ids: Option<PathBuf>,
    csv_path: &PathBuf,
    concurrency: usize,
    dry_run: bool,
    output_format: OutputFormat,
) -> Result<()> {
    let account_id = get_account_id(account)?;

    // Parse CSV file
    let mut reader = csv::Reader::from_path(csv_path)
        .with_context(|| format!("Failed to open CSV file: {}", csv_path.display()))?;

    let mut rows: Vec<CsvUpdateRow> = Vec::new();
    let mut validation_errors: Vec<String> = Vec::new();

    for (i, result) in reader.deserialize().enumerate() {
        match result {
            Ok(row) => {
                let row: CsvUpdateRow = row;
                // Validate email
                if row.email.is_empty() || !row.email.contains('@') {
                    validation_errors.push(format!("Row {}: invalid email '{}'", i + 2, row.email));
                    continue;
                }
                // Validate at least one field to update
                if row.role.is_none() && row.company.is_none() {
                    validation_errors.push(format!(
                        "Row {}: email '{}' has no role or company to update",
                        i + 2,
                        row.email
                    ));
                    continue;
                }
                rows.push(row);
            }
            Err(e) => {
                validation_errors.push(format!("Row {}: parse error: {}", i + 2, e));
            }
        }
    }

    // Report validation errors
    if !validation_errors.is_empty() {
        if output_format.supports_colors() {
            println!("{} CSV validation errors:", "✗".red().bold());
            for err in &validation_errors {
                println!("  {} {}", "•".red(), err);
            }
        }
        anyhow::bail!(
            "CSV validation failed with {} error(s). Fix errors before proceeding.",
            validation_errors.len()
        );
    }

    if rows.is_empty() {
        anyhow::bail!("No valid rows found in CSV file");
    }

    if output_format.supports_colors() {
        println!(
            "\n{} CSV update: {} rows from {}",
            "→".cyan(),
            rows.len().to_string().green(),
            csv_path.display().to_string().cyan()
        );
        if dry_run {
            println!("  {} Dry-run mode enabled", "⚠".yellow());
        }
        println!();
    }

    let http_config = HttpClientConfig::default();
    let admin_client = AccountAdminClient::new_with_http_config(
        config.clone(),
        auth_client.clone(),
        http_config.clone(),
    );

    let project_filter = parse_filter_with_ids(&filter, &project_ids)?;

    let mut updated = 0usize;
    let skipped = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();

    let progress_bar = create_bulk_progress_bar(output_format);
    if let Some(ref pb) = progress_bar {
        pb.set_length(rows.len() as u64);
    }

    for row in &rows {
        if let Some(ref pb) = progress_bar {
            pb.set_message(row.email.to_string());
        }

        if dry_run {
            if output_format.supports_colors() {
                let mut changes = Vec::new();
                if let Some(ref r) = row.role {
                    changes.push(format!("role={}", r));
                }
                if let Some(ref c) = row.company {
                    changes.push(format!("company={}", c));
                }
                if let Some(ref pb) = progress_bar {
                    pb.println(format!(
                        "  {} {} → {}",
                        "→".dimmed(),
                        row.email,
                        changes.join(", ")
                    ));
                }
            }
            updated += 1;
        } else {
            let mut row_updated = false;

            // Update company at account level if specified
            if let Some(ref company_name) = row.company {
                match admin_client
                    .find_user_by_email(&account_id, &row.email)
                    .await
                {
                    Ok(Some(user)) => {
                        let update_req = raps_acc::admin::UpdateAccountUserRequest {
                            company_id: None,
                            company_name: Some(company_name.clone()),
                        };
                        match admin_client
                            .update_user(&account_id, &user.id, update_req)
                            .await
                        {
                            Ok(_) => {
                                row_updated = true;
                            }
                            Err(e) => {
                                failed += 1;
                                errors.push(CsvUpdateErrorOutput {
                                    email: row.email.clone(),
                                    error: format!("company update failed: {}", e),
                                });
                                if let Some(ref pb) = progress_bar {
                                    pb.inc(1);
                                }
                                continue;
                            }
                        }
                    }
                    Ok(None) => {
                        failed += 1;
                        errors.push(CsvUpdateErrorOutput {
                            email: row.email.clone(),
                            error: "user not found in account".to_string(),
                        });
                        if let Some(ref pb) = progress_bar {
                            pb.inc(1);
                        }
                        continue;
                    }
                    Err(e) => {
                        failed += 1;
                        errors.push(CsvUpdateErrorOutput {
                            email: row.email.clone(),
                            error: format!("user lookup failed: {}", e),
                        });
                        if let Some(ref pb) = progress_bar {
                            pb.inc(1);
                        }
                        continue;
                    }
                }
            }

            // Update role across projects if specified
            if let Some(ref role_value) = row.role {
                let users_client = Arc::new(ProjectUsersClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                ));

                let bulk_config = BulkConfig {
                    concurrency: concurrency.min(50),
                    dry_run: false,
                    ..Default::default()
                };

                let noop_progress = |_: ProgressUpdate| {};

                match raps_admin::bulk_update_role(
                    &admin_client,
                    users_client,
                    &account_id,
                    &row.email,
                    role_value,
                    None,
                    &project_filter,
                    bulk_config,
                    noop_progress,
                )
                .await
                {
                    Ok(result) => {
                        if result.failed > 0 {
                            failed += 1;
                            errors.push(CsvUpdateErrorOutput {
                                email: row.email.clone(),
                                error: format!(
                                    "role update: {}/{} projects failed",
                                    result.failed, result.total
                                ),
                            });
                        } else {
                            row_updated = true;
                        }
                    }
                    Err(e) => {
                        failed += 1;
                        errors.push(CsvUpdateErrorOutput {
                            email: row.email.clone(),
                            error: format!("role update failed: {}", e),
                        });
                    }
                }
            }

            if row_updated {
                updated += 1;
            }
        }

        if let Some(ref pb) = progress_bar {
            pb.inc(1);
        }
    }

    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    let output = CsvUpdateResultOutput {
        total: rows.len(),
        updated,
        skipped,
        failed,
        errors,
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "CSV Update Results:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "Total:".bold(), output.total);
            println!(
                "{:<15} {}",
                "Updated:".bold(),
                output.updated.to_string().green()
            );
            println!(
                "{:<15} {}",
                "Skipped:".bold(),
                output.skipped.to_string().yellow()
            );
            println!(
                "{:<15} {}",
                "Failed:".bold(),
                output.failed.to_string().red()
            );
            println!("{}", "─".repeat(60));

            if !output.errors.is_empty() {
                println!("\n{}", "Errors:".red().bold());
                for err in &output.errors {
                    println!("  {} {} - {}", "✗".red(), err.email, err.error.dimmed());
                }
            }

            if output.failed == 0 {
                println!(
                    "\n{} All {} user(s) updated successfully!",
                    "✓".green().bold(),
                    output.updated
                );
            } else {
                println!(
                    "\n{} Completed with {} failure(s)",
                    "⚠".yellow().bold(),
                    output.failed
                );
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    if output.failed > 0 {
        anyhow::bail!(
            "Bulk operation partially failed: {} items failed",
            output.failed
        );
    }

    Ok(())
}

// ============================================================================
// CSV IMPORT (new users)
// ============================================================================

/// A single row from the CSV import file
#[derive(Debug, serde::Deserialize)]
struct CsvImportRow {
    email: String,
    #[serde(default)]
    role_id: Option<String>,
}

#[derive(Serialize, schemars::JsonSchema)]
struct CsvImportResultOutput {
    total: usize,
    imported: usize,
    failed: usize,
    errors: Vec<CsvImportErrorOutput>,
}

#[derive(Serialize, schemars::JsonSchema)]
struct CsvImportErrorOutput {
    email: String,
    error: String,
}

/// Execute import of new users into a project from a CSV file
///
/// Expected columns: email (required), role_id (optional)
pub(crate) async fn execute_csv_import(
    config: &Config,
    auth_client: &AuthClient,
    project_id: &str,
    csv_path: &PathBuf,
    output_format: OutputFormat,
) -> Result<()> {
    // Parse CSV file
    let mut reader = csv::Reader::from_path(csv_path)
        .with_context(|| format!("Failed to open CSV file: {}", csv_path.display()))?;

    let mut rows: Vec<CsvImportRow> = Vec::new();
    let mut validation_errors: Vec<String> = Vec::new();

    for (i, result) in reader.deserialize().enumerate() {
        match result {
            Ok(row) => {
                let row: CsvImportRow = row;
                // Validate email
                if row.email.is_empty() || !row.email.contains('@') {
                    validation_errors.push(format!("Row {}: invalid email '{}'", i + 2, row.email));
                    continue;
                }
                rows.push(row);
            }
            Err(e) => {
                validation_errors.push(format!("Row {}: parse error: {}", i + 2, e));
            }
        }
    }

    // Report validation errors
    if !validation_errors.is_empty() {
        if output_format.supports_colors() {
            println!("{} CSV validation errors:", "✗".red().bold());
            for err in &validation_errors {
                println!("  {} {}", "•".red(), err);
            }
        }
        anyhow::bail!(
            "CSV validation failed with {} error(s). Fix errors before proceeding.",
            validation_errors.len()
        );
    }

    if rows.is_empty() {
        anyhow::bail!("No valid rows found in CSV file");
    }

    if output_format.supports_colors() {
        println!(
            "\n{} Import users: {} rows from {} into project {}",
            "→".cyan(),
            rows.len().to_string().green(),
            csv_path.display().to_string().cyan(),
            project_id.cyan()
        );
        println!();
    }

    // Build import requests
    let users: Vec<ImportUserRequest> = rows
        .iter()
        .map(|row| ImportUserRequest {
            email: row.email.clone(),
            role_id: row.role_id.clone(),
            products: None,
        })
        .collect();

    let total = users.len();

    // Show spinner during concurrent import (up to 10 parallel requests)
    let spinner = if output_format.supports_colors() {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .expect("hardcoded progress template is valid")
                .tick_strings(&[
                    "\u{280B}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283C}", "\u{2834}",
                    "\u{2826}", "\u{2827}", "\u{2807}", "\u{280F}",
                ]),
        );
        sp.set_message(format!("Importing {} users concurrently...", total));
        sp.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(sp)
    } else {
        None
    };

    // Create users client and call import_users (concurrent with semaphore)
    let http_config = HttpClientConfig::default();
    let users_client =
        ProjectUsersClient::new_with_http_config(config.clone(), auth_client.clone(), http_config);

    let result = users_client.import_users(project_id, users).await?;

    // Finish spinner
    if let Some(sp) = spinner {
        sp.finish_and_clear();
    }

    let errors: Vec<CsvImportErrorOutput> = result
        .errors
        .iter()
        .map(|e| CsvImportErrorOutput {
            email: e.email.clone(),
            error: e.error.clone(),
        })
        .collect();

    let output = CsvImportResultOutput {
        total: result.total,
        imported: result.imported,
        failed: result.failed,
        errors,
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Import Results:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "Total:".bold(), output.total);
            println!(
                "{:<15} {}",
                "Imported:".bold(),
                output.imported.to_string().green()
            );
            println!(
                "{:<15} {}",
                "Failed:".bold(),
                output.failed.to_string().red()
            );
            println!("{}", "─".repeat(60));

            if !output.errors.is_empty() {
                println!("\n{}", "Errors:".red().bold());
                for err in &output.errors {
                    println!("  {} {} - {}", "✗".red(), err.email, err.error.dimmed());
                }
            }

            if output.failed == 0 {
                println!(
                    "\n{} All {} user(s) imported successfully!",
                    "✓".green().bold(),
                    output.imported
                );
            } else {
                println!(
                    "\n{} Completed with {} failure(s)",
                    "⚠".yellow().bold(),
                    output.failed
                );
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    if output.failed > 0 {
        anyhow::bail!(
            "Bulk operation partially failed: {} items failed",
            output.failed
        );
    }

    Ok(())
}

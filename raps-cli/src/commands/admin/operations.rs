// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Operation management command implementations

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use raps_admin::{BulkOperationResult, ItemResult, OperationStatus, StateManager};

use crate::output::OutputFormat;

use super::OperationCommands;

#[derive(Serialize, schemars::JsonSchema)]
struct OperationStatusOutput {
    operation_id: String,
    operation_type: String,
    status: String,
    total: usize,
    completed: usize,
    skipped: usize,
    failed: usize,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, schemars::JsonSchema)]
struct OperationListOutput {
    operation_id: String,
    operation_type: String,
    status: String,
    progress: String,
    updated_at: String,
}

pub(crate) fn format_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "completed" => status.green().to_string(),
        "failed" => status.red().to_string(),
        "inprogress" | "in_progress" => status.yellow().to_string(),
        "cancelled" => status.dimmed().to_string(),
        _ => status.to_string(),
    }
}

/// Output format for bulk operation results
#[derive(Serialize, schemars::JsonSchema)]
struct BulkResultOutput {
    operation_id: String,
    total: usize,
    completed: usize,
    skipped: usize,
    failed: usize,
    duration_secs: f64,
    details: Vec<BulkResultDetailOutput>,
}

#[derive(Serialize, schemars::JsonSchema)]
struct BulkResultDetailOutput {
    project_id: String,
    project_name: Option<String>,
    status: String,
    message: Option<String>,
    attempts: u32,
}

/// Display bulk operation results
pub(crate) fn display_bulk_result(
    result: &BulkOperationResult,
    output_format: OutputFormat,
) -> Result<()> {
    let details: Vec<BulkResultDetailOutput> = result
        .details
        .iter()
        .map(|d| {
            let (status, message) = match &d.result {
                ItemResult::Success => ("success".to_string(), None),
                ItemResult::Skipped { reason } => ("skipped".to_string(), Some(reason.clone())),
                ItemResult::Failed { error, .. } => ("failed".to_string(), Some(error.clone())),
            };
            BulkResultDetailOutput {
                project_id: d.project_id.clone(),
                project_name: d.project_name.clone(),
                status,
                message,
                attempts: d.attempts,
            }
        })
        .collect();

    let output = BulkResultOutput {
        operation_id: result.operation_id.to_string(),
        total: result.total,
        completed: result.completed,
        skipped: result.skipped,
        failed: result.failed,
        duration_secs: result.duration.as_secs_f64(),
        details,
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Bulk Operation Results:".bold());
            println!("{}", "─".repeat(60));
            println!("{:<15} {}", "Operation:".bold(), output.operation_id.cyan());
            println!("{:<15} {}", "Total:".bold(), output.total);
            println!(
                "{:<15} {}",
                "Completed:".bold(),
                output.completed.to_string().green()
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
            println!("{:<15} {:.2}s", "Duration:".bold(), output.duration_secs);
            println!("{}", "─".repeat(60));

            // Show failed items if any
            if result.failed > 0 {
                println!("\n{}", "Failed Projects:".red().bold());
                for detail in &output.details {
                    if detail.status == "failed" {
                        let name = detail.project_name.as_deref().unwrap_or(&detail.project_id);
                        let msg = detail.message.as_deref().unwrap_or("Unknown error");
                        println!("  {} {} - {}", "✗".red(), name, msg.dimmed());
                    }
                }
            }

            // Summary
            println!();
            if result.failed == 0 && result.total > 0 {
                println!("{} Operation completed successfully!", "✓".green().bold());
            } else if result.failed > 0 {
                println!(
                    "{} Operation completed with {} failure(s)",
                    "⚠".yellow().bold(),
                    result.failed
                );
            }
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

impl OperationCommands {
    pub async fn execute(self, output_format: OutputFormat) -> Result<()> {
        match self {
            OperationCommands::Status { operation_id } => {
                let state_manager = StateManager::new()?;

                let op_id = match operation_id {
                    Some(id) => id,
                    None => {
                        // Get most recent operation
                        let ops = state_manager.list_operations(None).await?;
                        if ops.is_empty() {
                            anyhow::bail!("No operations found");
                        }
                        ops[0].operation_id
                    }
                };

                let state = state_manager.load_operation(op_id).await?;

                let output = OperationStatusOutput {
                    operation_id: state.operation_id.to_string(),
                    operation_type: format!("{:?}", state.operation_type),
                    status: format!("{:?}", state.status),
                    total: state.project_ids.len(),
                    completed: state
                        .results
                        .values()
                        .filter(|r| matches!(r.result, raps_admin::ItemResult::Success))
                        .count(),
                    skipped: state
                        .results
                        .values()
                        .filter(|r| matches!(r.result, raps_admin::ItemResult::Skipped { .. }))
                        .count(),
                    failed: state
                        .results
                        .values()
                        .filter(|r| matches!(r.result, raps_admin::ItemResult::Failed { .. }))
                        .count(),
                    created_at: state.created_at.to_rfc3339(),
                    updated_at: state.updated_at.to_rfc3339(),
                };

                match output_format {
                    OutputFormat::Table => {
                        println!("\n{}", "Operation Status:".bold());
                        println!("{}", "─".repeat(60));
                        println!("{:<15} {}", "Operation:".bold(), output.operation_id.cyan());
                        println!("{:<15} {}", "Type:".bold(), output.operation_type);
                        println!("{:<15} {}", "Status:".bold(), format_status(&output.status));
                        println!(
                            "{:<15} {}/{} ({}%)",
                            "Progress:".bold(),
                            output.completed + output.skipped + output.failed,
                            output.total,
                            if output.total > 0 {
                                ((output.completed + output.skipped + output.failed) * 100)
                                    / output.total
                            } else {
                                100
                            }
                        );
                        println!(
                            "{:<15} {}",
                            "Completed:".bold(),
                            output.completed.to_string().green()
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
                        println!("{:<15} {}", "Created:".bold(), output.created_at);
                        println!("{:<15} {}", "Updated:".bold(), output.updated_at);
                        println!("{}", "─".repeat(60));
                    }
                    _ => {
                        output_format.write(&output)?;
                    }
                }

                Ok(())
            }

            OperationCommands::Resume {
                operation_id,
                concurrency,
            } => {
                let state_manager = StateManager::new()?;

                // Find operation to resume
                let op_id = match operation_id {
                    Some(id) => id,
                    None => {
                        // Get most recent resumable operation
                        match state_manager.get_resumable_operation().await? {
                            Some(id) => id,
                            None => anyhow::bail!("No resumable operation found"),
                        }
                    }
                };

                let state = state_manager.load_operation(op_id).await?;

                // Verify operation can be resumed
                if state.status != OperationStatus::InProgress
                    && state.status != OperationStatus::Pending
                {
                    anyhow::bail!(
                        "Operation cannot be resumed (current status: {:?})",
                        state.status
                    );
                }

                let pending = state_manager.get_pending_projects(&state);
                if pending.is_empty() {
                    if output_format.supports_colors() {
                        println!("{} Operation {} is already complete", "✓".green(), op_id);
                    }
                    return Ok(());
                }

                let concurrency_limit = concurrency.unwrap_or(10).min(50);

                if output_format.supports_colors() {
                    println!(
                        "\n{} Resuming operation: {}",
                        "→".cyan(),
                        op_id.to_string().cyan()
                    );
                    println!("  Type: {:?}", state.operation_type);
                    println!(
                        "  Pending: {}/{} items",
                        pending.len(),
                        state.project_ids.len()
                    );
                    println!("  Concurrency: {}", concurrency_limit);
                    println!();

                    // Note: For full resume support, we'd need the original API clients
                    // For now, just report pending items
                    println!(
                        "{} Resume requires re-running with the original command and credentials.",
                        "⚠".yellow()
                    );
                    println!("  Pending projects:");
                    for (i, project_id) in pending.iter().take(10).enumerate() {
                        println!("    {}. {}", i + 1, project_id.dimmed());
                    }
                    if pending.len() > 10 {
                        println!("    ... and {} more", pending.len() - 10);
                    }
                }

                Ok(())
            }

            OperationCommands::Cancel {
                operation_id,
                yes: _,
            } => {
                let state_manager = StateManager::new()?;

                // Find operation to cancel
                let op_id = match operation_id {
                    Some(id) => id,
                    None => {
                        // Get most recent in-progress operation
                        match state_manager.get_resumable_operation().await? {
                            Some(id) => id,
                            None => anyhow::bail!("No active operation found to cancel"),
                        }
                    }
                };

                let state = state_manager.load_operation(op_id).await?;

                if output_format.supports_colors() {
                    println!(
                        "\n{} Cancelling operation: {}",
                        "→".cyan(),
                        op_id.to_string().cyan()
                    );
                    println!("  Type: {:?}", state.operation_type);
                    println!("  Current status: {:?}", state.status);
                }

                // Cancel the operation
                state_manager.cancel_operation(op_id).await?;

                if output_format.supports_colors() {
                    let processed = state.results.len();
                    let total = state.project_ids.len();
                    println!("\n{} Operation cancelled", "✓".green());
                    println!(
                        "  Processed: {}/{} items before cancellation",
                        processed, total
                    );
                }

                Ok(())
            }

            OperationCommands::List { status, limit } => {
                let state_manager = StateManager::new()?;

                let status_filter = status
                    .as_ref()
                    .and_then(|s| match s.to_lowercase().as_str() {
                        "pending" => Some(OperationStatus::Pending),
                        "in_progress" | "in-progress" => Some(OperationStatus::InProgress),
                        "completed" => Some(OperationStatus::Completed),
                        "failed" => Some(OperationStatus::Failed),
                        "cancelled" => Some(OperationStatus::Cancelled),
                        _ => None,
                    });

                let operations = state_manager.list_operations(status_filter).await?;
                let operations: Vec<_> = operations.into_iter().take(limit).collect();

                if operations.is_empty() {
                    match output_format {
                        OutputFormat::Table => println!("{}", "No operations found.".yellow()),
                        _ => output_format.write(&Vec::<OperationListOutput>::new())?,
                    }
                    return Ok(());
                }

                let outputs: Vec<OperationListOutput> = operations
                    .iter()
                    .map(|op| OperationListOutput {
                        operation_id: op.operation_id.to_string(),
                        operation_type: format!("{:?}", op.operation_type),
                        status: format!("{:?}", op.status),
                        progress: format!("{}/{}", op.completed + op.skipped + op.failed, op.total),
                        updated_at: op.updated_at.to_rfc3339(),
                    })
                    .collect();

                match output_format {
                    OutputFormat::Table => {
                        println!("\n{}", "Operations:".bold());
                        println!("{}", "─".repeat(100));
                        println!(
                            "{:<38} {:<15} {:<12} {:<12} {}",
                            "ID".bold(),
                            "Type".bold(),
                            "Status".bold(),
                            "Progress".bold(),
                            "Updated".bold()
                        );
                        println!("{}", "─".repeat(100));

                        for op in &outputs {
                            println!(
                                "{:<38} {:<15} {:<12} {:<12} {}",
                                op.operation_id.cyan(),
                                op.operation_type,
                                format_status(&op.status),
                                op.progress,
                                op.updated_at.dimmed()
                            );
                        }

                        println!("{}", "─".repeat(100));
                        println!("{} {} operation(s) found", "→".cyan(), outputs.len());
                    }
                    _ => {
                        output_format.write(&outputs)?;
                    }
                }

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use raps_admin::{BulkOperationResult, ItemDetail, ItemResult};
    use std::time::Duration;
    use uuid::Uuid;

    // ── format_status ──────────────────────────────────────────────────────

    #[test]
    fn format_status_completed_contains_text() {
        assert!(format_status("completed").contains("completed"));
    }

    #[test]
    fn format_status_failed_contains_text() {
        assert!(format_status("failed").contains("failed"));
    }

    #[test]
    fn format_status_inprogress_contains_text() {
        assert!(format_status("inprogress").contains("inprogress"));
    }

    #[test]
    fn format_status_in_progress_variant_contains_text() {
        assert!(format_status("in_progress").contains("in_progress"));
    }

    #[test]
    fn format_status_cancelled_contains_text() {
        assert!(format_status("cancelled").contains("cancelled"));
    }

    #[test]
    fn format_status_unknown_passes_through() {
        assert!(format_status("pending").contains("pending"));
    }

    // ── display_bulk_result ────────────────────────────────────────────────

    fn make_result(
        completed: usize,
        skipped: usize,
        failed: usize,
        details: Vec<ItemDetail>,
    ) -> BulkOperationResult {
        BulkOperationResult {
            operation_id: Uuid::new_v4(),
            total: completed + skipped + failed,
            completed,
            skipped,
            failed,
            duration: Duration::from_millis(123),
            details,
        }
    }

    #[test]
    fn display_bulk_result_json_all_success_returns_ok() {
        let result = make_result(
            2,
            0,
            0,
            vec![
                ItemDetail {
                    project_id: "proj-1".into(),
                    project_name: Some("Alpha".into()),
                    result: ItemResult::Success,
                    attempts: 1,
                },
                ItemDetail {
                    project_id: "proj-2".into(),
                    project_name: None,
                    result: ItemResult::Success,
                    attempts: 1,
                },
            ],
        );
        assert!(display_bulk_result(&result, OutputFormat::Json).is_ok());
    }

    #[test]
    fn display_bulk_result_json_with_skipped_and_failed_returns_ok() {
        let result = make_result(
            1,
            1,
            1,
            vec![
                ItemDetail {
                    project_id: "p1".into(),
                    project_name: None,
                    result: ItemResult::Success,
                    attempts: 1,
                },
                ItemDetail {
                    project_id: "p2".into(),
                    project_name: Some("Beta".into()),
                    result: ItemResult::Skipped {
                        reason: "already member".into(),
                    },
                    attempts: 1,
                },
                ItemDetail {
                    project_id: "p3".into(),
                    project_name: Some("Gamma".into()),
                    result: ItemResult::Failed {
                        error: "403 Forbidden".into(),
                        retryable: false,
                    },
                    attempts: 3,
                },
            ],
        );
        assert!(display_bulk_result(&result, OutputFormat::Json).is_ok());
    }

    #[test]
    fn display_bulk_result_table_no_failures_returns_ok() {
        let result = make_result(
            3,
            0,
            0,
            vec![ItemDetail {
                project_id: "p1".into(),
                project_name: Some("One".into()),
                result: ItemResult::Success,
                attempts: 1,
            }],
        );
        assert!(display_bulk_result(&result, OutputFormat::Table).is_ok());
    }

    #[test]
    fn display_bulk_result_table_with_failures_returns_ok() {
        let result = make_result(
            1,
            0,
            1,
            vec![
                ItemDetail {
                    project_id: "p1".into(),
                    project_name: Some("Good".into()),
                    result: ItemResult::Success,
                    attempts: 1,
                },
                ItemDetail {
                    project_id: "p2".into(),
                    project_name: None,
                    result: ItemResult::Failed {
                        error: "timeout".into(),
                        retryable: true,
                    },
                    attempts: 3,
                },
            ],
        );
        assert!(display_bulk_result(&result, OutputFormat::Table).is_ok());
    }

    #[test]
    fn display_bulk_result_table_empty_total_no_panic() {
        let result = make_result(0, 0, 0, vec![]);
        assert!(display_bulk_result(&result, OutputFormat::Table).is_ok());
    }
}

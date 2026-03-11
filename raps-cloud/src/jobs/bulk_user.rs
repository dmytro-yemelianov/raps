// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Bulk user add/remove job executors.
//!
//! These functions bridge the raps-cloud job runner with the battle-tested
//! `raps_admin::operations` bulk add/remove implementations, which handle
//! upsert logic, state persistence, and retry semantics.

use std::sync::Arc;

use anyhow::{Context, Result};

use raps_admin::filter::ProjectFilter;
use raps_admin::{BulkConfig, BulkOperationResult};

use super::aps;
use crate::AppState;

/// Expected input JSON for bulk_user_add:
/// {
///   "account_id": "uuid-string",
///   "email": "user@example.com",
///   "role_id": "role-uuid",          // optional
///   "products": [...],               // optional ACC product access list
///   "project_filter": "name:*test*", // optional
///   "concurrency": 10,               // optional
///   "dry_run": false                  // optional
/// }
pub async fn execute_add(
    state: &AppState,
    job: &crate::db::jobs::Job,
) -> Result<serde_json::Value> {
    let input = &job.input;
    let account_id = input["account_id"]
        .as_str()
        .context("Missing account_id")?;
    let email = input["email"].as_str().context("Missing email")?;
    let role_id = input["role_id"].as_str();
    let concurrency = input["concurrency"].as_u64().unwrap_or(10) as usize;
    let dry_run = input["dry_run"].as_bool().unwrap_or(false);

    // Parse optional product access list
    let products: Vec<raps_acc::types::ProductAccess> = input["products"]
        .as_array()
        .and_then(|arr| serde_json::from_value(serde_json::Value::Array(arr.clone())).ok())
        .unwrap_or_default();

    // Parse optional project filter
    let project_filter = match input["project_filter"].as_str() {
        Some(expr) => ProjectFilter::from_expression(expr)
            .map_err(|e| anyhow::anyhow!("Invalid project filter: {e}"))?,
        None => ProjectFilter::new(),
    };

    let credential_id = job.credential_id.context("Missing credential_id")?;
    let clients = aps::build_clients(state, credential_id).await?;

    let config = BulkConfig {
        concurrency,
        dry_run,
        ..Default::default()
    };

    let result = raps_admin::operations::bulk_add_user(
        &clients.admin,
        Arc::new(clients.users),
        account_id,
        email,
        role_id,
        products,
        &project_filter,
        config,
        |_progress| {},
    )
    .await?;

    Ok(bulk_result_to_json(&result))
}

/// Expected input JSON for bulk_user_remove:
/// {
///   "account_id": "uuid-string",
///   "email": "user@example.com",
///   "project_filter": "name:*test*", // optional
///   "concurrency": 10,               // optional
///   "dry_run": false                  // optional
/// }
pub async fn execute_remove(
    state: &AppState,
    job: &crate::db::jobs::Job,
) -> Result<serde_json::Value> {
    let input = &job.input;
    let account_id = input["account_id"]
        .as_str()
        .context("Missing account_id")?;
    let email = input["email"].as_str().context("Missing email")?;
    let concurrency = input["concurrency"].as_u64().unwrap_or(10) as usize;
    let dry_run = input["dry_run"].as_bool().unwrap_or(false);

    // Parse optional project filter
    let project_filter = match input["project_filter"].as_str() {
        Some(expr) => ProjectFilter::from_expression(expr)
            .map_err(|e| anyhow::anyhow!("Invalid project filter: {e}"))?,
        None => ProjectFilter::new(),
    };

    let credential_id = job.credential_id.context("Missing credential_id")?;
    let clients = aps::build_clients(state, credential_id).await?;

    let config = BulkConfig {
        concurrency,
        dry_run,
        ..Default::default()
    };

    let result = raps_admin::operations::bulk_remove_user(
        &clients.admin,
        Arc::new(clients.users),
        account_id,
        email,
        &project_filter,
        config,
        |_progress| {},
    )
    .await?;

    Ok(bulk_result_to_json(&result))
}

/// Convert a `BulkOperationResult` into a JSON summary suitable for job output.
fn bulk_result_to_json(result: &BulkOperationResult) -> serde_json::Value {
    serde_json::json!({
        "operation_id": result.operation_id.to_string(),
        "total": result.total,
        "completed": result.completed,
        "failed": result.failed,
        "skipped": result.skipped,
        "duration_ms": result.duration.as_millis() as u64,
    })
}

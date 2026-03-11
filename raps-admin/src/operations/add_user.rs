// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Bulk add user operation

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use uuid::Uuid;

use raps_acc::admin::AccountAdminClient;
use raps_acc::types::ProductAccess;
use raps_acc::users::{AddProjectUserRequest, ProjectUsersClient, UpdateProjectUserRequest};

use crate::bulk::executor::{
    BulkConfig, BulkExecutor, BulkOperationResult, ItemResult, ProcessItem, ProgressUpdate,
};
use crate::bulk::state::{StateManager, StateUpdate};
use crate::filter::ProjectFilter;
use crate::types::OperationType;

/// Parameters for bulk add user operation
#[derive(Debug, Clone)]
pub struct BulkAddUserParams {
    /// Account ID
    pub account_id: String,
    /// User email to add
    pub user_email: String,
    /// Role ID to assign (optional)
    pub role_id: Option<String>,
    /// Product access configurations
    pub products: Vec<ProductAccess>,
}

/// Add a user to multiple projects in bulk
///
/// # Arguments
/// * `admin_client` - Client for account admin API (user/project lookup)
/// * `users_client` - Client for project users API (add user)
/// * `account_id` - The account ID
/// * `user_email` - Email of the user to add
/// * `role_id` - Optional BIM 360 role UUID to assign
/// * `products` - ACC product access list (used instead of role_id for ACC hubs)
/// * `project_filter` - Filter for selecting target projects
/// * `config` - Bulk execution configuration
/// * `on_progress` - Progress callback
///
/// # Returns
/// Result containing the bulk operation outcome
#[allow(clippy::too_many_arguments)]
pub async fn bulk_add_user<P>(
    admin_client: &AccountAdminClient,
    users_client: Arc<ProjectUsersClient>,
    account_id: &str,
    user_email: &str,
    role_id: Option<&str>,
    products: Vec<ProductAccess>,
    project_filter: &ProjectFilter,
    config: BulkConfig,
    on_progress: P,
) -> Result<BulkOperationResult>
where
    P: Fn(ProgressUpdate) + Send + Sync + 'static,
{
    // Step 1: Get list of projects matching the filter
    let all_projects = admin_client.list_all_projects(account_id).await?;
    let filtered_projects = project_filter.apply(all_projects);

    if filtered_projects.is_empty() {
        return Ok(BulkOperationResult {
            operation_id: Uuid::new_v4(),
            total: 0,
            completed: 0,
            failed: 0,
            skipped: 0,
            duration: std::time::Duration::from_secs(0),
            details: vec![],
        });
    }

    // Step 2: Create operation state for resumability
    let state_manager = StateManager::new()?;
    let project_ids: Vec<String> = filtered_projects.iter().map(|p| p.id.clone()).collect();

    let params = serde_json::json!({
        "account_id": account_id,
        "user_email": user_email,
        "role_id": role_id,
        "products": products,
    });

    let operation_id = state_manager
        .create_operation(OperationType::AddUser, params, project_ids)
        .await?;

    // Mark operation as in progress
    state_manager
        .update_state(
            operation_id,
            StateUpdate::StatusChanged {
                status: crate::types::OperationStatus::InProgress,
            },
        )
        .await?;

    // Step 3: Build a map of project_id → product keys so the BIM 360 import
    // path knows which services each project actually has (avoids nil:NilClass
    // crash when sending services the project doesn't support).
    let product_keys_map: HashMap<String, Vec<String>> = filtered_projects
        .iter()
        .map(|p| {
            let keys = p
                .products
                .as_ref()
                .map(|prods| {
                    prods
                        .iter()
                        .filter_map(|v| v.get("key").and_then(|k| k.as_str()).map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            (p.id.clone(), keys)
        })
        .collect();

    // Step 4: Prepare items for processing
    let items: Vec<ProcessItem> = filtered_projects
        .into_iter()
        .map(|p| ProcessItem {
            project_id: p.id,
            project_name: Some(p.name),
        })
        .collect();

    // Step 5: Create the processor closure
    let email_clone = user_email.to_string();
    let role_id_clone = role_id.map(|s| s.to_string());
    let products_clone = products.clone();
    let users_client_clone = Arc::clone(&users_client);

    let processor = move |project_id: String| {
        let email = email_clone.clone();
        let role_id = role_id_clone.clone();
        let products = products_clone.clone();
        let users_client = Arc::clone(&users_client_clone);
        let project_keys = product_keys_map.get(&project_id).cloned();

        async move {
            add_user_to_project(&users_client, &project_id, &email, role_id.as_deref(), products, project_keys).await
        }
    };

    // Step 6: Execute bulk operation
    let executor = BulkExecutor::new(config);
    let result = executor
        .execute(operation_id, items, processor, on_progress)
        .await;

    // Step 7: Update final operation status
    let final_status = if result.failed > 0 {
        crate::types::OperationStatus::Failed
    } else {
        crate::types::OperationStatus::Completed
    };

    state_manager
        .complete_operation(operation_id, final_status)
        .await?;

    Ok(result)
}

/// Add a single user to a single project with duplicate detection
async fn add_user_to_project(
    users_client: &ProjectUsersClient,
    project_id: &str,
    email: &str,
    role_id: Option<&str>,
    products: Vec<ProductAccess>,
    project_product_keys: Option<Vec<String>>,
) -> ItemResult {
    // Add the user to the project by email; ACC sends an invitation if the
    // user is not yet an account member.
    // Note: no pre-check by email — user_exists requires a UUID, not an email.
    // Duplicate detection is handled by treating HTTP 409 as Skipped below.
    let products_for_upsert = products.clone();
    let request = AddProjectUserRequest {
        email: email.to_string(),
        role_ids: role_id.map(|s| vec![s.to_string()]).unwrap_or_default(),
        products,
        suppress_administrative_emails: true,
        project_product_keys,
    };

    match users_client.add_user(project_id, request).await {
        Ok(_) => ItemResult::Success,
        Err(e) => {
            let error_str = e.to_string();
            // 409 = user already in project — upsert: update role if it differs
            if is_already_member_error(&error_str) {
                return upsert_existing_member(
                    users_client,
                    project_id,
                    email,
                    role_id,
                    products_for_upsert,
                )
                .await;
            }
            let retryable = is_retryable_error(&error_str);
            ItemResult::Failed {
                error: error_str,
                retryable,
            }
        }
    }
}

/// Upsert: user is already in the project — update their role if it differs.
///
/// Finds the user by email, compares their current role/products with the
/// requested ones, and PATCHes if a change is needed. Returns Skipped if
/// the user already has the correct role, or Success if updated.
async fn upsert_existing_member(
    users_client: &ProjectUsersClient,
    project_id: &str,
    email: &str,
    role_id: Option<&str>,
    products: Vec<ProductAccess>,
) -> ItemResult {
    // No role/products requested — nothing to update
    if role_id.is_none() && products.is_empty() {
        return ItemResult::Skipped {
            reason: "already_exists".to_string(),
        };
    }

    // Find the user in the project by email
    let existing = match users_client.find_project_user_by_email(project_id, email).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            // 409 fired but user not found by email lookup — the user may have
            // no email in their project profile (e.g. invited via user_id).
            // Treat as already in the project since we can't determine their role.
            return ItemResult::Skipped {
                reason: "already_exists_no_email_match".to_string(),
            };
        }
        Err(e) => {
            return ItemResult::Failed {
                error: format!("Failed to look up existing user for role update: {e}"),
                retryable: true,
            };
        }
    };

    // Check if role already matches
    let role_matches = if let Some(rid) = role_id {
        existing.role_ids.contains(&rid.to_string())
    } else if !products.is_empty() {
        // Compare product access keys — if current products cover the requested ones, skip
        let current_keys: std::collections::HashSet<&str> = existing
            .products
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|p| p.key.as_str())
            .collect();
        products.iter().all(|p| {
            existing
                .products
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .any(|cp| cp.key == p.key && cp.access == p.access)
        }) && products.iter().all(|p| current_keys.contains(p.key.as_str()))
    } else {
        true
    };

    if role_matches {
        return ItemResult::Skipped {
            reason: "already_exists_same_role".to_string(),
        };
    }

    // Role differs — update
    let update = UpdateProjectUserRequest {
        role_ids: role_id.map(|s| vec![s.to_string()]).unwrap_or_default(),
        products: if products.is_empty() { None } else { Some(products) },
    };

    match users_client.update_user(project_id, &existing.id, update).await {
        Ok(_) => ItemResult::Success,
        Err(e) => {
            let error_str = e.to_string();
            if is_insight_restriction_error(&error_str) {
                return ItemResult::Skipped {
                    reason: "insight_role_locked".to_string(),
                };
            }
            ItemResult::Failed {
                error: format!("Failed to update user role: {e}"),
                retryable: is_retryable_error(&error_str),
            }
        }
    }
}

/// Check if the error indicates the user is already a member (HTTP 409)
fn is_already_member_error(error: &str) -> bool {
    let lower = error.to_lowercase();
    lower.contains("409")
        || lower.contains("already belongs")
        || lower.contains("already exists")
        || lower.contains("conflict")
}

/// Check if the error is an Insight product restriction (role cannot be changed via this API)
fn is_insight_restriction_error(error: &str) -> bool {
    let lower = error.to_lowercase();
    lower.contains("cannot remove user access from insight")
        || lower.contains("insight")
            && (lower.contains("cannot") || lower.contains("not allowed") || lower.contains("restricted"))
}

/// Check if an error is retryable
fn is_retryable_error(error: &str) -> bool {
    let lower = error.to_lowercase();
    lower.contains("429")
        || lower.contains("rate limit")
        || lower.contains("too many requests")
        || lower.contains("503")
        || lower.contains("service unavailable")
        || lower.contains("502")
        || lower.contains("bad gateway")
        || lower.contains("timeout")
        || lower.contains("connection")
}

/// Resume an interrupted bulk add user operation
pub async fn resume_bulk_add_user<P>(
    users_client: Arc<ProjectUsersClient>,
    operation_id: Uuid,
    config: BulkConfig,
    on_progress: P,
) -> Result<BulkOperationResult>
where
    P: Fn(ProgressUpdate) + Send + Sync + 'static,
{
    let state_manager = StateManager::new()?;
    let state = state_manager.load_operation(operation_id).await?;

    // Get the user email from saved parameters
    let user_email = state.parameters["user_email"]
        .as_str()
        .context("Missing user_email in operation parameters")?
        .to_string();

    let role_id = state.parameters["role_id"].as_str().map(|s| s.to_string());
    let products: Vec<ProductAccess> = state.parameters["products"]
        .as_array()
        .and_then(|arr| serde_json::from_value(serde_json::Value::Array(arr.clone())).ok())
        .unwrap_or_default();

    // Get pending projects (not yet processed)
    let pending_project_ids = state_manager.get_pending_projects(&state);

    if pending_project_ids.is_empty() {
        // All projects already processed
        return Ok(BulkOperationResult {
            operation_id,
            total: state.project_ids.len(),
            completed: state
                .results
                .values()
                .filter(|r| matches!(r.result, ItemResult::Success))
                .count(),
            failed: state
                .results
                .values()
                .filter(|r| matches!(r.result, ItemResult::Failed { .. }))
                .count(),
            skipped: state
                .results
                .values()
                .filter(|r| matches!(r.result, ItemResult::Skipped { .. }))
                .count(),
            duration: std::time::Duration::from_secs(0),
            details: vec![],
        });
    }

    // Mark operation as in progress again
    state_manager
        .update_state(
            operation_id,
            StateUpdate::StatusChanged {
                status: crate::types::OperationStatus::InProgress,
            },
        )
        .await?;

    // Prepare items for processing
    let items: Vec<ProcessItem> = pending_project_ids
        .into_iter()
        .map(|id| ProcessItem {
            project_id: id,
            project_name: None,
        })
        .collect();

    // Create the processor closure
    let users_client_clone = Arc::clone(&users_client);

    let processor = move |project_id: String| {
        let email = user_email.clone();
        let role_id = role_id.clone();
        let products = products.clone();
        let users_client = Arc::clone(&users_client_clone);

        async move {
            // Resume path: no cached product keys — the BIM 360 path will fetch if needed
            add_user_to_project(&users_client, &project_id, &email, role_id.as_deref(), products, None).await
        }
    };

    // Execute bulk operation
    let executor = BulkExecutor::new(config);
    let result = executor
        .execute(operation_id, items, processor, on_progress)
        .await;

    // Update final operation status
    let final_status = if result.failed > 0 {
        crate::types::OperationStatus::Failed
    } else {
        crate::types::OperationStatus::Completed
    };

    state_manager
        .complete_operation(operation_id, final_status)
        .await?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable_error() {
        assert!(is_retryable_error("429 Too Many Requests"));
        assert!(is_retryable_error("Rate limit exceeded"));
        assert!(is_retryable_error("503 Service Unavailable"));
        assert!(is_retryable_error("Connection timeout"));
        assert!(!is_retryable_error("404 Not Found"));
        assert!(!is_retryable_error("400 Bad Request"));
    }

    #[test]
    fn test_is_insight_restriction_error() {
        assert!(is_insight_restriction_error("Cannot remove user access from Insight"));
        assert!(is_insight_restriction_error(
            r#"Failed to update project user (400 Bad Request): {"detail":"Cannot remove user access from Insight"}"#
        ));
        assert!(!is_insight_restriction_error("400 Bad Request"));
        assert!(!is_insight_restriction_error("403 Forbidden"));
    }
}

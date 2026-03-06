// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Bulk add user operation

use std::sync::Arc;

use anyhow::{Context, Result};
use uuid::Uuid;

use raps_acc::admin::AccountAdminClient;
use raps_acc::types::ProductAccess;
use raps_acc::users::{AddProjectUserRequest, ProjectUsersClient};

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

    // Step 3: Prepare items for processing
    let items: Vec<ProcessItem> = filtered_projects
        .into_iter()
        .map(|p| ProcessItem {
            project_id: p.id,
            project_name: Some(p.name),
        })
        .collect();

    // Step 4: Create the processor closure
    let email_clone = user_email.to_string();
    let role_id_clone = role_id.map(|s| s.to_string());
    let products_clone = products.clone();
    let users_client_clone = Arc::clone(&users_client);

    let processor = move |project_id: String| {
        let email = email_clone.clone();
        let role_id = role_id_clone.clone();
        let products = products_clone.clone();
        let users_client = Arc::clone(&users_client_clone);

        async move {
            add_user_to_project(&users_client, &project_id, &email, role_id.as_deref(), products).await
        }
    };

    // Step 5: Execute bulk operation
    let executor = BulkExecutor::new(config);
    let result = executor
        .execute(operation_id, items, processor, on_progress)
        .await;

    // Step 6: Update final operation status
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
) -> ItemResult {
    // Check if user already exists in the project
    match users_client.user_exists(project_id, email).await {
        Ok(true) => {
            return ItemResult::Skipped {
                reason: "already_exists".to_string(),
            };
        }
        Ok(false) => {
            // User doesn't exist, proceed to add
        }
        Err(e) => {
            // Error checking existence - treat as retryable
            return ItemResult::Failed {
                error: format!("Failed to check user existence: {}", e),
                retryable: true,
            };
        }
    }

    // Add the user to the project by email; ACC sends an invitation if the
    // user is not yet an account member.
    let request = AddProjectUserRequest {
        email: email.to_string(),
        role_id: role_id.map(|s| s.to_string()),
        products,
    };

    match users_client.add_user(project_id, request).await {
        Ok(_) => ItemResult::Success,
        Err(e) => {
            let error_str = e.to_string();
            // 409 = user already belongs to the project — treat as skipped, not failed
            if is_already_member_error(&error_str) {
                return ItemResult::Skipped {
                    reason: "already_exists".to_string(),
                };
            }
            let retryable = is_retryable_error(&error_str);
            ItemResult::Failed {
                error: error_str,
                retryable,
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
            add_user_to_project(&users_client, &project_id, &email, role_id.as_deref(), products).await
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
}

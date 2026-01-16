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
use crate::error::AdminError;
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
/// * `role_id` - Optional role ID to assign
/// * `project_filter` - Filter for selecting target projects
/// * `config` - Bulk execution configuration
/// * `on_progress` - Progress callback
///
/// # Returns
/// Result containing the bulk operation outcome
pub async fn bulk_add_user<P>(
    admin_client: &AccountAdminClient,
    users_client: Arc<ProjectUsersClient>,
    account_id: &str,
    user_email: &str,
    role_id: Option<&str>,
    project_filter: &ProjectFilter,
    config: BulkConfig,
    on_progress: P,
) -> Result<BulkOperationResult>
where
    P: Fn(ProgressUpdate) + Send + Sync + 'static,
{
    // Step 1: Look up user by email to get their user ID
    let user = admin_client
        .find_user_by_email(account_id, user_email)
        .await?
        .ok_or_else(|| AdminError::UserNotFound {
            email: user_email.to_string(),
        })?;

    let user_id = user.id.clone();

    // Step 2: Get list of projects matching the filter
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

    // Step 3: Create operation state for resumability
    let state_manager = StateManager::new()?;
    let project_ids: Vec<String> = filtered_projects.iter().map(|p| p.id.clone()).collect();

    let params = serde_json::json!({
        "account_id": account_id,
        "user_email": user_email,
        "user_id": user_id,
        "role_id": role_id,
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

    // Step 4: Prepare items for processing
    let items: Vec<ProcessItem> = filtered_projects
        .into_iter()
        .map(|p| ProcessItem {
            project_id: p.id,
            project_name: Some(p.name),
        })
        .collect();

    // Step 5: Create the processor closure
    let user_id_clone = user_id.clone();
    let role_id_clone = role_id.map(|s| s.to_string());
    let users_client_clone = Arc::clone(&users_client);

    let processor = move |project_id: String| {
        let user_id = user_id_clone.clone();
        let role_id = role_id_clone.clone();
        let users_client = Arc::clone(&users_client_clone);

        async move {
            add_user_to_project(&users_client, &project_id, &user_id, role_id.as_deref()).await
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
    user_id: &str,
    role_id: Option<&str>,
) -> ItemResult {
    // Check if user already exists in the project
    match users_client.user_exists(project_id, user_id).await {
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

    // Add the user to the project
    let request = AddProjectUserRequest {
        user_id: user_id.to_string(),
        role_id: role_id.map(|s| s.to_string()),
        products: vec![], // Default product access
    };

    match users_client.add_user(project_id, request).await {
        Ok(_) => ItemResult::Success,
        Err(e) => {
            let error_str = e.to_string();
            // Check if it's a rate limit or temporary error
            let retryable = is_retryable_error(&error_str);
            ItemResult::Failed {
                error: error_str,
                retryable,
            }
        }
    }
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

    // Get the user ID from saved parameters
    let user_id = state.parameters["user_id"]
        .as_str()
        .context("Missing user_id in operation parameters")?
        .to_string();

    let role_id = state.parameters["role_id"].as_str().map(|s| s.to_string());

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
        let user_id = user_id.clone();
        let role_id = role_id.clone();
        let users_client = Arc::clone(&users_client_clone);

        async move {
            add_user_to_project(&users_client, &project_id, &user_id, role_id.as_deref()).await
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

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Bulk folder rights operation

use std::sync::Arc;

use anyhow::{Context, Result};
use uuid::Uuid;

use raps_acc::admin::AccountAdminClient;
use raps_acc::permissions::{
    BatchUpdatePermissionsRequest, FolderPermissionsClient, UpdatePermissionRequest,
};

use crate::bulk::executor::{
    BulkConfig, BulkExecutor, BulkOperationResult, ItemResult, ProcessItem, ProgressUpdate,
};
use crate::bulk::state::{StateManager, StateUpdate};
use crate::error::AdminError;
use crate::filter::ProjectFilter;
use crate::types::{FolderType, OperationType, PermissionLevel};

/// Parameters for bulk update folder rights operation
#[derive(Debug, Clone)]
pub struct BulkUpdateFolderRightsParams {
    /// Account ID
    pub account_id: String,
    /// User email to update
    pub user_email: String,
    /// Permission level to set
    pub permission_level: PermissionLevel,
    /// Folder type to update
    pub folder_type: FolderType,
}

/// Update a user's folder permissions across multiple projects in bulk
///
/// # Arguments
/// * `admin_client` - Client for account admin API (user/project lookup)
/// * `permissions_client` - Client for folder permissions API
/// * `account_id` - The account ID
/// * `user_email` - Email of the user to update
/// * `permission_level` - The permission level to assign
/// * `folder_type` - The folder type to update (ProjectFiles, Plans, or Custom)
/// * `project_filter` - Filter for selecting target projects
/// * `config` - Bulk execution configuration
/// * `on_progress` - Progress callback
///
/// # Returns
/// Result containing the bulk operation outcome
pub async fn bulk_update_folder_rights<P>(
    admin_client: &AccountAdminClient,
    permissions_client: Arc<FolderPermissionsClient>,
    account_id: &str,
    user_email: &str,
    permission_level: PermissionLevel,
    folder_type: FolderType,
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

    let folder_type_str = match &folder_type {
        FolderType::ProjectFiles => "project_files".to_string(),
        FolderType::Plans => "plans".to_string(),
        FolderType::Custom(path) => format!("custom:{}", path),
    };

    let params = serde_json::json!({
        "account_id": account_id,
        "user_email": user_email,
        "user_id": user_id,
        "permission_level": format!("{:?}", permission_level),
        "folder_type": folder_type_str,
    });

    let operation_id = state_manager
        .create_operation(OperationType::UpdateFolderRights, params, project_ids)
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
    let folder_type_clone = folder_type.clone();
    let permissions_client_clone = Arc::clone(&permissions_client);
    let actions: Vec<String> = permission_level
        .to_actions()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let processor = move |project_id: String| {
        let user_id = user_id_clone.clone();
        let folder_type = folder_type_clone.clone();
        let permissions_client = Arc::clone(&permissions_client_clone);
        let actions = actions.clone();

        async move {
            update_folder_permissions(
                &permissions_client,
                &project_id,
                &user_id,
                &folder_type,
                actions,
            )
            .await
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

/// Update folder permissions for a single user in a single project
async fn update_folder_permissions(
    permissions_client: &FolderPermissionsClient,
    project_id: &str,
    user_id: &str,
    folder_type: &FolderType,
    actions: Vec<String>,
) -> ItemResult {
    // Get the folder ID based on folder type
    let folder_id = match folder_type {
        FolderType::ProjectFiles => {
            match permissions_client
                .get_project_files_folder_id(project_id)
                .await
            {
                Ok(id) => id,
                Err(e) => {
                    let error_str = e.to_string();
                    if error_str.contains("not found") {
                        return ItemResult::Skipped {
                            reason: "project_files_folder_not_found".to_string(),
                        };
                    }
                    return ItemResult::Failed {
                        error: format!("Failed to get Project Files folder: {}", error_str),
                        retryable: is_retryable_error(&error_str),
                    };
                }
            }
        }
        FolderType::Plans => match permissions_client.get_plans_folder_id(project_id).await {
            Ok(id) => id,
            Err(e) => {
                let error_str = e.to_string();
                if error_str.contains("not found") {
                    return ItemResult::Skipped {
                        reason: "plans_folder_not_found".to_string(),
                    };
                }
                return ItemResult::Failed {
                    error: format!("Failed to get Plans folder: {}", error_str),
                    retryable: is_retryable_error(&error_str),
                };
            }
        },
        FolderType::Custom(folder_id) => folder_id.clone(),
    };

    // Update permissions
    let request = BatchUpdatePermissionsRequest {
        permissions: vec![UpdatePermissionRequest {
            subject_id: user_id.to_string(),
            subject_type: "USER".to_string(),
            actions,
        }],
    };

    match permissions_client
        .update_permissions(project_id, &folder_id, request)
        .await
    {
        Ok(()) => ItemResult::Success,
        Err(e) => {
            let error_str = e.to_string();
            ItemResult::Failed {
                error: error_str.clone(),
                retryable: is_retryable_error(&error_str),
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

/// Resume an interrupted bulk update folder rights operation
pub async fn resume_bulk_update_folder_rights<P>(
    permissions_client: Arc<FolderPermissionsClient>,
    operation_id: Uuid,
    config: BulkConfig,
    on_progress: P,
) -> Result<BulkOperationResult>
where
    P: Fn(ProgressUpdate) + Send + Sync + 'static,
{
    let state_manager = StateManager::new()?;
    let state = state_manager.load_operation(operation_id).await?;

    // Get parameters from saved state
    let user_id = state.parameters["user_id"]
        .as_str()
        .context("Missing user_id in operation parameters")?
        .to_string();

    let permission_level_str = state.parameters["permission_level"]
        .as_str()
        .context("Missing permission_level in operation parameters")?;

    let folder_type_str = state.parameters["folder_type"]
        .as_str()
        .context("Missing folder_type in operation parameters")?;

    // Parse folder type
    let folder_type = if folder_type_str == "project_files" {
        FolderType::ProjectFiles
    } else if folder_type_str == "plans" {
        FolderType::Plans
    } else if let Some(path) = folder_type_str.strip_prefix("custom:") {
        FolderType::Custom(path.to_string())
    } else {
        anyhow::bail!("Unknown folder type: {}", folder_type_str);
    };

    // Parse permission level
    let permission_level = match permission_level_str {
        "ViewOnly" => PermissionLevel::ViewOnly,
        "ViewDownload" => PermissionLevel::ViewDownload,
        "UploadOnly" => PermissionLevel::UploadOnly,
        "ViewDownloadUpload" => PermissionLevel::ViewDownloadUpload,
        "ViewDownloadUploadEdit" => PermissionLevel::ViewDownloadUploadEdit,
        "FolderControl" => PermissionLevel::FolderControl,
        _ => anyhow::bail!("Unknown permission level: {}", permission_level_str),
    };

    let actions: Vec<String> = permission_level
        .to_actions()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    // Get pending projects
    let pending_project_ids = state_manager.get_pending_projects(&state);

    if pending_project_ids.is_empty() {
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
    let permissions_client_clone = Arc::clone(&permissions_client);

    let processor = move |project_id: String| {
        let user_id = user_id.clone();
        let folder_type = folder_type.clone();
        let permissions_client = Arc::clone(&permissions_client_clone);
        let actions = actions.clone();

        async move {
            update_folder_permissions(
                &permissions_client,
                &project_id,
                &user_id,
                &folder_type,
                actions,
            )
            .await
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
        assert!(!is_retryable_error("404 Not Found"));
        assert!(!is_retryable_error("403 Forbidden"));
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Integration tests for operation resume and state management

use raps_admin::bulk::executor::ItemResult;
use raps_admin::bulk::state::{StateManager, StateUpdate};
use raps_admin::types::{OperationStatus, OperationType};
use tempfile::TempDir;

async fn create_test_manager() -> (StateManager, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let manager = StateManager::with_dir(temp_dir.path().to_path_buf()).unwrap();
    (manager, temp_dir)
}

/// Test creating and loading an operation
#[tokio::test]
async fn test_create_and_load_operation() {
    let (manager, _temp_dir) = create_test_manager().await;

    let project_ids = vec![
        "proj-1".to_string(),
        "proj-2".to_string(),
        "proj-3".to_string(),
    ];
    let params = serde_json::json!({"email": "user@example.com", "role": "admin"});

    let operation_id = manager
        .create_operation(OperationType::AddUser, params.clone(), project_ids.clone())
        .await
        .unwrap();

    let state = manager.load_operation(operation_id).await.unwrap();

    assert_eq!(state.operation_id, operation_id);
    assert_eq!(state.operation_type, OperationType::AddUser);
    assert_eq!(state.status, OperationStatus::Pending);
    assert_eq!(state.project_ids.len(), 3);
    assert_eq!(state.results.len(), 0);
}

/// Test updating operation with item completions
#[tokio::test]
async fn test_update_item_completions() {
    let (manager, _temp_dir) = create_test_manager().await;

    let project_ids = vec![
        "proj-1".to_string(),
        "proj-2".to_string(),
        "proj-3".to_string(),
    ];
    let operation_id = manager
        .create_operation(
            OperationType::UpdateRole,
            serde_json::json!({}),
            project_ids,
        )
        .await
        .unwrap();

    // Complete first project successfully
    manager
        .update_state(
            operation_id,
            StateUpdate::ItemCompleted {
                project_id: "proj-1".to_string(),
                result: ItemResult::Success,
                attempts: 1,
            },
        )
        .await
        .unwrap();

    // Skip second project
    manager
        .update_state(
            operation_id,
            StateUpdate::ItemCompleted {
                project_id: "proj-2".to_string(),
                result: ItemResult::Skipped {
                    reason: "user_not_in_project".to_string(),
                },
                attempts: 1,
            },
        )
        .await
        .unwrap();

    let state = manager.load_operation(operation_id).await.unwrap();
    assert_eq!(state.results.len(), 2);
    assert!(matches!(
        state.results["proj-1"].result,
        ItemResult::Success
    ));
    assert!(matches!(
        state.results["proj-2"].result,
        ItemResult::Skipped { .. }
    ));
}

/// Test get_pending_projects returns only unprocessed projects
#[tokio::test]
async fn test_get_pending_projects() {
    let (manager, _temp_dir) = create_test_manager().await;

    let project_ids = vec![
        "proj-1".to_string(),
        "proj-2".to_string(),
        "proj-3".to_string(),
        "proj-4".to_string(),
    ];
    let operation_id = manager
        .create_operation(
            OperationType::RemoveUser,
            serde_json::json!({}),
            project_ids,
        )
        .await
        .unwrap();

    // Complete two projects
    for project_id in &["proj-1", "proj-3"] {
        manager
            .update_state(
                operation_id,
                StateUpdate::ItemCompleted {
                    project_id: project_id.to_string(),
                    result: ItemResult::Success,
                    attempts: 1,
                },
            )
            .await
            .unwrap();
    }

    let state = manager.load_operation(operation_id).await.unwrap();
    let pending = manager.get_pending_projects(&state);

    assert_eq!(pending.len(), 2);
    assert!(pending.contains(&"proj-2".to_string()));
    assert!(pending.contains(&"proj-4".to_string()));
    assert!(!pending.contains(&"proj-1".to_string()));
    assert!(!pending.contains(&"proj-3".to_string()));
}

/// Test list_operations returns all operations
#[tokio::test]
async fn test_list_operations() {
    let (manager, _temp_dir) = create_test_manager().await;

    // Create multiple operations
    let op1 = manager
        .create_operation(
            OperationType::AddUser,
            serde_json::json!({}),
            vec!["proj-1".to_string()],
        )
        .await
        .unwrap();

    let op2 = manager
        .create_operation(
            OperationType::UpdateRole,
            serde_json::json!({}),
            vec!["proj-2".to_string()],
        )
        .await
        .unwrap();

    // List all operations
    let operations = manager.list_operations(None).await.unwrap();
    assert_eq!(operations.len(), 2);

    // Verify both operations are in the list
    let op_ids: Vec<_> = operations.iter().map(|o| o.operation_id).collect();
    assert!(op_ids.contains(&op1));
    assert!(op_ids.contains(&op2));
}

/// Test list_operations with status filter
#[tokio::test]
async fn test_list_operations_with_filter() {
    let (manager, _temp_dir) = create_test_manager().await;

    // Create operations with different statuses
    let op1 = manager
        .create_operation(
            OperationType::AddUser,
            serde_json::json!({}),
            vec!["proj-1".to_string()],
        )
        .await
        .unwrap();

    let op2 = manager
        .create_operation(
            OperationType::UpdateRole,
            serde_json::json!({}),
            vec!["proj-2".to_string()],
        )
        .await
        .unwrap();

    // Mark op2 as in progress
    manager
        .update_state(
            op2,
            StateUpdate::StatusChanged {
                status: OperationStatus::InProgress,
            },
        )
        .await
        .unwrap();

    // List only pending operations
    let pending_ops = manager
        .list_operations(Some(OperationStatus::Pending))
        .await
        .unwrap();
    assert_eq!(pending_ops.len(), 1);
    assert_eq!(pending_ops[0].operation_id, op1);

    // List only in-progress operations
    let in_progress_ops = manager
        .list_operations(Some(OperationStatus::InProgress))
        .await
        .unwrap();
    assert_eq!(in_progress_ops.len(), 1);
    assert_eq!(in_progress_ops[0].operation_id, op2);
}

/// Test get_resumable_operation returns most recent in-progress operation
#[tokio::test]
async fn test_get_resumable_operation() {
    let (manager, _temp_dir) = create_test_manager().await;

    // Create a pending operation
    let _op1 = manager
        .create_operation(
            OperationType::AddUser,
            serde_json::json!({}),
            vec!["proj-1".to_string()],
        )
        .await
        .unwrap();

    // No resumable operations yet (all pending)
    let resumable = manager.get_resumable_operation().await.unwrap();
    assert!(resumable.is_none());

    // Create an in-progress operation
    let op2 = manager
        .create_operation(
            OperationType::UpdateRole,
            serde_json::json!({}),
            vec!["proj-2".to_string()],
        )
        .await
        .unwrap();

    manager
        .update_state(
            op2,
            StateUpdate::StatusChanged {
                status: OperationStatus::InProgress,
            },
        )
        .await
        .unwrap();

    // Now we should have a resumable operation
    let resumable = manager.get_resumable_operation().await.unwrap();
    assert_eq!(resumable, Some(op2));
}

/// Test cancel_operation
#[tokio::test]
async fn test_cancel_operation() {
    let (manager, _temp_dir) = create_test_manager().await;

    let operation_id = manager
        .create_operation(
            OperationType::AddUser,
            serde_json::json!({}),
            vec!["proj-1".to_string(), "proj-2".to_string()],
        )
        .await
        .unwrap();

    // Mark as in progress
    manager
        .update_state(
            operation_id,
            StateUpdate::StatusChanged {
                status: OperationStatus::InProgress,
            },
        )
        .await
        .unwrap();

    // Complete one item
    manager
        .update_state(
            operation_id,
            StateUpdate::ItemCompleted {
                project_id: "proj-1".to_string(),
                result: ItemResult::Success,
                attempts: 1,
            },
        )
        .await
        .unwrap();

    // Cancel the operation
    manager.cancel_operation(operation_id).await.unwrap();

    // Verify it's cancelled
    let state = manager.load_operation(operation_id).await.unwrap();
    assert_eq!(state.status, OperationStatus::Cancelled);

    // Results from before cancellation should be preserved
    assert_eq!(state.results.len(), 1);
    assert!(matches!(
        state.results["proj-1"].result,
        ItemResult::Success
    ));
}

/// Test cannot cancel completed operation
#[tokio::test]
async fn test_cannot_cancel_completed_operation() {
    let (manager, _temp_dir) = create_test_manager().await;

    let operation_id = manager
        .create_operation(
            OperationType::AddUser,
            serde_json::json!({}),
            vec!["proj-1".to_string()],
        )
        .await
        .unwrap();

    // Complete the operation
    manager
        .complete_operation(operation_id, OperationStatus::Completed)
        .await
        .unwrap();

    // Try to cancel - should fail
    let result = manager.cancel_operation(operation_id).await;
    assert!(result.is_err());
}

/// Test operation summary counts are correct
#[tokio::test]
async fn test_operation_summary_counts() {
    let (manager, _temp_dir) = create_test_manager().await;

    let project_ids = vec![
        "proj-1".to_string(),
        "proj-2".to_string(),
        "proj-3".to_string(),
        "proj-4".to_string(),
        "proj-5".to_string(),
    ];
    let operation_id = manager
        .create_operation(OperationType::AddUser, serde_json::json!({}), project_ids)
        .await
        .unwrap();

    // Add mixed results
    manager
        .update_state(
            operation_id,
            StateUpdate::ItemCompleted {
                project_id: "proj-1".to_string(),
                result: ItemResult::Success,
                attempts: 1,
            },
        )
        .await
        .unwrap();

    manager
        .update_state(
            operation_id,
            StateUpdate::ItemCompleted {
                project_id: "proj-2".to_string(),
                result: ItemResult::Success,
                attempts: 1,
            },
        )
        .await
        .unwrap();

    manager
        .update_state(
            operation_id,
            StateUpdate::ItemCompleted {
                project_id: "proj-3".to_string(),
                result: ItemResult::Skipped {
                    reason: "already_exists".to_string(),
                },
                attempts: 1,
            },
        )
        .await
        .unwrap();

    manager
        .update_state(
            operation_id,
            StateUpdate::ItemCompleted {
                project_id: "proj-4".to_string(),
                result: ItemResult::Failed {
                    error: "permission denied".to_string(),
                    retryable: false,
                },
                attempts: 2,
            },
        )
        .await
        .unwrap();

    let operations = manager.list_operations(None).await.unwrap();
    let summary = operations
        .iter()
        .find(|o| o.operation_id == operation_id)
        .unwrap();

    assert_eq!(summary.total, 5);
    assert_eq!(summary.completed, 2);
    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.failed, 1);
}

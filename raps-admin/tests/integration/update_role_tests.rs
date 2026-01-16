// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Integration tests for bulk_update_role operation

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use raps_admin::bulk::executor::{
    BulkConfig, BulkExecutor, ItemResult, ProcessItem, ProgressUpdate,
};
use uuid::Uuid;

/// Test successful bulk role update with all items succeeding
#[tokio::test]
async fn test_bulk_update_role_success_all_items() {
    let executor = BulkExecutor::new(BulkConfig {
        concurrency: 5,
        max_retries: 3,
        retry_base_delay: Duration::from_millis(10),
        continue_on_error: true,
        dry_run: false,
    });

    let items: Vec<ProcessItem> = (1..=10)
        .map(|i| ProcessItem {
            project_id: format!("proj-{}", i),
            project_name: Some(format!("Project {}", i)),
        })
        .collect();

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            |_project_id| async { ItemResult::Success },
            |_: ProgressUpdate| {},
        )
        .await;

    assert_eq!(result.total, 10);
    assert_eq!(result.completed, 10);
    assert_eq!(result.failed, 0);
    assert_eq!(result.skipped, 0);
}

/// Test bulk role update with from-role filtering (skip if current role doesn't match)
#[tokio::test]
async fn test_bulk_update_role_from_role_filter() {
    let executor = BulkExecutor::new(BulkConfig::default());

    // Simulate: Projects 1,3,5 have "viewer" role, others have "editor"
    let viewer_projects: std::collections::HashSet<String> = ["proj-1", "proj-3", "proj-5"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let viewers = Arc::new(viewer_projects);

    let items: Vec<ProcessItem> = (1..=6)
        .map(|i| ProcessItem {
            project_id: format!("proj-{}", i),
            project_name: Some(format!("Project {}", i)),
        })
        .collect();

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            move |project_id| {
                let viewers = Arc::clone(&viewers);
                async move {
                    // Simulating from-role filter: only update if current role is "viewer"
                    if viewers.contains(&project_id) {
                        ItemResult::Success // Role was "viewer", updated to new role
                    } else {
                        ItemResult::Skipped {
                            reason: "role_mismatch: current=editor".to_string(),
                        }
                    }
                }
            },
            |_| {},
        )
        .await;

    assert_eq!(result.total, 6);
    assert_eq!(result.completed, 3); // Projects 1, 3, 5
    assert_eq!(result.skipped, 3); // Projects 2, 4, 6 (role mismatch)
    assert_eq!(result.failed, 0);
}

/// Test bulk role update skips users not in project
#[tokio::test]
async fn test_bulk_update_role_user_not_in_project() {
    let executor = BulkExecutor::new(BulkConfig::default());

    // User is only in projects 1 and 3
    let user_projects: std::collections::HashSet<String> =
        ["proj-1", "proj-3"].iter().map(|s| s.to_string()).collect();
    let user_projs = Arc::new(user_projects);

    let items: Vec<ProcessItem> = (1..=5)
        .map(|i| ProcessItem {
            project_id: format!("proj-{}", i),
            project_name: Some(format!("Project {}", i)),
        })
        .collect();

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            move |project_id| {
                let user_projs = Arc::clone(&user_projs);
                async move {
                    if user_projs.contains(&project_id) {
                        ItemResult::Success
                    } else {
                        ItemResult::Skipped {
                            reason: "user_not_in_project".to_string(),
                        }
                    }
                }
            },
            |_| {},
        )
        .await;

    assert_eq!(result.total, 5);
    assert_eq!(result.completed, 2); // Projects 1, 3
    assert_eq!(result.skipped, 3); // Projects 2, 4, 5
    assert_eq!(result.failed, 0);
}

/// Test bulk role update skips if user already has target role
#[tokio::test]
async fn test_bulk_update_role_already_has_role() {
    let executor = BulkExecutor::new(BulkConfig::default());

    // User already has "admin" role in projects 2 and 4
    let already_admin: std::collections::HashSet<String> =
        ["proj-2", "proj-4"].iter().map(|s| s.to_string()).collect();
    let admins = Arc::new(already_admin);

    let items: Vec<ProcessItem> = (1..=5)
        .map(|i| ProcessItem {
            project_id: format!("proj-{}", i),
            project_name: Some(format!("Project {}", i)),
        })
        .collect();

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            move |project_id| {
                let admins = Arc::clone(&admins);
                async move {
                    if admins.contains(&project_id) {
                        ItemResult::Skipped {
                            reason: "already_has_role".to_string(),
                        }
                    } else {
                        ItemResult::Success
                    }
                }
            },
            |_| {},
        )
        .await;

    assert_eq!(result.total, 5);
    assert_eq!(result.completed, 3); // Projects 1, 3, 5
    assert_eq!(result.skipped, 2); // Projects 2, 4
    assert_eq!(result.failed, 0);
}

/// Test bulk role update with mixed results including failures
#[tokio::test]
async fn test_bulk_update_role_mixed_results() {
    let executor = BulkExecutor::new(BulkConfig {
        max_retries: 1,
        retry_base_delay: Duration::from_millis(1),
        ..Default::default()
    });

    let items: Vec<ProcessItem> = (1..=9)
        .map(|i| ProcessItem {
            project_id: format!("proj-{}", i),
            project_name: Some(format!("Project {}", i)),
        })
        .collect();

    let counter = Arc::new(AtomicUsize::new(0));

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            move |_project_id| {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                async move {
                    match count % 3 {
                        0 => ItemResult::Success,
                        1 => ItemResult::Skipped {
                            reason: "already_has_role".to_string(),
                        },
                        _ => ItemResult::Failed {
                            error: "permission denied".to_string(),
                            retryable: false,
                        },
                    }
                }
            },
            |_| {},
        )
        .await;

    assert_eq!(result.total, 9);
    assert_eq!(result.completed, 3);
    assert_eq!(result.skipped, 3);
    assert_eq!(result.failed, 3);
}

/// Test bulk role update in dry-run mode
#[tokio::test]
async fn test_bulk_update_role_dry_run() {
    let executor = BulkExecutor::new(BulkConfig {
        dry_run: true,
        ..Default::default()
    });

    let items: Vec<ProcessItem> = (1..=5)
        .map(|i| ProcessItem {
            project_id: format!("proj-{}", i),
            project_name: Some(format!("Project {}", i)),
        })
        .collect();

    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_check = Arc::clone(&call_count);

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            move |_project_id| {
                let counter = Arc::clone(&call_count);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    ItemResult::Success
                }
            },
            |_| {},
        )
        .await;

    // In dry-run mode, all items should be skipped
    assert_eq!(result.total, 5);
    assert_eq!(result.completed, 0);
    assert_eq!(result.skipped, 5);
    assert_eq!(result.failed, 0);

    // Processor should not have been called
    assert_eq!(call_count_check.load(Ordering::SeqCst), 0);
}

/// Test that skip reasons are correctly recorded in details
#[tokio::test]
async fn test_bulk_update_role_skip_reasons_in_details() {
    let executor = BulkExecutor::new(BulkConfig::default());

    let items = vec![
        ProcessItem {
            project_id: "proj-1".to_string(),
            project_name: Some("Project 1".to_string()),
        },
        ProcessItem {
            project_id: "proj-2".to_string(),
            project_name: Some("Project 2".to_string()),
        },
        ProcessItem {
            project_id: "proj-3".to_string(),
            project_name: Some("Project 3".to_string()),
        },
    ];

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            |project_id| async move {
                match project_id.as_str() {
                    "proj-1" => ItemResult::Success,
                    "proj-2" => ItemResult::Skipped {
                        reason: "user_not_in_project".to_string(),
                    },
                    _ => ItemResult::Skipped {
                        reason: "already_has_role".to_string(),
                    },
                }
            },
            |_| {},
        )
        .await;

    assert_eq!(result.details.len(), 3);

    // Check proj-1 success
    let d1 = result
        .details
        .iter()
        .find(|d| d.project_id == "proj-1")
        .unwrap();
    assert!(matches!(d1.result, ItemResult::Success));

    // Check proj-2 skipped with correct reason
    let d2 = result
        .details
        .iter()
        .find(|d| d.project_id == "proj-2")
        .unwrap();
    match &d2.result {
        ItemResult::Skipped { reason } => assert_eq!(reason, "user_not_in_project"),
        _ => panic!("Expected Skipped result"),
    }

    // Check proj-3 skipped with correct reason
    let d3 = result
        .details
        .iter()
        .find(|d| d.project_id == "proj-3")
        .unwrap();
    match &d3.result {
        ItemResult::Skipped { reason } => assert_eq!(reason, "already_has_role"),
        _ => panic!("Expected Skipped result"),
    }
}

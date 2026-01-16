// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Integration tests for bulk_remove_user operation

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use raps_admin::bulk::executor::{
    BulkConfig, BulkExecutor, ItemResult, ProcessItem, ProgressUpdate,
};
use uuid::Uuid;

/// Test successful bulk remove user with all items succeeding
#[tokio::test]
async fn test_bulk_remove_user_success_all_items() {
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

/// Test bulk remove user skips users not in project
#[tokio::test]
async fn test_bulk_remove_user_not_in_project() {
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

/// Test bulk remove user with mixed results including failures
#[tokio::test]
async fn test_bulk_remove_user_mixed_results() {
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
                            reason: "user_not_in_project".to_string(),
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

/// Test bulk remove user in dry-run mode
#[tokio::test]
async fn test_bulk_remove_user_dry_run() {
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
async fn test_bulk_remove_user_skip_reasons_in_details() {
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
                    _ => ItemResult::Failed {
                        error: "permission denied".to_string(),
                        retryable: false,
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

    // Check proj-3 failed
    let d3 = result
        .details
        .iter()
        .find(|d| d.project_id == "proj-3")
        .unwrap();
    match &d3.result {
        ItemResult::Failed { error, .. } => assert!(error.contains("permission denied")),
        _ => panic!("Expected Failed result"),
    }
}

/// Test concurrency limit is respected
#[tokio::test]
async fn test_bulk_remove_user_respects_concurrency() {
    let executor = BulkExecutor::new(BulkConfig {
        concurrency: 2,
        ..Default::default()
    });

    let items: Vec<ProcessItem> = (1..=6)
        .map(|i| ProcessItem {
            project_id: format!("proj-{}", i),
            project_name: Some(format!("Project {}", i)),
        })
        .collect();

    let concurrent_count = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));
    let max_concurrent_clone = Arc::clone(&max_concurrent);

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            move |_project_id| {
                let concurrent = Arc::clone(&concurrent_count);
                let max = Arc::clone(&max_concurrent);
                async move {
                    let current = concurrent.fetch_add(1, Ordering::SeqCst) + 1;
                    // Update max if current is higher
                    loop {
                        let old_max = max.load(Ordering::SeqCst);
                        if current <= old_max {
                            break;
                        }
                        if max
                            .compare_exchange(old_max, current, Ordering::SeqCst, Ordering::SeqCst)
                            .is_ok()
                        {
                            break;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    concurrent.fetch_sub(1, Ordering::SeqCst);
                    ItemResult::Success
                }
            },
            |_| {},
        )
        .await;

    assert_eq!(result.total, 6);
    assert_eq!(result.completed, 6);
    // Max concurrent should not exceed the configured limit of 2
    assert!(max_concurrent_clone.load(Ordering::SeqCst) <= 2);
}

/// Test progress callback is called for each item
#[tokio::test]
async fn test_bulk_remove_user_progress_callback() {
    let executor = BulkExecutor::new(BulkConfig::default());

    let items: Vec<ProcessItem> = (1..=5)
        .map(|i| ProcessItem {
            project_id: format!("proj-{}", i),
            project_name: Some(format!("Project {}", i)),
        })
        .collect();

    let progress_count = Arc::new(AtomicUsize::new(0));
    let progress_count_clone = Arc::clone(&progress_count);

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            |_project_id| async { ItemResult::Success },
            move |_update: ProgressUpdate| {
                progress_count_clone.fetch_add(1, Ordering::SeqCst);
            },
        )
        .await;

    assert_eq!(result.total, 5);
    assert_eq!(result.completed, 5);
    // Progress callback should be called at least once per item
    assert!(progress_count.load(Ordering::SeqCst) >= 5);
}

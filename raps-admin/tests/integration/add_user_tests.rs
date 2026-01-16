// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Integration tests for bulk_add_user operation

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use raps_admin::bulk::executor::{
    BulkConfig, BulkExecutor, ItemResult, ProcessItem, ProgressUpdate,
};
use uuid::Uuid;

/// Test successful bulk add operation with all items succeeding
#[tokio::test]
async fn test_bulk_add_success_all_items() {
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

    let progress_updates = Arc::new(std::sync::Mutex::new(Vec::new()));
    let progress_clone = Arc::clone(&progress_updates);

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            |_project_id| async { ItemResult::Success },
            move |progress: ProgressUpdate| {
                progress_clone.lock().unwrap().push(progress);
            },
        )
        .await;

    assert_eq!(result.total, 10);
    assert_eq!(result.completed, 10);
    assert_eq!(result.failed, 0);
    assert_eq!(result.skipped, 0);

    // Verify progress updates were received
    let updates = progress_updates.lock().unwrap();
    assert!(!updates.is_empty());

    // Last update should show all completed
    let last = updates.last().unwrap();
    assert_eq!(last.completed + last.skipped + last.failed, 10);
}

/// Test bulk operation with mixed results (success, skip, fail)
#[tokio::test]
async fn test_bulk_add_mixed_results() {
    let executor = BulkExecutor::new(BulkConfig {
        concurrency: 5,
        max_retries: 1, // Limit retries for faster test
        retry_base_delay: Duration::from_millis(1),
        continue_on_error: true,
        dry_run: false,
    });

    let items: Vec<ProcessItem> = (1..=9)
        .map(|i| ProcessItem {
            project_id: format!("proj-{}", i),
            project_name: Some(format!("Project {}", i)),
        })
        .collect();

    // Counter to cycle through different results
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
                            reason: "already_exists".to_string(),
                        },
                        _ => ItemResult::Failed {
                            error: "test error".to_string(),
                            retryable: false,
                        },
                    }
                }
            },
            |_| {},
        )
        .await;

    assert_eq!(result.total, 9);
    assert_eq!(result.completed, 3); // Items 0, 3, 6
    assert_eq!(result.skipped, 3); // Items 1, 4, 7
    assert_eq!(result.failed, 3); // Items 2, 5, 8
}

/// Test bulk operation with duplicate detection (skip if exists)
#[tokio::test]
async fn test_bulk_add_duplicate_detection() {
    let executor = BulkExecutor::new(BulkConfig::default());

    // Simulate existing users in projects 1, 3, 5
    let existing_projects: std::collections::HashSet<String> = ["proj-1", "proj-3", "proj-5"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let existing = Arc::new(existing_projects);

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
                let existing = Arc::clone(&existing);
                async move {
                    if existing.contains(&project_id) {
                        ItemResult::Skipped {
                            reason: "already_exists".to_string(),
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
    assert_eq!(result.completed, 2); // Projects 2, 4
    assert_eq!(result.skipped, 3); // Projects 1, 3, 5
    assert_eq!(result.failed, 0);
}

/// Test bulk operation with retry on transient failures
#[tokio::test]
async fn test_bulk_add_retry_on_transient_failure() {
    let executor = BulkExecutor::new(BulkConfig {
        concurrency: 1,
        max_retries: 3,
        retry_base_delay: Duration::from_millis(1),
        continue_on_error: true,
        dry_run: false,
    });

    let items = vec![ProcessItem {
        project_id: "proj-1".to_string(),
        project_name: Some("Project 1".to_string()),
    }];

    // Fail twice, then succeed on third attempt
    let attempt_counter = Arc::new(AtomicUsize::new(0));

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            move |_project_id| {
                let counter = Arc::clone(&attempt_counter);
                async move {
                    let attempt = counter.fetch_add(1, Ordering::SeqCst);
                    if attempt < 2 {
                        ItemResult::Failed {
                            error: "429 Rate limit exceeded".to_string(),
                            retryable: true,
                        }
                    } else {
                        ItemResult::Success
                    }
                }
            },
            |_| {},
        )
        .await;

    assert_eq!(result.total, 1);
    assert_eq!(result.completed, 1);
    assert_eq!(result.failed, 0);

    // Verify the item shows 3 attempts
    assert_eq!(result.details[0].attempts, 3);
}

/// Test bulk operation in dry-run mode
#[tokio::test]
async fn test_bulk_add_dry_run() {
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

    // This processor should never be called in dry-run mode
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

/// Test concurrency limiting
#[tokio::test]
async fn test_bulk_add_concurrency_limit() {
    let executor = BulkExecutor::new(BulkConfig {
        concurrency: 2, // Only 2 concurrent operations
        max_retries: 1,
        retry_base_delay: Duration::from_millis(1),
        continue_on_error: true,
        dry_run: false,
    });

    let items: Vec<ProcessItem> = (1..=10)
        .map(|i| ProcessItem {
            project_id: format!("proj-{}", i),
            project_name: Some(format!("Project {}", i)),
        })
        .collect();

    // Track concurrent operations
    let active = Arc::new(AtomicUsize::new(0));
    let max_concurrent = Arc::new(AtomicUsize::new(0));
    let max_concurrent_check = Arc::clone(&max_concurrent);

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            move |_project_id| {
                let active = Arc::clone(&active);
                let max_concurrent = Arc::clone(&max_concurrent);
                async move {
                    // Increment active count
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;

                    // Track maximum
                    let mut max = max_concurrent.load(Ordering::SeqCst);
                    while current > max {
                        match max_concurrent.compare_exchange_weak(
                            max,
                            current,
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                        ) {
                            Ok(_) => break,
                            Err(x) => max = x,
                        }
                    }

                    // Small delay to allow concurrency to be tested
                    tokio::time::sleep(Duration::from_millis(10)).await;

                    // Decrement active count
                    active.fetch_sub(1, Ordering::SeqCst);

                    ItemResult::Success
                }
            },
            |_| {},
        )
        .await;

    assert_eq!(result.total, 10);
    assert_eq!(result.completed, 10);

    // Maximum concurrent should not exceed the configured limit
    // Note: Due to timing, it might be slightly less but never more
    assert!(max_concurrent_check.load(Ordering::SeqCst) <= 2);
}

/// Test that details contain correct project information
#[tokio::test]
async fn test_bulk_add_details_contain_project_info() {
    let executor = BulkExecutor::new(BulkConfig::default());

    let items = vec![
        ProcessItem {
            project_id: "proj-abc".to_string(),
            project_name: Some("Alpha Project".to_string()),
        },
        ProcessItem {
            project_id: "proj-xyz".to_string(),
            project_name: Some("Zeta Project".to_string()),
        },
    ];

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            |project_id| async move {
                if project_id == "proj-abc" {
                    ItemResult::Success
                } else {
                    ItemResult::Skipped {
                        reason: "test skip".to_string(),
                    }
                }
            },
            |_| {},
        )
        .await;

    assert_eq!(result.details.len(), 2);

    // Find the alpha project detail
    let alpha = result
        .details
        .iter()
        .find(|d| d.project_id == "proj-abc")
        .unwrap();
    assert_eq!(alpha.project_name, Some("Alpha Project".to_string()));
    assert!(matches!(alpha.result, ItemResult::Success));

    // Find the zeta project detail
    let zeta = result
        .details
        .iter()
        .find(|d| d.project_id == "proj-xyz")
        .unwrap();
    assert_eq!(zeta.project_name, Some("Zeta Project".to_string()));
    assert!(matches!(zeta.result, ItemResult::Skipped { .. }));
}

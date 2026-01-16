// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Integration tests for dry-run mode across all bulk operations

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use raps_admin::bulk::executor::{
    BulkConfig, BulkExecutor, ItemResult, ProcessItem, ProgressUpdate,
};
use uuid::Uuid;

/// Test that dry-run mode skips all items without calling processor
#[tokio::test]
async fn test_dry_run_skips_all_items() {
    let executor = BulkExecutor::new(BulkConfig {
        dry_run: true,
        ..Default::default()
    });

    let items: Vec<ProcessItem> = (1..=10)
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
            |_: ProgressUpdate| {},
        )
        .await;

    // All items should be skipped in dry-run mode
    assert_eq!(result.total, 10);
    assert_eq!(result.completed, 0);
    assert_eq!(result.skipped, 10);
    assert_eq!(result.failed, 0);

    // Processor should NOT have been called
    assert_eq!(call_count_check.load(Ordering::SeqCst), 0);
}

/// Test that dry-run mode records "dry_run" as skip reason
#[tokio::test]
async fn test_dry_run_records_skip_reason() {
    let executor = BulkExecutor::new(BulkConfig {
        dry_run: true,
        ..Default::default()
    });

    let items = vec![
        ProcessItem {
            project_id: "proj-1".to_string(),
            project_name: Some("Project 1".to_string()),
        },
        ProcessItem {
            project_id: "proj-2".to_string(),
            project_name: Some("Project 2".to_string()),
        },
    ];

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            |_project_id| async { ItemResult::Success },
            |_| {},
        )
        .await;

    assert_eq!(result.details.len(), 2);

    // All items should have "dry-run mode" as skip reason
    for detail in &result.details {
        match &detail.result {
            ItemResult::Skipped { reason } => {
                assert_eq!(reason, "dry-run mode");
            }
            _ => panic!("Expected Skipped result with dry-run mode reason"),
        }
    }
}

/// Test that dry-run mode still tracks progress
#[tokio::test]
async fn test_dry_run_tracks_progress() {
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

    let progress_updates = Arc::new(std::sync::Mutex::new(Vec::new()));
    let progress_updates_clone = Arc::clone(&progress_updates);

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            |_project_id| async { ItemResult::Success },
            move |update: ProgressUpdate| {
                progress_updates_clone.lock().unwrap().push(update);
            },
        )
        .await;

    assert_eq!(result.total, 5);
    assert_eq!(result.skipped, 5);

    // Progress updates should have been recorded
    let updates = progress_updates.lock().unwrap();
    assert!(!updates.is_empty());

    // Final update should show all items as skipped
    if let Some(last_update) = updates.last() {
        assert_eq!(last_update.total, 5);
        assert_eq!(last_update.skipped, 5);
        assert_eq!(last_update.completed, 0);
        assert_eq!(last_update.failed, 0);
    }
}

/// Test that dry-run mode preserves project information in details
#[tokio::test]
async fn test_dry_run_preserves_project_info() {
    let executor = BulkExecutor::new(BulkConfig {
        dry_run: true,
        ..Default::default()
    });

    let items = vec![
        ProcessItem {
            project_id: "proj-abc-123".to_string(),
            project_name: Some("Alpha Project".to_string()),
        },
        ProcessItem {
            project_id: "proj-def-456".to_string(),
            project_name: Some("Beta Project".to_string()),
        },
    ];

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            |_project_id| async { ItemResult::Success },
            |_| {},
        )
        .await;

    // Verify project information is preserved in details
    let detail1 = result
        .details
        .iter()
        .find(|d| d.project_id == "proj-abc-123");
    assert!(detail1.is_some());
    let d1 = detail1.unwrap();
    assert_eq!(d1.project_name, Some("Alpha Project".to_string()));

    let detail2 = result
        .details
        .iter()
        .find(|d| d.project_id == "proj-def-456");
    assert!(detail2.is_some());
    let d2 = detail2.unwrap();
    assert_eq!(d2.project_name, Some("Beta Project".to_string()));
}

/// Test that dry-run mode works with zero items
#[tokio::test]
async fn test_dry_run_with_empty_items() {
    let executor = BulkExecutor::new(BulkConfig {
        dry_run: true,
        ..Default::default()
    });

    let items: Vec<ProcessItem> = vec![];

    let result = executor
        .execute(
            Uuid::new_v4(),
            items,
            |_project_id| async { ItemResult::Success },
            |_: ProgressUpdate| {},
        )
        .await;

    assert_eq!(result.total, 0);
    assert_eq!(result.completed, 0);
    assert_eq!(result.skipped, 0);
    assert_eq!(result.failed, 0);
    assert!(result.details.is_empty());
}

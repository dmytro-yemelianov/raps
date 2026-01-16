// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Parallel execution engine for bulk operations

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::bulk::retry::exponential_backoff;

/// Configuration for bulk execution
#[derive(Debug, Clone)]
pub struct BulkConfig {
    /// Number of concurrent operations (default: 10)
    pub concurrency: usize,
    /// Maximum retry attempts per item (default: 5)
    pub max_retries: usize,
    /// Base delay for exponential backoff (default: 1s)
    pub retry_base_delay: Duration,
    /// Continue processing even if some items fail (default: true)
    pub continue_on_error: bool,
    /// Preview mode - don't execute actual API calls (default: false)
    pub dry_run: bool,
}

impl Default for BulkConfig {
    fn default() -> Self {
        Self {
            concurrency: 10,
            max_retries: 5,
            retry_base_delay: Duration::from_secs(1),
            continue_on_error: true,
            dry_run: false,
        }
    }
}

/// Progress update for callbacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    /// Total number of items to process
    pub total: usize,
    /// Number of successfully completed items
    pub completed: usize,
    /// Number of failed items
    pub failed: usize,
    /// Number of skipped items
    pub skipped: usize,
    /// Current item being processed (for display)
    pub current_item: Option<String>,
    /// Estimated time remaining
    #[serde(skip)]
    pub estimated_remaining: Option<Duration>,
}

impl ProgressUpdate {
    /// Calculate completion percentage
    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            (self.completed + self.failed + self.skipped) as f64 / self.total as f64 * 100.0
        }
    }

    /// Check if processing is complete
    pub fn is_complete(&self) -> bool {
        self.completed + self.failed + self.skipped >= self.total
    }
}

/// Result of processing a single item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemResult {
    /// Item processed successfully
    Success,
    /// Item was skipped (e.g., already exists)
    Skipped { reason: String },
    /// Item failed after all retries
    Failed { error: String, retryable: bool },
}

/// Detail of a single item's processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDetail {
    /// Project ID this result is for
    pub project_id: String,
    /// Project name (for display)
    pub project_name: Option<String>,
    /// Processing result
    pub result: ItemResult,
    /// Number of attempts made
    pub attempts: u32,
}

/// Final result of a bulk operation
#[derive(Debug)]
pub struct BulkOperationResult {
    /// Unique operation identifier
    pub operation_id: Uuid,
    /// Total items processed
    pub total: usize,
    /// Successfully completed items
    pub completed: usize,
    /// Failed items
    pub failed: usize,
    /// Skipped items
    pub skipped: usize,
    /// Total duration
    pub duration: Duration,
    /// Per-item details
    pub details: Vec<ItemDetail>,
}

/// Item to be processed (with metadata)
pub struct ProcessItem {
    /// Project ID
    pub project_id: String,
    /// Project name (for display)
    pub project_name: Option<String>,
}

/// Bulk operation executor
///
/// Orchestrates parallel execution of bulk operations with:
/// - Configurable concurrency using semaphores
/// - Retry logic with exponential backoff
/// - Progress tracking and callbacks
pub struct BulkExecutor {
    config: BulkConfig,
}

impl BulkExecutor {
    /// Create a new executor with the given configuration
    pub fn new(config: BulkConfig) -> Self {
        Self { config }
    }

    /// Get the configuration
    pub fn config(&self) -> &BulkConfig {
        &self.config
    }

    /// Execute a bulk operation with progress tracking
    ///
    /// # Arguments
    /// * `operation_id` - Unique identifier for this operation
    /// * `items` - List of items to process
    /// * `processor` - Async function to process each item
    /// * `on_progress` - Callback for progress updates
    ///
    /// # Type Parameters
    /// * `F` - Processor function type
    /// * `Fut` - Future returned by processor
    pub async fn execute<F, Fut, P>(
        &self,
        operation_id: Uuid,
        items: Vec<ProcessItem>,
        processor: F,
        on_progress: P,
    ) -> BulkOperationResult
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ItemResult> + Send,
        P: Fn(ProgressUpdate) + Send + Sync + 'static,
    {
        let start_time = Instant::now();
        let total = items.len();

        // If dry-run, simulate success for all items
        if self.config.dry_run {
            let details: Vec<ItemDetail> = items
                .into_iter()
                .map(|item| ItemDetail {
                    project_id: item.project_id,
                    project_name: item.project_name,
                    result: ItemResult::Skipped {
                        reason: "dry-run mode".to_string(),
                    },
                    attempts: 0,
                })
                .collect();

            on_progress(ProgressUpdate {
                total,
                completed: 0,
                failed: 0,
                skipped: total,
                current_item: None,
                estimated_remaining: None,
            });

            return BulkOperationResult {
                operation_id,
                total,
                completed: 0,
                failed: 0,
                skipped: total,
                duration: start_time.elapsed(),
                details,
            };
        }

        // Shared counters for progress tracking
        let completed = Arc::new(AtomicUsize::new(0));
        let failed = Arc::new(AtomicUsize::new(0));
        let skipped = Arc::new(AtomicUsize::new(0));

        // Semaphore for concurrency control
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));

        // Wrap processor and progress callback in Arc for sharing
        let processor = Arc::new(processor);
        let on_progress = Arc::new(on_progress);

        // Process all items concurrently (limited by semaphore)
        let mut handles = Vec::with_capacity(items.len());

        for item in items {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let processor = Arc::clone(&processor);
            let on_progress = Arc::clone(&on_progress);
            let completed = Arc::clone(&completed);
            let failed = Arc::clone(&failed);
            let skipped = Arc::clone(&skipped);
            let config = self.config.clone();

            let handle = tokio::spawn(async move {
                let result = process_with_retry(
                    &item.project_id,
                    &*processor,
                    config.max_retries,
                    config.retry_base_delay,
                )
                .await;

                // Update counters
                match &result.0 {
                    ItemResult::Success => {
                        completed.fetch_add(1, Ordering::SeqCst);
                    }
                    ItemResult::Failed { .. } => {
                        failed.fetch_add(1, Ordering::SeqCst);
                    }
                    ItemResult::Skipped { .. } => {
                        skipped.fetch_add(1, Ordering::SeqCst);
                    }
                }

                // Send progress update
                on_progress(ProgressUpdate {
                    total,
                    completed: completed.load(Ordering::SeqCst),
                    failed: failed.load(Ordering::SeqCst),
                    skipped: skipped.load(Ordering::SeqCst),
                    current_item: Some(item.project_id.clone()),
                    estimated_remaining: None,
                });

                drop(permit); // Release semaphore permit

                ItemDetail {
                    project_id: item.project_id,
                    project_name: item.project_name,
                    result: result.0,
                    attempts: result.1,
                }
            });

            handles.push(handle);
        }

        // Collect all results
        let mut details = Vec::with_capacity(handles.len());
        for handle in handles {
            if let Ok(detail) = handle.await {
                details.push(detail);
            }
        }

        BulkOperationResult {
            operation_id,
            total,
            completed: completed.load(Ordering::SeqCst),
            failed: failed.load(Ordering::SeqCst),
            skipped: skipped.load(Ordering::SeqCst),
            duration: start_time.elapsed(),
            details,
        }
    }
}

/// Process a single item with retry logic
async fn process_with_retry<F, Fut>(
    project_id: &str,
    processor: &F,
    max_retries: usize,
    base_delay: Duration,
) -> (ItemResult, u32)
where
    F: Fn(String) -> Fut + Send + Sync,
    Fut: Future<Output = ItemResult> + Send,
{
    let max_delay = Duration::from_secs(60);
    let mut attempts = 0u32;

    loop {
        attempts += 1;
        let result = processor(project_id.to_string()).await;

        match &result {
            ItemResult::Success | ItemResult::Skipped { .. } => {
                return (result, attempts);
            }
            ItemResult::Failed { retryable, .. } => {
                if !retryable || attempts as usize >= max_retries {
                    return (result, attempts);
                }

                // Wait before retry with exponential backoff
                let delay = exponential_backoff(attempts - 1, base_delay, max_delay);
                tokio::time::sleep(delay).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    #[tokio::test]
    async fn test_execute_success() {
        let executor = BulkExecutor::new(BulkConfig::default());
        let operation_id = Uuid::new_v4();

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
                operation_id,
                items,
                |_project_id| async { ItemResult::Success },
                |_progress| {},
            )
            .await;

        assert_eq!(result.total, 2);
        assert_eq!(result.completed, 2);
        assert_eq!(result.failed, 0);
        assert_eq!(result.skipped, 0);
    }

    #[tokio::test]
    async fn test_execute_dry_run() {
        let config = BulkConfig {
            dry_run: true,
            ..Default::default()
        };
        let executor = BulkExecutor::new(config);
        let operation_id = Uuid::new_v4();

        let items = vec![ProcessItem {
            project_id: "proj-1".to_string(),
            project_name: None,
        }];

        let result = executor
            .execute(
                operation_id,
                items,
                |_project_id| async { ItemResult::Success },
                |_progress| {},
            )
            .await;

        assert_eq!(result.total, 1);
        assert_eq!(result.skipped, 1);
        assert_eq!(result.completed, 0);
    }

    #[tokio::test]
    async fn test_execute_with_retries() {
        let config = BulkConfig {
            max_retries: 3,
            retry_base_delay: Duration::from_millis(10),
            ..Default::default()
        };
        let executor = BulkExecutor::new(config);
        let operation_id = Uuid::new_v4();

        // Counter to track attempts
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = Arc::clone(&attempts);

        let items = vec![ProcessItem {
            project_id: "proj-1".to_string(),
            project_name: None,
        }];

        let result = executor
            .execute(
                operation_id,
                items,
                move |_project_id| {
                    let attempts = Arc::clone(&attempts_clone);
                    async move {
                        let count = attempts.fetch_add(1, Ordering::SeqCst);
                        if count < 2 {
                            ItemResult::Failed {
                                error: "temporary error".to_string(),
                                retryable: true,
                            }
                        } else {
                            ItemResult::Success
                        }
                    }
                },
                |_progress| {},
            )
            .await;

        assert_eq!(result.completed, 1);
        assert_eq!(result.details[0].attempts, 3); // Initial + 2 retries
    }
}

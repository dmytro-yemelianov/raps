// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Operation state persistence

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::bulk::executor::{ItemResult, ProgressUpdate};
use crate::error::AdminError;
use crate::types::{OperationStatus, OperationType};

/// Manages persistent state for bulk operations
pub struct StateManager {
    state_dir: PathBuf,
}

/// Persisted state of an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationState {
    /// Unique operation identifier
    pub operation_id: Uuid,
    /// Type of operation
    pub operation_type: OperationType,
    /// Current status
    pub status: OperationStatus,
    /// Operation parameters (JSON value for flexibility)
    pub parameters: serde_json::Value,
    /// List of project IDs to process
    pub project_ids: Vec<String>,
    /// Per-project results
    pub results: HashMap<String, ProjectResultState>,
    /// When the operation was created
    pub created_at: DateTime<Utc>,
    /// Last update time
    pub updated_at: DateTime<Utc>,
}

/// State of a single project's processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectResultState {
    /// Processing result
    pub result: ItemResult,
    /// Number of attempts made
    pub attempts: u32,
    /// When processing completed (if done)
    pub completed_at: Option<DateTime<Utc>>,
}

/// Summary of an operation (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationSummary {
    /// Operation ID
    pub operation_id: Uuid,
    /// Type of operation
    pub operation_type: OperationType,
    /// Current status
    pub status: OperationStatus,
    /// Total projects
    pub total: usize,
    /// Completed count
    pub completed: usize,
    /// Failed count
    pub failed: usize,
    /// Skipped count
    pub skipped: usize,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Last update time
    pub updated_at: DateTime<Utc>,
}

/// Types of state updates
pub enum StateUpdate {
    /// An item was processed
    ItemCompleted {
        project_id: String,
        result: ItemResult,
        attempts: u32,
    },
    /// Operation status changed
    StatusChanged { status: OperationStatus },
    /// Progress was updated (for partial saves)
    ProgressUpdated { progress: ProgressUpdate },
}

impl StateManager {
    /// Create a new state manager using the default state directory
    pub fn new() -> Result<Self, AdminError> {
        let state_dir = Self::default_state_dir()?;
        std::fs::create_dir_all(&state_dir)?;
        Ok(Self { state_dir })
    }

    /// Create a state manager with a custom directory (for testing)
    pub fn with_dir(state_dir: PathBuf) -> Result<Self, AdminError> {
        std::fs::create_dir_all(&state_dir)?;
        Ok(Self { state_dir })
    }

    /// Get the state directory path
    pub fn state_dir(&self) -> &PathBuf {
        &self.state_dir
    }

    /// Get the default state directory based on platform
    fn default_state_dir() -> Result<PathBuf, AdminError> {
        let base_dirs =
            directories::ProjectDirs::from("xyz", "rapscli", "raps").ok_or_else(|| {
                AdminError::StateError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Could not determine user data directory",
                ))
            })?;

        Ok(base_dirs.data_dir().join("operations"))
    }

    /// Get the file path for an operation's state
    fn operation_path(&self, operation_id: Uuid) -> PathBuf {
        self.state_dir.join(format!("{}.json", operation_id))
    }

    /// Create a new operation state
    ///
    /// # Arguments
    /// * `operation_type` - Type of bulk operation
    /// * `parameters` - Operation parameters (as JSON)
    /// * `project_ids` - List of project IDs to process
    ///
    /// # Returns
    /// The new operation's UUID
    pub async fn create_operation(
        &self,
        operation_type: OperationType,
        parameters: serde_json::Value,
        project_ids: Vec<String>,
    ) -> Result<Uuid, AdminError> {
        let operation_id = Uuid::new_v4();
        let now = Utc::now();

        let state = OperationState {
            operation_id,
            operation_type,
            status: OperationStatus::Pending,
            parameters,
            project_ids,
            results: HashMap::new(),
            created_at: now,
            updated_at: now,
        };

        self.save_state(&state).await?;
        Ok(operation_id)
    }

    /// Load an existing operation state
    pub async fn load_operation(&self, operation_id: Uuid) -> Result<OperationState, AdminError> {
        let path = self.operation_path(operation_id);

        if !path.exists() {
            return Err(AdminError::OperationNotFound { id: operation_id });
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let state: OperationState = serde_json::from_str(&content)?;

        Ok(state)
    }

    /// Update operation state
    pub async fn update_state(
        &self,
        operation_id: Uuid,
        update: StateUpdate,
    ) -> Result<(), AdminError> {
        let mut state = self.load_operation(operation_id).await?;
        state.updated_at = Utc::now();

        match update {
            StateUpdate::ItemCompleted {
                project_id,
                result,
                attempts,
            } => {
                state.results.insert(
                    project_id,
                    ProjectResultState {
                        result,
                        attempts,
                        completed_at: Some(Utc::now()),
                    },
                );
            }
            StateUpdate::StatusChanged { status } => {
                state.status = status;
            }
            StateUpdate::ProgressUpdated { .. } => {
                // Progress updates don't modify persisted state directly
                // They're used for in-memory tracking
            }
        }

        self.save_state(&state).await
    }

    /// Mark operation as complete with final result
    pub async fn complete_operation(
        &self,
        operation_id: Uuid,
        status: OperationStatus,
    ) -> Result<(), AdminError> {
        let mut state = self.load_operation(operation_id).await?;
        state.status = status;
        state.updated_at = Utc::now();
        self.save_state(&state).await
    }

    /// List all operations, optionally filtered by status
    pub async fn list_operations(
        &self,
        status_filter: Option<OperationStatus>,
    ) -> Result<Vec<OperationSummary>, AdminError> {
        let mut summaries = Vec::new();

        let entries = std::fs::read_dir(&self.state_dir)?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(state) = serde_json::from_str::<OperationState>(&content) {
                        // Apply status filter if provided
                        if let Some(filter_status) = status_filter {
                            if state.status != filter_status {
                                continue;
                            }
                        }

                        let (completed, failed, skipped) = count_results(&state.results);

                        summaries.push(OperationSummary {
                            operation_id: state.operation_id,
                            operation_type: state.operation_type,
                            status: state.status,
                            total: state.project_ids.len(),
                            completed,
                            failed,
                            skipped,
                            created_at: state.created_at,
                            updated_at: state.updated_at,
                        });
                    }
                }
            }
        }

        // Sort by updated_at descending (most recent first)
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(summaries)
    }

    /// Get the most recent incomplete operation
    pub async fn get_resumable_operation(&self) -> Result<Option<Uuid>, AdminError> {
        let operations = self
            .list_operations(Some(OperationStatus::InProgress))
            .await?;

        Ok(operations.first().map(|s| s.operation_id))
    }

    /// Get project IDs that haven't been processed yet
    pub fn get_pending_projects(&self, state: &OperationState) -> Vec<String> {
        state
            .project_ids
            .iter()
            .filter(|id| !state.results.contains_key(*id))
            .cloned()
            .collect()
    }

    /// Cancel an operation (mark as cancelled)
    pub async fn cancel_operation(&self, operation_id: Uuid) -> Result<(), AdminError> {
        let mut state = self.load_operation(operation_id).await?;

        // Only allow cancelling in-progress or pending operations
        if state.status != OperationStatus::InProgress && state.status != OperationStatus::Pending {
            return Err(AdminError::InvalidOperation {
                message: format!("Cannot cancel operation with status {:?}", state.status),
            });
        }

        state.status = OperationStatus::Cancelled;
        state.updated_at = Utc::now();
        self.save_state(&state).await
    }

    /// Delete an operation state file
    pub async fn delete_operation(&self, operation_id: Uuid) -> Result<(), AdminError> {
        let path = self.operation_path(operation_id);
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
        }
        Ok(())
    }

    /// Save operation state to disk
    async fn save_state(&self, state: &OperationState) -> Result<(), AdminError> {
        let path = self.operation_path(state.operation_id);
        let content = serde_json::to_string_pretty(state)?;
        tokio::fs::write(&path, content).await?;
        Ok(())
    }
}

/// Count completed, failed, and skipped results
fn count_results(results: &HashMap<String, ProjectResultState>) -> (usize, usize, usize) {
    let mut completed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for result in results.values() {
        match &result.result {
            ItemResult::Success => completed += 1,
            ItemResult::Failed { .. } => failed += 1,
            ItemResult::Skipped { .. } => skipped += 1,
        }
    }

    (completed, failed, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_manager() -> (StateManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = StateManager::with_dir(temp_dir.path().to_path_buf()).unwrap();
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_create_and_load_operation() {
        let (manager, _temp_dir) = create_test_manager().await;

        let project_ids = vec!["proj-1".to_string(), "proj-2".to_string()];
        let params = serde_json::json!({"email": "user@example.com"});

        let operation_id = manager
            .create_operation(OperationType::AddUser, params.clone(), project_ids.clone())
            .await
            .unwrap();

        let state = manager.load_operation(operation_id).await.unwrap();

        assert_eq!(state.operation_id, operation_id);
        assert_eq!(state.operation_type, OperationType::AddUser);
        assert_eq!(state.status, OperationStatus::Pending);
        assert_eq!(state.project_ids.len(), 2);
    }

    #[tokio::test]
    async fn test_update_state() {
        let (manager, _temp_dir) = create_test_manager().await;

        let operation_id = manager
            .create_operation(
                OperationType::AddUser,
                serde_json::json!({}),
                vec!["proj-1".to_string()],
            )
            .await
            .unwrap();

        // Update with item completion
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

        let state = manager.load_operation(operation_id).await.unwrap();
        assert!(state.results.contains_key("proj-1"));
    }

    #[tokio::test]
    async fn test_get_pending_projects() {
        let (manager, _temp_dir) = create_test_manager().await;

        let operation_id = manager
            .create_operation(
                OperationType::AddUser,
                serde_json::json!({}),
                vec![
                    "proj-1".to_string(),
                    "proj-2".to_string(),
                    "proj-3".to_string(),
                ],
            )
            .await
            .unwrap();

        // Complete one project
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

        let state = manager.load_operation(operation_id).await.unwrap();
        let pending = manager.get_pending_projects(&state);

        assert_eq!(pending.len(), 2);
        assert!(pending.contains(&"proj-2".to_string()));
        assert!(pending.contains(&"proj-3".to_string()));
    }
}

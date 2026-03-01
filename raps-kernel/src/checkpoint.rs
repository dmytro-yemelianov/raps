// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Checkpoint store for resumable batch operations.
//!
//! Persists progress as JSON files so that interrupted batches
//! (uploads, translations, permission changes) can resume from
//! where they left off.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single checkpoint tracking progress of a batch workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Unique identifier for this workflow run.
    pub workflow_id: String,
    /// Type of workflow (e.g., "upload", "translate", "permissions").
    pub workflow_type: String,
    /// Hash of the input parameters (for finding resumable checkpoints).
    pub input_hash: String,
    /// Total number of work units.
    pub total_units: usize,
    /// Indices of completed units.
    pub completed: Vec<usize>,
    /// Failed units: (index, error message).
    pub failed: Vec<(usize, String)>,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 last update timestamp.
    pub updated_at: String,
}

impl Checkpoint {
    /// Create a new checkpoint for a workflow.
    pub fn new(workflow_id: String, workflow_type: String, input_hash: String, total_units: usize) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            workflow_id,
            workflow_type,
            input_hash,
            total_units,
            completed: Vec::new(),
            failed: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Mark a unit as completed.
    pub fn mark_completed(&mut self, index: usize) {
        if !self.completed.contains(&index) {
            self.completed.push(index);
            self.updated_at = chrono::Utc::now().to_rfc3339();
        }
    }

    /// Mark a unit as failed.
    pub fn mark_failed(&mut self, index: usize, error: String) {
        self.failed.push((index, error));
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Indices that still need processing.
    pub fn remaining(&self) -> Vec<usize> {
        let done: std::collections::HashSet<usize> = self.completed.iter().copied().collect();
        let failed: std::collections::HashSet<usize> = self.failed.iter().map(|(i, _)| *i).collect();
        (0..self.total_units)
            .filter(|i| !done.contains(i) && !failed.contains(i))
            .collect()
    }

    /// Whether the workflow is complete (all units processed).
    pub fn is_complete(&self) -> bool {
        self.completed.len() + self.failed.len() >= self.total_units
    }

    /// Progress as a fraction (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        if self.total_units == 0 {
            return 1.0;
        }
        (self.completed.len() + self.failed.len()) as f64 / self.total_units as f64
    }
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

/// Persistent checkpoint store backed by JSON files.
pub struct CheckpointStore {
    dir: PathBuf,
}

impl CheckpointStore {
    /// Create a new store at the given directory.
    pub fn new(dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create checkpoint dir: {}", dir.display()))?;
        Ok(Self { dir })
    }

    /// Default store location: `~/.local/share/raps/checkpoints/`
    pub fn default_dir() -> PathBuf {
        directories::ProjectDirs::from("", "", "raps")
            .map(|d| d.data_dir().join("checkpoints"))
            .unwrap_or_else(|| PathBuf::from(".raps/checkpoints"))
    }

    fn path_for(&self, workflow_id: &str) -> PathBuf {
        self.dir.join(format!("{}.json", workflow_id))
    }

    /// Save a checkpoint to disk.
    pub fn save(&self, cp: &Checkpoint) -> Result<()> {
        let path = self.path_for(&cp.workflow_id);
        let json = serde_json::to_string_pretty(cp)?;
        std::fs::write(&path, json)
            .with_context(|| format!("failed to write checkpoint: {}", path.display()))?;
        Ok(())
    }

    /// Load a checkpoint by workflow ID.
    pub fn load(&self, workflow_id: &str) -> Result<Option<Checkpoint>> {
        let path = self.path_for(workflow_id);
        if !path.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read checkpoint: {}", path.display()))?;
        let cp: Checkpoint = serde_json::from_str(&data)?;
        Ok(Some(cp))
    }

    /// Find a resumable checkpoint matching the workflow type and input hash.
    pub fn find_resumable(&self, workflow_type: &str, input_hash: &str) -> Result<Option<Checkpoint>> {
        let entries = std::fs::read_dir(&self.dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Ok(data) = std::fs::read_to_string(&path) {
                    if let Ok(cp) = serde_json::from_str::<Checkpoint>(&data) {
                        if cp.workflow_type == workflow_type
                            && cp.input_hash == input_hash
                            && !cp.is_complete()
                        {
                            return Ok(Some(cp));
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Remove a checkpoint (after successful completion).
    pub fn remove(&self, workflow_id: &str) -> Result<()> {
        let path = self.path_for(workflow_id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// List all checkpoints.
    pub fn list(&self) -> Result<Vec<Checkpoint>> {
        let mut results = Vec::new();
        if !self.dir.exists() {
            return Ok(results);
        }
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Ok(data) = std::fs::read_to_string(&path) {
                    if let Ok(cp) = serde_json::from_str::<Checkpoint>(&data) {
                        results.push(cp);
                    }
                }
            }
        }
        Ok(results)
    }

    /// Prune completed checkpoints older than the given age.
    pub fn prune(&self, max_age: std::time::Duration) -> Result<usize> {
        let cutoff = chrono::Utc::now() - chrono::Duration::from_std(max_age).unwrap_or_default();
        let mut removed = 0;
        for cp in self.list()? {
            if cp.is_complete() {
                if let Ok(updated) = chrono::DateTime::parse_from_rfc3339(&cp.updated_at) {
                    if updated < cutoff {
                        self.remove(&cp.workflow_id)?;
                        removed += 1;
                    }
                }
            }
        }
        Ok(removed)
    }
}

// ---------------------------------------------------------------------------
// Hash helper
// ---------------------------------------------------------------------------

/// Generate a deterministic hash for checkpoint input matching.
pub fn hash_input(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(input.as_bytes());
    format!("{:x}", hash)[..16].to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (tempfile::TempDir, CheckpointStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = CheckpointStore::new(dir.path().to_path_buf()).unwrap();
        (dir, store)
    }

    #[test]
    fn test_checkpoint_lifecycle() {
        let (_dir, store) = temp_store();
        let mut cp = Checkpoint::new(
            "wf-001".to_string(),
            "upload".to_string(),
            "abc123".to_string(),
            5,
        );
        assert_eq!(cp.remaining(), vec![0, 1, 2, 3, 4]);
        assert!(!cp.is_complete());

        cp.mark_completed(0);
        cp.mark_completed(2);
        cp.mark_failed(1, "network error".to_string());
        store.save(&cp).unwrap();

        let loaded = store.load("wf-001").unwrap().unwrap();
        assert_eq!(loaded.completed, vec![0, 2]);
        assert_eq!(loaded.failed.len(), 1);
        assert_eq!(loaded.remaining(), vec![3, 4]);
        assert!(!loaded.is_complete());
    }

    #[test]
    fn test_checkpoint_complete() {
        let mut cp = Checkpoint::new(
            "wf-002".to_string(),
            "translate".to_string(),
            "def456".to_string(),
            3,
        );
        cp.mark_completed(0);
        cp.mark_completed(1);
        cp.mark_completed(2);
        assert!(cp.is_complete());
        assert!(cp.remaining().is_empty());
        assert!((cp.progress() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_find_resumable() {
        let (_dir, store) = temp_store();

        // Complete checkpoint — should NOT be found
        let mut cp1 = Checkpoint::new("wf-a".to_string(), "upload".to_string(), "hash1".to_string(), 2);
        cp1.mark_completed(0);
        cp1.mark_completed(1);
        store.save(&cp1).unwrap();

        // Incomplete checkpoint — should be found
        let mut cp2 = Checkpoint::new("wf-b".to_string(), "upload".to_string(), "hash1".to_string(), 3);
        cp2.mark_completed(0);
        store.save(&cp2).unwrap();

        let found = store.find_resumable("upload", "hash1").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().workflow_id, "wf-b");

        // No match for different type
        assert!(store.find_resumable("translate", "hash1").unwrap().is_none());
    }

    #[test]
    fn test_remove() {
        let (_dir, store) = temp_store();
        let cp = Checkpoint::new("wf-del".to_string(), "test".to_string(), "h".to_string(), 1);
        store.save(&cp).unwrap();
        assert!(store.load("wf-del").unwrap().is_some());

        store.remove("wf-del").unwrap();
        assert!(store.load("wf-del").unwrap().is_none());
    }

    #[test]
    fn test_list() {
        let (_dir, store) = temp_store();
        store.save(&Checkpoint::new("a".to_string(), "t".to_string(), "h".to_string(), 1)).unwrap();
        store.save(&Checkpoint::new("b".to_string(), "t".to_string(), "h".to_string(), 1)).unwrap();
        assert_eq!(store.list().unwrap().len(), 2);
    }

    #[test]
    fn test_hash_input() {
        let h1 = hash_input("some input data");
        let h2 = hash_input("some input data");
        let h3 = hash_input("different data");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert_eq!(h1.len(), 16);
    }

    #[test]
    fn test_progress() {
        let mut cp = Checkpoint::new("p".to_string(), "t".to_string(), "h".to_string(), 4);
        assert!((cp.progress() - 0.0).abs() < f64::EPSILON);
        cp.mark_completed(0);
        cp.mark_failed(1, "err".to_string());
        assert!((cp.progress() - 0.5).abs() < f64::EPSILON);
    }
}

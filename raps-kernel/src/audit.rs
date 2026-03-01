// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Append-only JSONL audit logger.
//!
//! Records every significant operation (upload, translate, permission
//! change, etc.) to daily log files.  Supports configurable retention
//! with automatic pruning of old files.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Audit entry
// ---------------------------------------------------------------------------

/// A single auditable operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub operation: String,
    pub resource: String,
    pub result: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl AuditEntry {
    /// Create a new audit entry with the current timestamp.
    pub fn new(operation: &str, resource: &str, result: &str, duration_ms: u64) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            operation: operation.to_string(),
            resource: resource.to_string(),
            result: result.to_string(),
            duration_ms,
            user: None,
            details: None,
        }
    }

    /// Set the user field.
    pub fn with_user(mut self, user: &str) -> Self {
        self.user = Some(user.to_string());
        self
    }

    /// Set the details field.
    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }
}

// ---------------------------------------------------------------------------
// Audit logger
// ---------------------------------------------------------------------------

/// Audit logger that writes JSONL to daily files.
pub struct AuditLogger {
    dir: PathBuf,
    retention_days: u32,
}

impl AuditLogger {
    /// Create a new audit logger.
    pub fn new(dir: PathBuf, retention_days: u32) -> Result<Self> {
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create audit dir: {}", dir.display()))?;
        Ok(Self { dir, retention_days })
    }

    /// Default audit log directory: `~/.local/share/raps/audit/`
    pub fn default_dir() -> PathBuf {
        directories::ProjectDirs::from("", "", "raps")
            .map(|d| d.data_dir().join("audit"))
            .unwrap_or_else(|| PathBuf::from(".raps/audit"))
    }

    fn today_file(&self) -> PathBuf {
        let date = chrono::Utc::now().format("%Y-%m-%d");
        self.dir.join(format!("{date}.jsonl"))
    }

    /// Log an audit entry.
    pub fn log(&self, entry: &AuditEntry) -> Result<()> {
        let path = self.today_file();
        let line = serde_json::to_string(entry)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("failed to open audit log: {}", path.display()))?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    /// Prune audit files older than the retention period.
    pub fn prune(&self) -> Result<usize> {
        let cutoff = chrono::Utc::now()
            - chrono::Duration::days(self.retention_days as i64);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();
        let mut removed = 0;

        if !self.dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                // Files are named YYYY-MM-DD.jsonl
                if stem < cutoff_str.as_str()
                    && path.extension().is_some_and(|e| e == "jsonl")
                {
                    fs::remove_file(&path)?;
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }

    /// Read entries from a specific date.
    pub fn read_date(&self, date: &str) -> Result<Vec<AuditEntry>> {
        let path = self.dir.join(format!("{date}.jsonl"));
        if !path.exists() {
            return Ok(Vec::new());
        }
        let data = fs::read_to_string(&path)?;
        let mut entries = Vec::new();
        for line in data.lines() {
            if let Ok(entry) = serde_json::from_str::<AuditEntry>(line) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    /// List available audit dates.
    pub fn available_dates(&self) -> Result<Vec<String>> {
        let mut dates = Vec::new();
        if !self.dir.exists() {
            return Ok(dates);
        }
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                if entry.path().extension().is_some_and(|e| e == "jsonl") {
                    dates.push(stem.to_string());
                }
            }
        }
        dates.sort();
        Ok(dates)
    }
}

// ---------------------------------------------------------------------------
// Global convenience
// ---------------------------------------------------------------------------

static AUDIT_LOGGER: std::sync::OnceLock<AuditLogger> = std::sync::OnceLock::new();

/// Initialize the global audit logger.
pub fn init(retention_days: u32) -> Result<()> {
    let dir = AuditLogger::default_dir();
    let logger = AuditLogger::new(dir, retention_days)?;
    let _ = AUDIT_LOGGER.set(logger);
    Ok(())
}

/// Log an operation to the global audit logger (no-op if not initialized).
pub fn log_operation(entry: &AuditEntry) {
    if let Some(logger) = AUDIT_LOGGER.get() {
        if let Err(e) = logger.log(entry) {
            tracing::warn!("audit log failed: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_logger() -> (tempfile::TempDir, AuditLogger) {
        let dir = tempfile::tempdir().unwrap();
        let logger = AuditLogger::new(dir.path().to_path_buf(), 90).unwrap();
        (dir, logger)
    }

    #[test]
    fn test_log_and_read() {
        let (_dir, logger) = temp_logger();
        let entry = AuditEntry::new("upload", "bucket/file.rvt", "success", 1500);
        logger.log(&entry).unwrap();

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let entries = logger.read_date(&today).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].operation, "upload");
        assert_eq!(entries[0].resource, "bucket/file.rvt");
    }

    #[test]
    fn test_multiple_entries() {
        let (_dir, logger) = temp_logger();
        logger.log(&AuditEntry::new("upload", "a.rvt", "success", 100)).unwrap();
        logger.log(&AuditEntry::new("translate", "urn:xxx", "success", 5000)).unwrap();
        logger.log(&AuditEntry::new("download", "b.ifc", "error", 200)).unwrap();

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let entries = logger.read_date(&today).unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_entry_with_user_and_details() {
        let entry = AuditEntry::new("permission", "project/123", "success", 50)
            .with_user("user@example.com")
            .with_details("granted admin access");
        assert_eq!(entry.user.as_deref(), Some("user@example.com"));
        assert_eq!(entry.details.as_deref(), Some("granted admin access"));
    }

    #[test]
    fn test_available_dates() {
        let (_dir, logger) = temp_logger();
        logger.log(&AuditEntry::new("test", "resource", "ok", 0)).unwrap();
        let dates = logger.available_dates().unwrap();
        assert_eq!(dates.len(), 1);
    }

    #[test]
    fn test_read_nonexistent_date() {
        let (_dir, logger) = temp_logger();
        let entries = logger.read_date("2020-01-01").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_prune_old_files() {
        let (dir, logger) = temp_logger();

        // Create a fake old audit file
        let old_path = dir.path().join("2020-01-01.jsonl");
        fs::write(&old_path, "{\"timestamp\":\"2020-01-01\",\"operation\":\"test\",\"resource\":\"r\",\"result\":\"ok\",\"duration_ms\":0}\n").unwrap();

        let removed = logger.prune().unwrap();
        assert_eq!(removed, 1);
        assert!(!old_path.exists());
    }
}

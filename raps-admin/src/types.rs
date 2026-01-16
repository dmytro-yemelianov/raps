// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Core types for bulk admin operations

use serde::{Deserialize, Serialize};

/// Type of bulk operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    /// Add user to projects
    AddUser,
    /// Remove user from projects
    RemoveUser,
    /// Update user role across projects
    UpdateRole,
    /// Update folder permissions across projects
    UpdateFolderRights,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::AddUser => write!(f, "add_user"),
            OperationType::RemoveUser => write!(f, "remove_user"),
            OperationType::UpdateRole => write!(f, "update_role"),
            OperationType::UpdateFolderRights => write!(f, "update_folder_rights"),
        }
    }
}

/// Status of a bulk operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    /// Operation created but not started
    Pending,
    /// Operation currently running
    InProgress,
    /// Operation completed successfully
    Completed,
    /// Operation was cancelled by user
    Cancelled,
    /// Operation failed to complete
    Failed,
}

impl std::fmt::Display for OperationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationStatus::Pending => write!(f, "pending"),
            OperationStatus::InProgress => write!(f, "in_progress"),
            OperationStatus::Completed => write!(f, "completed"),
            OperationStatus::Cancelled => write!(f, "cancelled"),
            OperationStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Folder types for permission management
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FolderType {
    /// Project Files folder
    ProjectFiles,
    /// Plans folder
    Plans,
    /// Custom folder path
    Custom(String),
}

/// Permission levels for folder access
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    /// View only access
    ViewOnly,
    /// View and download
    ViewDownload,
    /// Upload only (publish)
    UploadOnly,
    /// View, download, and upload
    ViewDownloadUpload,
    /// View, download, upload, and edit
    ViewDownloadUploadEdit,
    /// Full folder control
    FolderControl,
}

impl PermissionLevel {
    /// Convert to API actions array
    pub fn to_actions(&self) -> Vec<&'static str> {
        match self {
            PermissionLevel::ViewOnly => vec!["VIEW", "COLLABORATE"],
            PermissionLevel::ViewDownload => vec!["VIEW", "DOWNLOAD", "COLLABORATE"],
            PermissionLevel::UploadOnly => vec!["PUBLISH"],
            PermissionLevel::ViewDownloadUpload => {
                vec!["PUBLISH", "VIEW", "DOWNLOAD", "COLLABORATE"]
            }
            PermissionLevel::ViewDownloadUploadEdit => {
                vec!["PUBLISH", "VIEW", "DOWNLOAD", "COLLABORATE", "EDIT"]
            }
            PermissionLevel::FolderControl => {
                vec![
                    "PUBLISH",
                    "VIEW",
                    "DOWNLOAD",
                    "COLLABORATE",
                    "EDIT",
                    "CONTROL",
                ]
            }
        }
    }
}

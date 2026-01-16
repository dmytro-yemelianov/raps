// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Account Admin Bulk Management
//!
//! This crate provides bulk user management capabilities for ACC/BIM 360 accounts:
//! - Add users to multiple projects in a single operation
//! - Remove users from multiple projects
//! - Update user roles across projects
//! - Manage folder-level permissions
//!
//! Features:
//! - Configurable concurrency with rate limit handling
//! - Progress tracking with callbacks
//! - State persistence for resumable operations
//! - Retry logic with exponential backoff

#![allow(clippy::uninlined_format_args)]

pub mod bulk;
pub mod error;
pub mod filter;
pub mod operations;
pub mod report;
pub mod types;

// Re-exports for convenience
pub use bulk::executor::{
    BulkConfig, BulkExecutor, BulkOperationResult, ItemDetail, ItemResult, ProcessItem,
    ProgressUpdate,
};
pub use bulk::state::{OperationState, OperationSummary, StateManager, StateUpdate};
pub use error::{AdminError, ExitCode};
pub use filter::ProjectFilter;
pub use operations::{
    BulkAddUserParams, BulkRemoveUserParams, BulkUpdateFolderRightsParams, BulkUpdateRoleParams,
    bulk_add_user, bulk_remove_user, bulk_update_folder_rights, bulk_update_role,
    resume_bulk_add_user, resume_bulk_remove_user, resume_bulk_update_folder_rights,
    resume_bulk_update_role,
};
pub use types::{FolderType, OperationStatus, OperationType, PermissionLevel};

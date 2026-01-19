// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! MCP Tool definitions and utilities
//!
//! This module contains additional tool implementations and helper utilities
//! for the RAPS MCP Server.

// Tool definitions are in server.rs.
// This module is reserved for additional utilities and extended tool implementations.

/// Available MCP tools in the RAPS server (v4.4 - 50 tools)
#[allow(dead_code)]
pub const TOOLS: &[&str] = &[
    // Authentication
    "auth_test",
    "auth_status",
    // OSS Buckets
    "bucket_list",
    "bucket_create",
    "bucket_get",
    "bucket_delete",
    // OSS Objects (basic)
    "object_list",
    "object_delete",
    "object_signed_url",
    "object_urn",
    // OSS Objects (v4.4 - upload/download/copy)
    "object_upload",
    "object_upload_batch",
    "object_download",
    "object_info",
    "object_copy",
    "object_delete_batch",
    // Model Derivative
    "translate_start",
    "translate_status",
    // Data Management
    "hub_list",
    "project_list",
    // Project Management (v4.4)
    "project_info",
    "project_users_list",
    "folder_contents",
    // Admin Bulk Operations (v4.0)
    "admin_project_list",
    "admin_user_add",
    "admin_user_remove",
    "admin_user_update_role",
    "admin_operation_list",
    "admin_operation_status",
    // ACC Project Admin (v4.4)
    "project_create",
    "project_user_add",
    "project_users_import",
    // Folder/Item Management
    "folder_list",
    "folder_create",
    "item_info",
    "item_versions",
    // Item Management (v4.4)
    "item_create",
    "item_delete",
    "item_rename",
    // Issues
    "issue_list",
    "issue_get",
    "issue_create",
    "issue_update",
    // RFIs
    "rfi_list",
    "rfi_get",
    // ACC Extended
    "acc_assets_list",
    "acc_submittals_list",
    "acc_checklists_list",
];

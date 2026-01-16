// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! MCP Tool definitions and utilities
//!
//! This module contains additional tool implementations and helper utilities
//! for the RAPS MCP Server.

// Tool definitions are in server.rs.
// This module is reserved for additional utilities and extended tool implementations.

/// Available MCP tools in the RAPS server (v4.0 - 35 tools)
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
    // OSS Objects
    "object_list",
    "object_delete",
    "object_signed_url",
    "object_urn",
    // Model Derivative
    "translate_start",
    "translate_status",
    // Data Management
    "hub_list",
    "project_list",
    // Admin Bulk Operations (v4.0)
    "admin_project_list",
    "admin_user_add",
    "admin_user_remove",
    "admin_user_update_role",
    "admin_operation_list",
    "admin_operation_status",
    // Folder/Item Management
    "folder_list",
    "folder_create",
    "item_info",
    "item_versions",
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

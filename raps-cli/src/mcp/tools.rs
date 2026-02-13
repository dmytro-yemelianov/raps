// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! MCP Tool definitions and utilities
//!
//! This module contains additional tool implementations and helper utilities
//! for the RAPS MCP Server.

// Tool definitions are in server.rs.
// This module is reserved for additional utilities and extended tool implementations.

/// Available MCP tools in the RAPS server (v4.5 - 72 tools)
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
    "hub_info",
    "project_list",
    // Project Management (v4.4)
    "project_info",
    "project_users_list",
    "folder_contents",
    // Admin Bulk Operations (v4.0+)
    "admin_project_list",
    "admin_user_add",
    "admin_user_remove",
    "admin_user_update_role",
    "admin_folder_rights",
    "admin_operation_list",
    "admin_operation_status",
    "admin_operation_resume",
    "admin_operation_cancel",
    // ACC Project Admin (v4.4)
    "project_create",
    "project_user_add",
    "project_users_import",
    // Template Management (v4.5)
    "template_list",
    "template_info",
    "template_create",
    "template_update",
    "template_archive",
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
    // Issue Comments
    "issue_comments_list",
    "issue_comment_add",
    "issue_comment_delete",
    // RFIs
    "rfi_list",
    "rfi_get",
    "rfi_create",
    "rfi_update",
    // ACC Extended
    "acc_assets_list",
    "asset_create",
    "asset_update",
    "asset_delete",
    "asset_get",
    "acc_submittals_list",
    "submittal_create",
    "submittal_update",
    "acc_checklists_list",
    "checklist_create",
    "checklist_update",
    "checklist_templates_list",
    // Custom API (v4.5)
    "api_request",
    // Admin User Listing (v4.6)
    "admin_user_list",
    // Portfolio Reports (v4.6)
    "report_rfi_summary",
    "report_issues_summary",
    // Webhooks (v4.6)
    "webhook_list",
    "webhook_create",
    "webhook_get",
    "webhook_update",
    "webhook_delete",
    "webhook_events",
    // Design Automation (v4.6)
    "da_engines_list",
    "da_appbundles_list",
    "da_activities_list",
    "da_workitem_create",
    "da_workitem_status",
    "da_workitems_list",
    // Reality Capture
    "reality_list",
    "reality_create",
    "reality_process",
    "reality_status",
    "reality_result",
    "reality_delete",
    "reality_formats",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_array_not_empty() {
        assert!(!TOOLS.is_empty());
        assert!(
            TOOLS.len() >= 70,
            "Expected at least 70 tools, found {}",
            TOOLS.len()
        );
    }

    #[test]
    fn test_tools_array_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for tool in TOOLS {
            assert!(seen.insert(tool), "Duplicate tool name: {}", tool);
        }
    }

    #[test]
    fn test_tools_array_naming_convention() {
        for tool in TOOLS {
            assert!(!tool.is_empty(), "Empty tool name found");
            assert!(!tool.contains(' '), "Tool name contains space: {}", tool);
            assert!(!tool.contains('-'), "Tool name contains dash: {}", tool);
            // Should be snake_case
            assert!(
                tool.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()),
                "Tool name not snake_case: {}",
                tool
            );
        }
    }

    #[test]
    fn test_essential_tools_present() {
        let tools: Vec<&&str> = TOOLS.iter().collect();
        // Auth
        assert!(tools.contains(&&"auth_test"));
        // Buckets
        assert!(tools.contains(&&"bucket_list"));
        // Webhooks
        assert!(tools.contains(&&"webhook_list"));
        assert!(tools.contains(&&"webhook_create"));
        assert!(tools.contains(&&"webhook_get"));
        assert!(tools.contains(&&"webhook_update"));
        // DA
        assert!(tools.contains(&&"da_engines_list"));
        assert!(tools.contains(&&"da_workitems_list"));
        // Reality
        assert!(tools.contains(&&"reality_create"));
        assert!(tools.contains(&&"reality_list"));
        // Issues
        assert!(tools.contains(&&"issue_list"));
        assert!(tools.contains(&&"issue_comments_list"));
        // Hub
        assert!(tools.contains(&&"hub_list"));
        assert!(tools.contains(&&"hub_info"));
        // Project users
        assert!(tools.contains(&&"project_users_list"));
        // Admin
        assert!(tools.contains(&&"admin_user_list"));
        // Checklist templates
        assert!(tools.contains(&&"checklist_templates_list"));
        // Asset
        assert!(tools.contains(&&"asset_get"));
    }
}

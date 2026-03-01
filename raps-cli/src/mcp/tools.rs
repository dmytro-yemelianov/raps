// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! MCP Tool definitions and utilities
//!
//! This module contains additional tool implementations and helper utilities
//! for the RAPS MCP Server.

// Tool definitions are in server.rs.
// This module is reserved for additional utilities and extended tool implementations.

/// Available MCP tools in the RAPS server — must stay in sync with `get_tools()` in server.rs
pub const TOOLS: &[&str] = &[
    // Authentication
    "auth_test",
    "auth_status",
    "auth_login",
    "auth_logout",
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
    // OSS Objects (upload/download/copy)
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
    // Project Management
    "project_info",
    "project_users_list",
    "folder_contents",
    // Admin Bulk Operations
    "admin_project_list",
    "admin_user_add",
    "admin_user_remove",
    "admin_user_update_role",
    "admin_folder_rights",
    "admin_operation_list",
    "admin_operation_status",
    "admin_operation_resume",
    "admin_operation_cancel",
    // ACC Project Admin
    "project_create",
    "project_update",
    "project_archive",
    "project_user_add",
    "project_user_remove",
    "project_user_update",
    "project_users_import",
    // Template Management
    "template_list",
    "template_info",
    "template_create",
    "template_update",
    "template_archive",
    "template_convert",
    // Folder/Item Management
    "folder_list",
    "folder_create",
    "item_info",
    "item_versions",
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
    // Custom API
    "api_request",
    // Admin User Listing
    "admin_user_list",
    // Portfolio Reports
    "report_rfi_summary",
    "report_issues_summary",
    // Webhooks
    "webhook_list",
    "webhook_create",
    "webhook_get",
    "webhook_update",
    "webhook_delete",
    "webhook_events",
    // Design Automation
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
    // Pipelines
    "pipeline_validate",
    "pipeline_dry_run",
    "pipeline_run",
    "pipeline_list_templates",
    // Compound Workflows
    "workflow_prepare_for_viewing",
    "workflow_analyze_model",
    "workflow_batch_translate",
    "swarm_status",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_array_count() {
        assert_eq!(
            TOOLS.len(),
            109,
            "TOOLS array has {} entries, expected 109 — sync with get_tools() in server.rs",
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
            assert!(
                tool.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()),
                "Tool name not snake_case: {}",
                tool
            );
        }
    }

    #[test]
    fn test_tools_count_matches_definitions() {
        let defined = crate::mcp::definitions::get_tools();
        assert_eq!(
            TOOLS.len(),
            defined.len(),
            "TOOLS constant ({}) out of sync with get_tools() ({}) in definitions.rs",
            TOOLS.len(),
            defined.len()
        );
    }

    #[test]
    fn test_tools_names_match_definitions() {
        let defined = crate::mcp::definitions::get_tools();
        let def_names: Vec<&str> = defined.iter().map(|t| t.name.as_ref()).collect();
        for tool_name in TOOLS {
            assert!(
                def_names.contains(tool_name),
                "TOOLS constant has '{}' but get_tools() does not",
                tool_name
            );
        }
        for def_name in &def_names {
            assert!(
                TOOLS.contains(def_name),
                "get_tools() has '{}' but TOOLS constant does not",
                def_name
            );
        }
    }

    #[test]
    fn test_all_definitions_have_descriptions() {
        let defined = crate::mcp::definitions::get_tools();
        for tool in &defined {
            assert!(
                !tool.description.as_deref().unwrap_or("").is_empty(),
                "Tool '{}' has empty description",
                tool.name
            );
        }
    }

    #[test]
    fn test_essential_tools_present() {
        let tools: Vec<&&str> = TOOLS.iter().collect();
        // Auth
        assert!(tools.contains(&&"auth_test"));
        assert!(tools.contains(&&"auth_login"));
        assert!(tools.contains(&&"auth_logout"));
        // Buckets
        assert!(tools.contains(&&"bucket_list"));
        // Webhooks
        assert!(tools.contains(&&"webhook_list"));
        // DA
        assert!(tools.contains(&&"da_engines_list"));
        // Reality
        assert!(tools.contains(&&"reality_create"));
        // Issues
        assert!(tools.contains(&&"issue_list"));
        // Hub
        assert!(tools.contains(&&"hub_list"));
        // Admin
        assert!(tools.contains(&&"admin_user_list"));
        // Project management
        assert!(tools.contains(&&"project_update"));
        assert!(tools.contains(&&"project_archive"));
        assert!(tools.contains(&&"template_convert"));
    }
}

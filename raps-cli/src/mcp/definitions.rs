// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//\! Tool definitions (get_tools) for the MCP server.

use std::sync::Arc;

use rmcp::model::Tool;
use serde_json::{Map, Value, json};

// Helper to create tool schema
pub(crate) fn schema(props: Value, required: &[&str]) -> Arc<Map<String, Value>> {
    let mut obj = Map::new();
    obj.insert("type".to_string(), json!("object"));
    obj.insert("properties".to_string(), props);
    obj.insert("required".to_string(), json!(required));
    Arc::new(obj)
}

// Tool definitions
pub(crate) fn get_tools() -> Vec<Tool> {
    vec![
        Tool::new(
            "auth_test",
            "Test 2-legged OAuth authentication with APS",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "auth_status",
            "Check authentication status (2-legged and 3-legged)",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "auth_login",
            "Get instructions for 3-legged OAuth login. Login requires browser interaction and must be done via CLI.",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "auth_logout",
            "Logout from 3-legged OAuth and clear stored tokens. WARNING: destructive — only call when the user explicitly requests logout.",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "bucket_list",
            "List OSS buckets. Buckets are containers for storing files.",
            schema(
                json!({
                    "region": {"type": "string", "description": "Filter by region: US or EMEA"},
                    "limit": {"type": "integer", "description": "Max buckets (default: 100)"}
                }),
                &[],
            ),
        ),
        Tool::new(
            "bucket_create",
            "Create a new OSS bucket. Keys must be globally unique, 3-128 chars.",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Unique bucket key"},
                    "policy": {"type": "string", "description": "transient (24h), temporary (30d), or persistent"},
                    "region": {"type": "string", "description": "US or EMEA (default: US)"}
                }),
                &["bucket_key"],
            ),
        ),
        Tool::new(
            "bucket_get",
            "Get detailed bucket information",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "The bucket key"}
                }),
                &["bucket_key"],
            ),
        ),
        Tool::new(
            "bucket_delete",
            "Delete an OSS bucket (must be empty)",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Bucket key to delete"}
                }),
                &["bucket_key"],
            ),
        ),
        Tool::new(
            "object_list",
            "List objects (files) in an OSS bucket",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "The bucket key"},
                    "limit": {"type": "integer", "description": "Max objects (default: 100)"}
                }),
                &["bucket_key"],
            ),
        ),
        Tool::new(
            "object_delete",
            "Delete an object from an OSS bucket",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "The bucket key"},
                    "object_key": {"type": "string", "description": "Object key (filename)"}
                }),
                &["bucket_key", "object_key"],
            ),
        ),
        Tool::new(
            "object_signed_url",
            "Generate pre-signed S3 URL for direct download",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "The bucket key"},
                    "object_key": {"type": "string", "description": "The object key"},
                    "minutes": {"type": "integer", "description": "Expiry (2-60 min, default: 10)"}
                }),
                &["bucket_key", "object_key"],
            ),
        ),
        Tool::new(
            "object_urn",
            "Get Base64-encoded URN for an object (used for translation)",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "The bucket key"},
                    "object_key": {"type": "string", "description": "The object key"}
                }),
                &["bucket_key", "object_key"],
            ),
        ),
        Tool::new(
            "translate_start",
            "Start CAD translation. Formats: svf2, obj, stl, step, iges, ifc",
            schema(
                json!({
                    "urn": {"type": "string", "description": "Base64-encoded URN"},
                    "format": {"type": "string", "description": "Output format (default: svf2)"}
                }),
                &["urn"],
            ),
        ),
        Tool::new(
            "translate_status",
            "Check translation status: pending, inprogress, success, failed",
            schema(
                json!({
                    "urn": {"type": "string", "description": "Base64-encoded URN"}
                }),
                &["urn"],
            ),
        ),
        Tool::new(
            "hub_list",
            "List accessible hubs (BIM 360/ACC). Requires 3-legged auth.",
            schema(
                json!({
                    "limit": {"type": "integer", "description": "Max hubs (default: 50)"}
                }),
                &[],
            ),
        ),
        Tool::new(
            "hub_info",
            "Get details of a specific hub (name, type, region). Requires 3-legged auth.",
            schema(
                json!({
                    "hub_id": {"type": "string", "description": "The hub ID"}
                }),
                &["hub_id"],
            ),
        ),
        Tool::new(
            "project_list",
            "List projects in a hub. Requires 3-legged auth.",
            schema(
                json!({
                    "hub_id": {"type": "string", "description": "The hub ID"},
                    "limit": {"type": "integer", "description": "Max projects (default: 50)"}
                }),
                &["hub_id"],
            ),
        ),
        // ================================================================
        // Admin Tools (v4.0 - Bulk Operations)
        // ================================================================
        Tool::new(
            "admin_project_list",
            "List projects in an ACC/BIM360 account with advanced filtering. Supports name patterns, status, platform, date ranges, and regions.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "filter": {"type": "string", "description": "Filter expression. Keys: name (glob with *), status (active|inactive|archived), platform (acc|bim360), created (>YYYY-MM-DD or <YYYY-MM-DD), region (us|emea). Example: 'name:*Hospital*,status:active,platform:acc,created:>2024-01-01'"},
                    "limit": {"type": "integer", "description": "Max projects (default: 100, max: 500)"}
                }),
                &["account_id"],
            ),
        ),
        Tool::new(
            "admin_user_add",
            "Bulk add a user to multiple projects across an account. Supports dry-run mode.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "email": {"type": "string", "description": "User email to add"},
                    "role": {"type": "string", "description": "Role: project_admin, viewer, editor, etc."},
                    "filter": {"type": "string", "description": "Project filter expression"},
                    "dry_run": {"type": "boolean", "description": "Preview without making changes (default: false)"}
                }),
                &["account_id", "email"],
            ),
        ),
        Tool::new(
            "admin_user_remove",
            "Bulk remove a user from multiple projects across an account. Supports dry-run mode.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "email": {"type": "string", "description": "User email to remove"},
                    "filter": {"type": "string", "description": "Project filter expression"},
                    "dry_run": {"type": "boolean", "description": "Preview without making changes (default: false)"}
                }),
                &["account_id", "email"],
            ),
        ),
        Tool::new(
            "admin_user_update_role",
            "Bulk update a user's role across multiple projects. Supports dry-run mode.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "email": {"type": "string", "description": "User email to update"},
                    "role": {"type": "string", "description": "New role: project_admin, viewer, editor, etc."},
                    "filter": {"type": "string", "description": "Project filter expression"},
                    "dry_run": {"type": "boolean", "description": "Preview without making changes (default: false)"}
                }),
                &["account_id", "email", "role"],
            ),
        ),
        Tool::new(
            "admin_operation_list",
            "List recent bulk admin operations for status tracking and resume.",
            schema(
                json!({
                    "limit": {"type": "integer", "description": "Max operations (default: 10)"}
                }),
                &[],
            ),
        ),
        Tool::new(
            "admin_operation_status",
            "Get detailed status of a bulk admin operation.",
            schema(
                json!({
                    "operation_id": {"type": "string", "description": "Operation ID (optional, defaults to most recent)"}
                }),
                &[],
            ),
        ),
        Tool::new(
            "admin_folder_rights",
            "Bulk update folder permissions for a user across multiple projects. Supports dry-run mode.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "email": {"type": "string", "description": "User email to update permissions for"},
                    "level": {"type": "string", "description": "Permission level: view_only, view_download, upload_only, view_download_upload, view_download_upload_edit, folder_control"},
                    "folder": {"type": "string", "description": "Folder type: project_files, plans, or custom folder ID"},
                    "filter": {"type": "string", "description": "Project filter expression"},
                    "dry_run": {"type": "boolean", "description": "Preview changes without applying (default: false)"}
                }),
                &["account_id", "email", "level"],
            ),
        ),
        Tool::new(
            "admin_operation_resume",
            "Resume an interrupted bulk admin operation from where it left off.",
            schema(
                json!({
                    "operation_id": {"type": "string", "description": "Operation ID to resume (optional, defaults to most recent in-progress)"}
                }),
                &[],
            ),
        ),
        Tool::new(
            "admin_operation_cancel",
            "Cancel an in-progress bulk admin operation.",
            schema(
                json!({
                    "operation_id": {"type": "string", "description": "Operation ID to cancel"}
                }),
                &["operation_id"],
            ),
        ),
        // ================================================================
        // Folder/Item Tools
        // ================================================================
        Tool::new(
            "folder_list",
            "List contents of a folder (files and subfolders). Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "folder_id": {"type": "string", "description": "The folder ID"}
                }),
                &["project_id", "folder_id"],
            ),
        ),
        Tool::new(
            "folder_create",
            "Create a new folder in a project. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "parent_folder_id": {"type": "string", "description": "Parent folder ID"},
                    "name": {"type": "string", "description": "New folder name"}
                }),
                &["project_id", "parent_folder_id", "name"],
            ),
        ),
        Tool::new(
            "item_info",
            "Get detailed information about a file/item. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "item_id": {"type": "string", "description": "The item ID"}
                }),
                &["project_id", "item_id"],
            ),
        ),
        Tool::new(
            "item_versions",
            "List all versions of a file/item. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "item_id": {"type": "string", "description": "The item ID"}
                }),
                &["project_id", "item_id"],
            ),
        ),
        // ================================================================
        // Issues Tools
        // ================================================================
        Tool::new(
            "issue_list",
            "List issues in an ACC/BIM360 project. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "filter": {"type": "string", "description": "Filter query (e.g., 'status=open')"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "issue_get",
            "Get detailed information about a specific issue.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "issue_id": {"type": "string", "description": "The issue ID"}
                }),
                &["project_id", "issue_id"],
            ),
        ),
        Tool::new(
            "issue_create",
            "Create a new issue in an ACC/BIM360 project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "title": {"type": "string", "description": "Issue title"},
                    "description": {"type": "string", "description": "Issue description"},
                    "status": {"type": "string", "description": "Status: open, pending, closed (default: open)"}
                }),
                &["project_id", "title"],
            ),
        ),
        Tool::new(
            "issue_update",
            "Update an existing issue.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "issue_id": {"type": "string", "description": "The issue ID"},
                    "title": {"type": "string", "description": "New title"},
                    "description": {"type": "string", "description": "New description"},
                    "status": {"type": "string", "description": "New status: open, pending, closed"}
                }),
                &["project_id", "issue_id"],
            ),
        ),
        // ================================================================
        // Issue Comment Tools
        // ================================================================
        Tool::new(
            "issue_comments_list",
            "List all comments on a specific issue. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "issue_id": {"type": "string", "description": "The issue ID"}
                }),
                &["project_id", "issue_id"],
            ),
        ),
        Tool::new(
            "issue_comment_add",
            "Add a comment to a specific issue.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "issue_id": {"type": "string", "description": "The issue ID"},
                    "body": {"type": "string", "description": "The comment text"}
                }),
                &["project_id", "issue_id", "body"],
            ),
        ),
        Tool::new(
            "issue_comment_delete",
            "Delete a comment from a specific issue.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "issue_id": {"type": "string", "description": "The issue ID"},
                    "comment_id": {"type": "string", "description": "The comment ID to delete"}
                }),
                &["project_id", "issue_id", "comment_id"],
            ),
        ),
        // ================================================================
        // RFI Tools
        // ================================================================
        Tool::new(
            "rfi_list",
            "List RFIs (Requests for Information) in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "rfi_get",
            "Get detailed information about a specific RFI.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "rfi_id": {"type": "string", "description": "The RFI ID"}
                }),
                &["project_id", "rfi_id"],
            ),
        ),
        Tool::new(
            "rfi_create",
            "Create a new RFI (Request for Information) in an ACC/BIM360 project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "title": {"type": "string", "description": "RFI title"},
                    "question": {"type": "string", "description": "RFI question text"},
                    "priority": {"type": "string", "description": "Priority: low, normal, high, critical (default: normal)"},
                    "due_date": {"type": "string", "description": "Due date in YYYY-MM-DD format"},
                    "assigned_to": {"type": "string", "description": "User ID to assign the RFI to"},
                    "location": {"type": "string", "description": "Location reference"}
                }),
                &["project_id", "title"],
            ),
        ),
        Tool::new(
            "rfi_update",
            "Update an existing RFI.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "rfi_id": {"type": "string", "description": "The RFI ID to update"},
                    "title": {"type": "string", "description": "New title"},
                    "question": {"type": "string", "description": "Updated question"},
                    "answer": {"type": "string", "description": "Answer to the RFI"},
                    "status": {"type": "string", "description": "New status: draft, open, answered, closed, void"},
                    "priority": {"type": "string", "description": "New priority: low, normal, high, critical"}
                }),
                &["project_id", "rfi_id"],
            ),
        ),
        // ================================================================
        // ACC Extended Tools
        // ================================================================
        Tool::new(
            "acc_assets_list",
            "List assets in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "asset_create",
            "Create a new asset in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "category_id": {"type": "string", "description": "Asset category ID"},
                    "description": {"type": "string", "description": "Asset description"},
                    "barcode": {"type": "string", "description": "Asset barcode"},
                    "client_asset_id": {"type": "string", "description": "Client-defined asset identifier"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "asset_update",
            "Update an existing asset.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "asset_id": {"type": "string", "description": "The asset ID to update"},
                    "category_id": {"type": "string", "description": "New category ID"},
                    "status_id": {"type": "string", "description": "New status ID"},
                    "description": {"type": "string", "description": "New description"},
                    "barcode": {"type": "string", "description": "New barcode"}
                }),
                &["project_id", "asset_id"],
            ),
        ),
        Tool::new(
            "asset_delete",
            "Delete an asset from a project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "asset_id": {"type": "string", "description": "The asset ID to delete"}
                }),
                &["project_id", "asset_id"],
            ),
        ),
        Tool::new(
            "asset_get",
            "Get details of a specific asset in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "asset_id": {"type": "string", "description": "The asset ID to retrieve"}
                }),
                &["project_id", "asset_id"],
            ),
        ),
        Tool::new(
            "acc_submittals_list",
            "List submittals in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "submittal_create",
            "Create a new submittal in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "title": {"type": "string", "description": "Submittal title"},
                    "spec_section": {"type": "string", "description": "Specification section reference"},
                    "due_date": {"type": "string", "description": "Due date in YYYY-MM-DD format"}
                }),
                &["project_id", "title"],
            ),
        ),
        Tool::new(
            "submittal_update",
            "Update an existing submittal.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "submittal_id": {"type": "string", "description": "The submittal ID to update"},
                    "title": {"type": "string", "description": "New title"},
                    "status": {"type": "string", "description": "New status"},
                    "due_date": {"type": "string", "description": "New due date"}
                }),
                &["project_id", "submittal_id"],
            ),
        ),
        Tool::new(
            "acc_checklists_list",
            "List checklists in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "checklist_create",
            "Create a new checklist in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "title": {"type": "string", "description": "Checklist title"},
                    "template_id": {"type": "string", "description": "Checklist template ID to use"},
                    "location": {"type": "string", "description": "Location reference"},
                    "due_date": {"type": "string", "description": "Due date in YYYY-MM-DD format"},
                    "assignee_id": {"type": "string", "description": "User ID to assign the checklist to"}
                }),
                &["project_id", "title"],
            ),
        ),
        Tool::new(
            "checklist_update",
            "Update an existing checklist.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"},
                    "checklist_id": {"type": "string", "description": "The checklist ID to update"},
                    "title": {"type": "string", "description": "New title"},
                    "status": {"type": "string", "description": "New status"},
                    "location": {"type": "string", "description": "New location"},
                    "due_date": {"type": "string", "description": "New due date"},
                    "assignee_id": {"type": "string", "description": "New assignee user ID"}
                }),
                &["project_id", "checklist_id"],
            ),
        ),
        Tool::new(
            "checklist_templates_list",
            "List checklist templates in an ACC project.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "The project ID"}
                }),
                &["project_id"],
            ),
        ),
        // ================================================================
        // Object Upload/Download Tools (v4.4)
        // ================================================================
        Tool::new(
            "object_upload",
            "Upload a file to an OSS bucket. Automatically uses chunked upload for files > 100MB.",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Target bucket key (3-128 chars, lowercase)"},
                    "file_path": {"type": "string", "description": "Absolute path to the file to upload"},
                    "object_key": {"type": "string", "description": "Optional object key (defaults to filename)"}
                }),
                &["bucket_key", "file_path"],
            ),
        ),
        Tool::new(
            "object_upload_batch",
            "Upload multiple files to an OSS bucket. Uses 4 parallel uploads. Returns summary with individual results.",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Target bucket key"},
                    "file_paths": {"type": "array", "items": {"type": "string"}, "description": "Array of absolute file paths to upload"}
                }),
                &["bucket_key", "file_paths"],
            ),
        ),
        Tool::new(
            "object_download",
            "Download an object from OSS to a local file path.",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Source bucket key"},
                    "object_key": {"type": "string", "description": "Object key to download"},
                    "output_path": {"type": "string", "description": "Local file path to save the downloaded file"}
                }),
                &["bucket_key", "object_key", "output_path"],
            ),
        ),
        Tool::new(
            "object_info",
            "Get detailed metadata for an object including size, content type, SHA1 hash, and timestamps.",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Bucket key"},
                    "object_key": {"type": "string", "description": "Object key"}
                }),
                &["bucket_key", "object_key"],
            ),
        ),
        Tool::new(
            "object_copy",
            "Copy an object from one bucket to another. If destination exists, returns existing object with warning (non-destructive).",
            schema(
                json!({
                    "source_bucket": {"type": "string", "description": "Source bucket key"},
                    "source_key": {"type": "string", "description": "Source object key"},
                    "dest_bucket": {"type": "string", "description": "Destination bucket key"},
                    "dest_key": {"type": "string", "description": "Destination object key (defaults to source key)"}
                }),
                &["source_bucket", "source_key", "dest_bucket"],
            ),
        ),
        Tool::new(
            "object_delete_batch",
            "Delete multiple objects from an OSS bucket. Returns summary with individual results.",
            schema(
                json!({
                    "bucket_key": {"type": "string", "description": "Bucket key"},
                    "object_keys": {"type": "array", "items": {"type": "string"}, "description": "Array of object keys to delete"}
                }),
                &["bucket_key", "object_keys"],
            ),
        ),
        // ================================================================
        // Project Management Tools (v4.4)
        // ================================================================
        Tool::new(
            "project_info",
            "Get project details including name, type, scopes, and top-level folders. Requires 3-legged auth.",
            schema(
                json!({
                    "hub_id": {"type": "string", "description": "Hub ID (e.g., b.abc123)"},
                    "project_id": {"type": "string", "description": "Project ID (e.g., b.project123)"}
                }),
                &["hub_id", "project_id"],
            ),
        ),
        Tool::new(
            "project_users_list",
            "List users with access to a project with pagination. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "limit": {"type": "integer", "description": "Max results per page (default: 50, max: 200)"},
                    "offset": {"type": "integer", "description": "Starting index for pagination"}
                }),
                &["project_id"],
            ),
        ),
        Tool::new(
            "folder_contents",
            "List all items and subfolders within a folder. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "folder_id": {"type": "string", "description": "Folder ID (URN format)"},
                    "limit": {"type": "integer", "description": "Max results per page (default: 50)"},
                    "offset": {"type": "integer", "description": "Starting index"}
                }),
                &["project_id", "folder_id"],
            ),
        ),
        // ================================================================
        // ACC Project Admin Tools (v4.4)
        // ================================================================
        Tool::new(
            "project_create",
            "Create a new ACC project from scratch or from a template. ACC only (not BIM 360). Polls until project is activated. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "name": {"type": "string", "description": "Project name"},
                    "template_project_id": {"type": "string", "description": "Optional template project ID to clone from"},
                    "products": {"type": "array", "items": {"type": "string"}, "description": "Products to enable (e.g., ['build', 'docs', 'model'])"}
                }),
                &["account_id", "name"],
            ),
        ),
        Tool::new(
            "project_user_add",
            "Add a user to an ACC project with optional role assignment. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "email": {"type": "string", "description": "User email address"},
                    "role_id": {"type": "string", "description": "Optional role ID to assign"}
                }),
                &["project_id", "email"],
            ),
        ),
        Tool::new(
            "project_users_import",
            "Import multiple users to an ACC project at once. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "users": {"type": "array", "items": {"type": "object", "properties": {"email": {"type": "string"}, "role_id": {"type": "string"}}, "required": ["email"]}, "description": "Array of users to import"}
                }),
                &["project_id", "users"],
            ),
        ),
        Tool::new(
            "project_update",
            "Update an ACC project's metadata (name, status, dates). Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "project_id": {"type": "string", "description": "Project ID to update"},
                    "name": {"type": "string", "description": "New project name"},
                    "status": {"type": "string", "description": "New status (active, archived, suspended)"},
                    "start_date": {"type": "string", "description": "Project start date (YYYY-MM-DD)"},
                    "end_date": {"type": "string", "description": "Project end date (YYYY-MM-DD)"}
                }),
                &["account_id", "project_id"],
            ),
        ),
        Tool::new(
            "project_archive",
            "Archive an ACC project (soft delete). Archived projects can be restored later. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "project_id": {"type": "string", "description": "Project ID to archive"}
                }),
                &["account_id", "project_id"],
            ),
        ),
        Tool::new(
            "project_user_remove",
            "Remove a user from an ACC project. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "user_id": {"type": "string", "description": "User ID to remove from project"}
                }),
                &["project_id", "user_id"],
            ),
        ),
        Tool::new(
            "project_user_update",
            "Update a user's role in an ACC project. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "user_id": {"type": "string", "description": "User ID to update"},
                    "role_id": {"type": "string", "description": "New role ID to assign"}
                }),
                &["project_id", "user_id"],
            ),
        ),
        // ================================================================
        // Template Management Tools (v4.5)
        // ================================================================
        Tool::new(
            "template_list",
            "List project templates in an ACC account. Templates are projects with classification='template' that can be used as blueprints. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "limit": {"type": "integer", "description": "Maximum results (default: 100, max: 200)"}
                }),
                &["account_id"],
            ),
        ),
        Tool::new(
            "template_info",
            "Get details of a project template including name, status, products, and member counts. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "template_id": {"type": "string", "description": "Template (project) ID"}
                }),
                &["account_id", "template_id"],
            ),
        ),
        Tool::new(
            "template_create",
            "Create a new project template. Templates can be used as blueprints when creating new projects via project_create. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "name": {"type": "string", "description": "Template name"},
                    "products": {"type": "array", "items": {"type": "string"}, "description": "Products to enable (e.g., ['build', 'docs', 'model'])"}
                }),
                &["account_id", "name"],
            ),
        ),
        Tool::new(
            "template_update",
            "Update a template's name or status. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "template_id": {"type": "string", "description": "Template (project) ID"},
                    "name": {"type": "string", "description": "New template name"},
                    "status": {"type": "string", "description": "New status (active, archived, suspended)"}
                }),
                &["account_id", "template_id"],
            ),
        ),
        Tool::new(
            "template_archive",
            "Archive a template (soft delete). Archived templates cannot be used for new projects. Requires 3-legged auth.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "template_id": {"type": "string", "description": "Template (project) ID to archive"}
                }),
                &["account_id", "template_id"],
            ),
        ),
        Tool::new(
            "template_convert",
            "Convert a production project to a template. Note: ACC API may not support this operation - use template_create instead.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "ACC account ID"},
                    "project_id": {"type": "string", "description": "Production project ID to convert"}
                }),
                &["account_id", "project_id"],
            ),
        ),
        // ================================================================
        // Item Management Tools (v4.4)
        // ================================================================
        Tool::new(
            "item_create",
            "Create a new item in a project folder by linking an OSS storage object. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "folder_id": {"type": "string", "description": "Target folder ID (URN format)"},
                    "display_name": {"type": "string", "description": "Display name for the item"},
                    "storage_id": {"type": "string", "description": "OSS storage object URN"}
                }),
                &["project_id", "folder_id", "display_name", "storage_id"],
            ),
        ),
        Tool::new(
            "item_delete",
            "Delete an item from a project folder. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "item_id": {"type": "string", "description": "Item ID to delete"}
                }),
                &["project_id", "item_id"],
            ),
        ),
        Tool::new(
            "item_rename",
            "Update an item's display name. Requires 3-legged auth.",
            schema(
                json!({
                    "project_id": {"type": "string", "description": "Project ID"},
                    "item_id": {"type": "string", "description": "Item ID"},
                    "new_name": {"type": "string", "description": "New display name"}
                }),
                &["project_id", "item_id", "new_name"],
            ),
        ),
        // ================================================================
        // Webhooks (v4.6)
        // ================================================================
        Tool::new(
            "webhook_list",
            "List all registered webhooks across all systems and events.",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "webhook_create",
            "Create a new webhook subscription for a specific system and event.",
            schema(
                json!({
                    "system": {"type": "string", "description": "System name (e.g., 'data', 'derivative')"},
                    "event": {"type": "string", "description": "Event name (e.g., 'dm.version.added', 'extraction.finished')"},
                    "callback_url": {"type": "string", "description": "URL to receive webhook callbacks"},
                    "folder_urn": {"type": "string", "description": "Optional: restrict to a specific folder URN"}
                }),
                &["system", "event", "callback_url"],
            ),
        ),
        Tool::new(
            "webhook_delete",
            "Delete a webhook subscription.",
            schema(
                json!({
                    "system": {"type": "string", "description": "System name"},
                    "event": {"type": "string", "description": "Event name"},
                    "hook_id": {"type": "string", "description": "Webhook ID to delete"}
                }),
                &["system", "event", "hook_id"],
            ),
        ),
        Tool::new(
            "webhook_events",
            "List all available webhook event types that can be subscribed to.",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "webhook_get",
            "Get details of a specific webhook subscription.",
            schema(
                json!({
                    "system": {"type": "string", "description": "System name (e.g., 'data', 'derivative')"},
                    "event": {"type": "string", "description": "Event name (e.g., 'dm.version.added')"},
                    "hook_id": {"type": "string", "description": "Webhook ID"}
                }),
                &["system", "event", "hook_id"],
            ),
        ),
        Tool::new(
            "webhook_update",
            "Update a webhook subscription (callback URL, status, or filter).",
            schema(
                json!({
                    "system": {"type": "string", "description": "System name (e.g., 'data', 'derivative')"},
                    "event": {"type": "string", "description": "Event name (e.g., 'dm.version.added')"},
                    "hook_id": {"type": "string", "description": "Webhook ID to update"},
                    "callback_url": {"type": "string", "description": "New callback URL"},
                    "status": {"type": "string", "description": "New status ('active' or 'inactive')"},
                    "filter": {"type": "string", "description": "New filter expression"}
                }),
                &["system", "event", "hook_id"],
            ),
        ),
        // ================================================================
        // Design Automation (v4.6)
        // ================================================================
        Tool::new(
            "da_engines_list",
            "List all available Design Automation engines (AutoCAD, Revit, Inventor, 3dsMax).",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "da_appbundles_list",
            "List all registered Design Automation appbundles (custom plugins).",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "da_activities_list",
            "List all registered Design Automation activities (processing recipes).",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "da_workitem_create",
            "Create and submit a new Design Automation workitem. Requires activity_id and arguments mapping input/output names to URLs.",
            schema(
                json!({
                    "activity_id": {"type": "string", "description": "Fully qualified activity ID (e.g., 'YourApp.ActivityName+alias')"},
                    "arguments": {"type": "object", "description": "Input/output argument mappings. Each value is either a URL string or {\"url\": \"...\", \"verb\": \"get|put\"}"}
                }),
                &["activity_id", "arguments"],
            ),
        ),
        Tool::new(
            "da_workitem_status",
            "Check the status and progress of a Design Automation workitem.",
            schema(
                json!({
                    "workitem_id": {"type": "string", "description": "Workitem ID to check status for"}
                }),
                &["workitem_id"],
            ),
        ),
        Tool::new(
            "da_workitems_list",
            "List all Design Automation workitems with their status and progress.",
            schema(json!({}), &[]),
        ),
        // ================================================================
        // Reality Capture
        // ================================================================
        Tool::new(
            "reality_list",
            "List all photoscenes for reality capture. Returns ID, name, type, status, and progress for each photoscene.",
            schema(json!({}), &[]),
        ),
        Tool::new(
            "reality_create",
            "Create a new photoscene for reality capture (photogrammetry). Use to set up a new 3D reconstruction job from photos.",
            schema(
                json!({
                    "name": {"type": "string", "description": "Name for the new photoscene"},
                    "scene_type": {"type": "string", "description": "Scene type: 'aerial' or 'object' (default: object)"},
                    "format": {"type": "string", "description": "Output format: rcm, rcs, obj, fbx, ortho (default: rcm)"}
                }),
                &["name"],
            ),
        ),
        Tool::new(
            "reality_process",
            "Start processing a photoscene. Call after uploading photos to begin 3D reconstruction.",
            schema(
                json!({
                    "photoscene_id": {"type": "string", "description": "The photoscene ID to start processing"}
                }),
                &["photoscene_id"],
            ),
        ),
        Tool::new(
            "reality_status",
            "Check photoscene processing progress. Returns percentage complete and status message.",
            schema(
                json!({
                    "photoscene_id": {"type": "string", "description": "The photoscene ID to check progress for"}
                }),
                &["photoscene_id"],
            ),
        ),
        Tool::new(
            "reality_result",
            "Get download link for a processed photoscene. Returns the scene link and file size when processing is complete.",
            schema(
                json!({
                    "photoscene_id": {"type": "string", "description": "The photoscene ID to get results for"},
                    "format": {"type": "string", "description": "Output format: rcm, rcs, obj, fbx, ortho (default: rcm)"}
                }),
                &["photoscene_id"],
            ),
        ),
        Tool::new(
            "reality_delete",
            "Delete a photoscene and its associated data.",
            schema(
                json!({
                    "photoscene_id": {"type": "string", "description": "The photoscene ID to delete"}
                }),
                &["photoscene_id"],
            ),
        ),
        Tool::new(
            "reality_formats",
            "List all available output formats for reality capture photoscenes.",
            schema(json!({}), &[]),
        ),
        // ================================================================
        // Custom API Requests (v4.5)
        // ================================================================
        Tool::new(
            "api_request",
            "Execute custom HTTP request to APS API endpoints using current authentication. \
             Only APS domains are allowed (developer.api.autodesk.com, acc.autodesk.com, etc.). \
             Use for API endpoints not covered by other tools.",
            schema(
                json!({
                    "method": {"type": "string", "description": "HTTP method: GET, POST, PUT, PATCH, DELETE"},
                    "endpoint": {"type": "string", "description": "API endpoint path (e.g., /oss/v2/buckets) or full URL"},
                    "query": {"type": "object", "description": "Optional query parameters as key-value pairs"},
                    "headers": {"type": "object", "description": "Optional custom headers (cannot override Authorization)"},
                    "body": {"type": "object", "description": "Optional JSON request body (POST, PUT, PATCH only)"}
                }),
                &["method", "endpoint"],
            ),
        ),
        // ================================================================
        // Admin User Listing (v4.6)
        // ================================================================
        Tool::new(
            "admin_user_list",
            "List users in an ACC/BIM360 account or project. Returns user details including email, name, role, status, and company.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "project_id": {"type": "string", "description": "Optional: list users for a specific project only"},
                    "search": {"type": "string", "description": "Search by email or name"},
                    "role": {"type": "string", "description": "Filter by role name"},
                    "status": {"type": "string", "description": "Filter by status (active, inactive, not_invited)"}
                }),
                &["account_id"],
            ),
        ),
        // ================================================================
        // Portfolio Reports (v4.6)
        // ================================================================
        Tool::new(
            "report_rfi_summary",
            "Generate an RFI summary report across all projects in an account. Shows total, open, answered, and closed RFI counts per project.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "filter": {"type": "string", "description": "Project filter expression (e.g., 'status:active,name:*Hospital*')"},
                    "status": {"type": "string", "description": "Filter RFIs by status (open, answered, closed, void)"}
                }),
                &["account_id"],
            ),
        ),
        Tool::new(
            "report_issues_summary",
            "Generate an issues summary report across all projects in an account. Shows total, open, and closed issue counts per project.",
            schema(
                json!({
                    "account_id": {"type": "string", "description": "The ACC/BIM360 account ID"},
                    "filter": {"type": "string", "description": "Project filter expression (e.g., 'status:active')"},
                    "status": {"type": "string", "description": "Filter issues by status (open, closed)"}
                }),
                &["account_id"],
            ),
        ),
        // ================================================================
        // Pipeline v2 Tools
        // ================================================================
        Tool::new(
            "pipeline_validate",
            "Validate a RAPS pipeline YAML/JSON file for syntax and structural errors.",
            schema(
                json!({
                    "file": {"type": "string", "description": "Path to the pipeline file (YAML or JSON)"}
                }),
                &["file"],
            ),
        ),
        Tool::new(
            "pipeline_dry_run",
            "Preview a RAPS pipeline execution without running any commands. Shows each step that would execute.",
            schema(
                json!({
                    "file": {"type": "string", "description": "Path to the pipeline file (YAML or JSON)"},
                    "variables": {"type": "string", "description": "Comma-separated key=value pairs for variable substitution"}
                }),
                &["file"],
            ),
        ),
        Tool::new(
            "pipeline_run",
            "Execute a RAPS pipeline file. Runs each step sequentially (or in parallel for parallel steps). Returns execution summary.",
            schema(
                json!({
                    "file": {"type": "string", "description": "Path to the pipeline file (YAML or JSON)"},
                    "dry_run": {"type": "boolean", "description": "Preview without executing (default: false)"},
                    "ignore_failure": {"type": "boolean", "description": "Continue on step failure (default: false)"},
                    "variables": {"type": "string", "description": "Comma-separated key=value pairs for variable substitution"}
                }),
                &["file"],
            ),
        ),
        Tool::new(
            "pipeline_list_templates",
            "List available pipeline template files. Searches current directory and common locations for .yaml/.yml/.json pipeline files.",
            schema(
                json!({
                    "directory": {"type": "string", "description": "Directory to search (default: current directory)"}
                }),
                &[],
            ),
        ),
        // ─── Compound Workflows ─────────────────────────────────
        Tool::new(
            "workflow_prepare_for_viewing",
            "Upload a file and start SVF2 translation in one step. Returns URN and translation status. Use translate_status to poll for completion.",
            schema(
                json!({
                    "file_path": {"type": "string", "description": "Local file path to upload (e.g. /path/to/model.rvt)"},
                    "bucket_key": {"type": "string", "description": "Target bucket (default: raps-workflow-temp)"},
                    "object_key": {"type": "string", "description": "Object key in bucket (default: filename)"}
                }),
                &["file_path"],
            ),
        ),
        Tool::new(
            "workflow_analyze_model",
            "Get comprehensive analysis of a translated model: translation status, metadata, and properties in one call.",
            schema(
                json!({
                    "urn": {"type": "string", "description": "Base64-encoded URN of the translated model"},
                    "region": {"type": "string", "description": "Region: US or EMEA (default: US)"}
                }),
                &["urn"],
            ),
        ),
        Tool::new(
            "workflow_batch_translate",
            "Start translation for multiple URNs at once. Returns status for each. Pass comma-separated URNs.",
            schema(
                json!({
                    "urns": {"type": "string", "description": "Comma-separated list of base64-encoded URNs"},
                    "output_format": {"type": "string", "description": "Output format: svf, svf2, thumbnail, stl, obj (default: svf2)"},
                    "region": {"type": "string", "description": "Region: US or EMEA (default: US)"}
                }),
                &["urns"],
            ),
        ),
        Tool::new(
            "workflow_compare_versions",
            "Compare two model versions by checking translation status for both URNs. Reports readiness for visual comparison in Autodesk Viewer.",
            schema(
                json!({
                    "urn_a": {"type": "string", "description": "Base64-encoded URN of the first model version"},
                    "urn_b": {"type": "string", "description": "Base64-encoded URN of the second model version"},
                    "label_a": {"type": "string", "description": "Label for version A (default: 'Version A')"},
                    "label_b": {"type": "string", "description": "Label for version B (default: 'Version B')"}
                }),
                &["urn_a", "urn_b"],
            ),
        ),
        Tool::new(
            "workflow_setup_project",
            "Set up a new project workspace: creates a persistent bucket for file storage. Returns bucket key and next steps for uploading and translating models.",
            schema(
                json!({
                    "project_name": {"type": "string", "description": "Project name (will be normalized to a valid bucket key)"},
                    "bucket_region": {"type": "string", "description": "Bucket region: US or EMEA (default: US)"}
                }),
                &["project_name"],
            ),
        ),
        Tool::new(
            "swarm_status",
            "Get swarm orchestration health: circuit breaker states, rate limit budgets, and response cache stats. Useful for diagnosing API connectivity issues.",
            schema(
                json!({}),
                &[],
            ),
        ),
    ]
}

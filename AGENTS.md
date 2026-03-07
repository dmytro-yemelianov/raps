# RAPS MCP Tool Reference

> Auto-generated from source — do not edit manually.
> Regenerate: `raps docs mcp --write`

This file describes every tool exposed by the RAPS MCP server.

## Authentication

| Type | When Required |
|---|---|
| 2-legged (client credentials) | OSS, Model Derivative, Admin bulk ops |
| 3-legged (user authorization) | Data Management, ACC (issues, RFIs, assets) |

Set `APS_CLIENT_ID` and `APS_CLIENT_SECRET` before starting the MCP server.
For 3-legged auth in headless environments: `raps auth login --device`

## Tools

| Tool | Auth | Description |
|---|---|---|
| `auth_test` | either | Test 2-legged OAuth authentication with APS |
| `auth_status` | either | Check authentication status (2-legged and 3-legged) |
| `auth_login` | either | Get instructions for 3-legged OAuth login. Login requires browser interaction and must be done via CLI. |
| `auth_logout` | either | Logout from 3-legged OAuth and clear stored tokens. WARNING: destructive — only call when the user explicitly requests logout. |
| `bucket_list` | 2-leg | List OSS buckets. Buckets are containers for storing files. |
| `bucket_create` | 2-leg | Create a new OSS bucket. Keys must be globally unique, 3-128 chars. |
| `bucket_get` | 2-leg | Get detailed bucket information |
| `bucket_delete` | 2-leg | Delete an OSS bucket (must be empty) |
| `object_list` | 2-leg | List objects (files) in an OSS bucket |
| `object_delete` | 2-leg | Delete an object from an OSS bucket |
| `object_signed_url` | 2-leg | Generate pre-signed S3 URL for direct download |
| `object_urn` | 2-leg | Get Base64-encoded URN for an object (used for translation) |
| `translate_start` | 2-leg | Start CAD translation. Formats: svf2, obj, stl, step, iges, ifc |
| `translate_status` | 2-leg | Check translation status: pending, inprogress, success, failed |
| `hub_list` | 3-leg | List accessible hubs (BIM 360/ACC). Requires 3-legged auth. |
| `hub_info` | 2-leg | Get details of a specific hub (name, type, region). Requires 3-legged auth. |
| `project_list` | 3-leg | List projects in a hub. Requires 3-legged auth. |
| `admin_project_list` | 2-leg | List projects in an ACC/BIM360 account with advanced filtering. Supports name patterns, status, platform, date ranges, and regions. |
| `admin_user_add` | 2-leg | Bulk add a user to multiple projects across an account. Supports dry-run mode. |
| `admin_user_remove` | 2-leg | Bulk remove a user from multiple projects across an account. Supports dry-run mode. |
| `admin_user_update_role` | 2-leg | Bulk update a user's role across multiple projects. Supports dry-run mode. |
| `admin_operation_list` | 2-leg | List recent bulk admin operations for status tracking and resume. |
| `admin_operation_status` | 2-leg | Get detailed status of a bulk admin operation. |
| `admin_folder_rights` | 2-leg | Bulk update folder permissions for a user across multiple projects. Supports dry-run mode. |
| `admin_operation_resume` | 2-leg | Resume an interrupted bulk admin operation from where it left off. |
| `admin_operation_cancel` | 2-leg | Cancel an in-progress bulk admin operation. |
| `folder_list` | 3-leg | List contents of a folder (files and subfolders). Requires 3-legged auth. |
| `folder_create` | 3-leg | Create a new folder in a project. Requires 3-legged auth. |
| `item_info` | 3-leg | Get detailed information about a file/item. Requires 3-legged auth. |
| `item_versions` | 3-leg | List all versions of a file/item. Requires 3-legged auth. |
| `issue_list` | 3-leg | List issues in an ACC/BIM360 project. Requires 3-legged auth. |
| `issue_get` | 3-leg | Get detailed information about a specific issue. |
| `issue_create` | 3-leg | Create a new issue in an ACC/BIM360 project. |
| `issue_update` | 3-leg | Update an existing issue. |
| `issue_comments_list` | 2-leg | List all comments on a specific issue. Requires 3-legged auth. |
| `issue_comment_add` | 2-leg | Add a comment to a specific issue. |
| `issue_comment_delete` | 2-leg | Delete a comment from a specific issue. |
| `rfi_list` | 3-leg | List RFIs (Requests for Information) in an ACC project. |
| `rfi_get` | 3-leg | Get detailed information about a specific RFI. |
| `rfi_create` | 3-leg | Create a new RFI (Request for Information) in an ACC/BIM360 project. |
| `rfi_update` | 3-leg | Update an existing RFI. |
| `acc_assets_list` | 3-leg | List assets in an ACC project. |
| `asset_create` | 3-leg | Create a new asset in an ACC project. |
| `asset_update` | 3-leg | Update an existing asset. |
| `asset_delete` | 3-leg | Delete an asset from a project. |
| `asset_get` | 2-leg | Get details of a specific asset in an ACC project. |
| `acc_submittals_list` | 3-leg | List submittals in an ACC project. |
| `submittal_create` | 3-leg | Create a new submittal in an ACC project. |
| `submittal_update` | 3-leg | Update an existing submittal. |
| `acc_checklists_list` | 3-leg | List checklists in an ACC project. |
| `checklist_create` | 3-leg | Create a new checklist in an ACC project. |
| `checklist_update` | 3-leg | Update an existing checklist. |
| `checklist_templates_list` | 2-leg | List checklist templates in an ACC project. |
| `object_upload` | 2-leg | Upload a file to an OSS bucket. Automatically uses chunked upload for files > 100MB. |
| `object_upload_batch` | 2-leg | Upload multiple files to an OSS bucket. Uses 4 parallel uploads. Returns summary with individual results. |
| `object_download` | 2-leg | Download an object from OSS to a local file path. |
| `object_info` | 2-leg | Get detailed metadata for an object including size, content type, SHA1 hash, and timestamps. |
| `object_copy` | 2-leg | Copy an object from one bucket to another. If destination exists, returns existing object with warning (non-destructive). |
| `object_delete_batch` | 2-leg | Delete multiple objects from an OSS bucket. Returns summary with individual results. |
| `project_info` | 3-leg | Get project details including name, type, scopes, and top-level folders. Requires 3-legged auth. |
| `project_users_list` | 3-leg | List users with access to a project with pagination. Requires 3-legged auth. |
| `folder_contents` | 3-leg | List all items and subfolders within a folder. Requires 3-legged auth. |
| `project_create` | 3-leg | Create a new ACC project from scratch or from a template. ACC only (not BIM 360). Polls until project is activated. Requires 3-legged auth. |
| `project_user_add` | 3-leg | Add a user to an ACC project with optional role assignment. Requires 3-legged auth. |
| `project_users_import` | 3-leg | Import multiple users to an ACC project at once. Requires 3-legged auth. |
| `project_update` | 2-leg | Update an ACC project's metadata (name, status, dates). Requires 3-legged auth. |
| `project_archive` | 2-leg | Archive an ACC project (soft delete). Archived projects can be restored later. Requires 3-legged auth. |
| `project_user_remove` | 2-leg | Remove a user from an ACC project. Requires 3-legged auth. |
| `project_user_update` | 2-leg | Update a user's role in an ACC project. Requires 3-legged auth. |
| `template_list` | 2-leg | List project templates in an ACC account. Templates are projects with classification='template' that can be used as blueprints. Requires 3-legged auth. |
| `template_info` | 2-leg | Get details of a project template including name, status, products, and member counts. Requires 3-legged auth. |
| `template_create` | 2-leg | Create a new project template. Templates can be used as blueprints when creating new projects via project_create. Requires 3-legged auth. |
| `template_update` | 2-leg | Update a template's name or status. Requires 3-legged auth. |
| `template_archive` | 2-leg | Archive a template (soft delete). Archived templates cannot be used for new projects. Requires 3-legged auth. |
| `template_convert` | 2-leg | Convert a production project to a template. Note: ACC API may not support this operation - use template_create instead. |
| `item_create` | 3-leg | Create a new item in a project folder by linking an OSS storage object. Requires 3-legged auth. |
| `item_delete` | 3-leg | Delete an item from a project folder. Requires 3-legged auth. |
| `item_rename` | 3-leg | Update an item's display name. Requires 3-legged auth. |
| `webhook_list` | 2-leg | List all registered webhooks across all systems and events. |
| `webhook_create` | 2-leg | Create a new webhook subscription for a specific system and event. |
| `webhook_delete` | 2-leg | Delete a webhook subscription. |
| `webhook_events` | 2-leg | List all available webhook event types that can be subscribed to. |
| `webhook_get` | 2-leg | Get details of a specific webhook subscription. |
| `webhook_update` | 2-leg | Update a webhook subscription (callback URL, status, or filter). |
| `da_engines_list` | 2-leg | List all available Design Automation engines (AutoCAD, Revit, Inventor, 3dsMax). |
| `da_appbundles_list` | 2-leg | List all registered Design Automation appbundles (custom plugins). |
| `da_activities_list` | 2-leg | List all registered Design Automation activities (processing recipes). |
| `da_workitem_create` | 2-leg | Create and submit a new Design Automation workitem. Requires activity_id and arguments mapping input/output names to URLs. |
| `da_workitem_status` | 2-leg | Check the status and progress of a Design Automation workitem. |
| `da_workitems_list` | 2-leg | List all Design Automation workitems with their status and progress. |
| `reality_list` | 2-leg | List all photoscenes for reality capture. Returns ID, name, type, status, and progress for each photoscene. |
| `reality_create` | 2-leg | Create a new photoscene for reality capture (photogrammetry). Use to set up a new 3D reconstruction job from photos. |
| `reality_process` | 2-leg | Start processing a photoscene. Call after uploading photos to begin 3D reconstruction. |
| `reality_status` | 2-leg | Check photoscene processing progress. Returns percentage complete and status message. |
| `reality_result` | 2-leg | Get download link for a processed photoscene. Returns the scene link and file size when processing is complete. |
| `reality_delete` | 2-leg | Delete a photoscene and its associated data. |
| `reality_formats` | 2-leg | List all available output formats for reality capture photoscenes. |
| `api_request` | 2-leg | Execute custom HTTP request to APS API endpoints using current authentication. Only APS domains are allowed (developer.api.autodesk.com, acc.autodesk.com, etc.). Use for API endpoints not covered by other tools. |
| `admin_user_list` | 2-leg | List users in an ACC/BIM360 account or project. Returns user details including email, name, role, status, and company. |
| `report_rfi_summary` | 2-leg | Generate an RFI summary report across all projects in an account. Shows total, open, answered, and closed RFI counts per project. |
| `report_issues_summary` | 2-leg | Generate an issues summary report across all projects in an account. Shows total, open, and closed issue counts per project. |
| `pipeline_validate` | 2-leg | Validate a RAPS pipeline YAML/JSON file for syntax and structural errors. |
| `pipeline_dry_run` | 2-leg | Preview a RAPS pipeline execution without running any commands. Shows each step that would execute. |
| `pipeline_run` | 2-leg | Execute a RAPS pipeline file. Runs each step sequentially (or in parallel for parallel steps). Returns execution summary. |
| `pipeline_list_templates` | 2-leg | List available pipeline template files. Searches current directory and common locations for .yaml/.yml/.json pipeline files. |
| `workflow_prepare_for_viewing` | 2-leg | Upload a file and start SVF2 translation in one step. Returns URN and translation status. Use translate_status to poll for completion. |
| `workflow_analyze_model` | 2-leg | Get comprehensive analysis of a translated model: translation status, metadata, and properties in one call. |
| `workflow_batch_translate` | 2-leg | Start translation for multiple URNs at once. Returns status for each. Pass comma-separated URNs. |
| `workflow_compare_versions` | 2-leg | Compare two model versions by checking translation status for both URNs. Reports readiness for visual comparison in Autodesk Viewer. |
| `workflow_setup_project` | 2-leg | Set up a new project workspace: creates a persistent bucket for file storage. Returns bucket key and next steps for uploading and translating models. |
| `swarm_status` | 2-leg | Get swarm orchestration health: circuit breaker states, rate limit budgets, and response cache stats. Useful for diagnosing API connectivity issues. |

## Output Schemas

All structured output types are queryable at runtime:

```
raps schema list                 # list available types
raps schema generate <name>      # JSON Schema for a specific type
raps schema all                  # all schemas as one JSON object
```

## Agent Invariants

Things that are true of RAPS CLI that agents cannot discover from `--help`:

- Non-interactive output defaults to JSON automatically (piped stdout → JSON, TTY → table)
- `RAPS_OUTPUT_FORMAT=json` forces JSON regardless of TTY
- `--dry-run` is supported by all `admin` bulk operations and `pipeline` commands
- All bucket keys must be globally unique, 3–128 chars, lowercase alphanumeric + hyphens
- Object URNs for translation must be Base64-encoded; get them via `raps object urn` or the `object_urn` MCP tool
- 3-legged tokens expire; use `auth_status` to check before long workflows
- Admin bulk operations are resumable — if interrupted, use `admin_operation_resume`
- `raps api` is a raw HTTP passthrough for API endpoints not yet covered by dedicated commands

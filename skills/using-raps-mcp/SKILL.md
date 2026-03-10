---
name: using-raps-mcp
version: "1.0"
description: Use when calling any RAPS MCP tool, working with Autodesk Platform Services APIs, or setting up RAPS MCP for the first time — covers batch tool fetching, domain groupings, and common parameter patterns.
---

# Using RAPS MCP Tools

RAPS MCP server exposes 107 tools for Autodesk Platform Services. All tools are deferred — fetch schemas via `ToolSearch` before calling.

## First-Time Setup

Install this skill via CLI or MCP:

```bash
raps skill install using-raps-mcp
```

Then verify auth works: fetch `auth_test` via ToolSearch and call it.

## How to Call Tools Efficiently

1. Identify the domain from the user's request
2. Batch-fetch ALL tools in that domain with a single `ToolSearch` call using `select:`
3. Call the tools — schemas are now available for the rest of the session

**Always batch-fetch the full domain group, not individual tools.**

## Domain Groups (ToolSearch Queries)

Use these exact `select:` queries to batch-fetch by domain:

### Authentication (4 tools)
```
select:mcp__raps__auth_test,mcp__raps__auth_status,mcp__raps__auth_login,mcp__raps__auth_logout
```

### OSS Buckets (4 tools)
```
select:mcp__raps__bucket_list,mcp__raps__bucket_create,mcp__raps__bucket_get,mcp__raps__bucket_delete
```

### OSS Objects (10 tools)
```
select:mcp__raps__object_list,mcp__raps__object_upload,mcp__raps__object_upload_batch,mcp__raps__object_download,mcp__raps__object_info,mcp__raps__object_delete,mcp__raps__object_delete_batch,mcp__raps__object_copy,mcp__raps__object_signed_url,mcp__raps__object_urn
```

### Translation (2 tools)
```
select:mcp__raps__translate_start,mcp__raps__translate_status
```

### Hubs & Projects (6 tools)
```
select:mcp__raps__hub_list,mcp__raps__hub_info,mcp__raps__project_list,mcp__raps__project_info,mcp__raps__project_create,mcp__raps__project_update
```

### Project Users (5 tools)
```
select:mcp__raps__project_users_list,mcp__raps__project_user_add,mcp__raps__project_user_remove,mcp__raps__project_user_update,mcp__raps__project_users_import
```

### Folders & Items (9 tools)
```
select:mcp__raps__folder_list,mcp__raps__folder_contents,mcp__raps__folder_create,mcp__raps__item_info,mcp__raps__item_create,mcp__raps__item_delete,mcp__raps__item_rename,mcp__raps__item_versions,mcp__raps__project_archive
```

### Templates (6 tools)
```
select:mcp__raps__template_list,mcp__raps__template_info,mcp__raps__template_create,mcp__raps__template_update,mcp__raps__template_archive,mcp__raps__template_convert
```

### Account Admin (9 tools)
```
select:mcp__raps__admin_user_list,mcp__raps__admin_user_add,mcp__raps__admin_user_remove,mcp__raps__admin_user_update_role,mcp__raps__admin_project_list,mcp__raps__admin_folder_set_permissions,mcp__raps__admin_operation_list,mcp__raps__admin_operation_status,mcp__raps__admin_operation_cancel,mcp__raps__admin_operation_resume
```

### Issues (7 tools)
```
select:mcp__raps__issue_list,mcp__raps__issue_get,mcp__raps__issue_create,mcp__raps__issue_update,mcp__raps__issue_comments_list,mcp__raps__issue_comment_add,mcp__raps__issue_comment_delete
```

### RFIs (4 tools)
```
select:mcp__raps__rfi_list,mcp__raps__rfi_get,mcp__raps__rfi_create,mcp__raps__rfi_update
```

### ACC (Assets, Submittals, Checklists) (9 tools)
```
select:mcp__raps__acc_assets_list,mcp__raps__asset_get,mcp__raps__asset_create,mcp__raps__asset_update,mcp__raps__asset_delete,mcp__raps__acc_submittals_list,mcp__raps__submittal_create,mcp__raps__submittal_update,mcp__raps__acc_checklists_list,mcp__raps__checklist_templates_list,mcp__raps__checklist_create,mcp__raps__checklist_update
```

### Design Automation (6 tools)
```
select:mcp__raps__da_engines_list,mcp__raps__da_activities_list,mcp__raps__da_appbundles_list,mcp__raps__da_workitem_create,mcp__raps__da_workitem_status,mcp__raps__da_workitems_list
```

### Reality Capture (7 tools)
```
select:mcp__raps__reality_list,mcp__raps__reality_create,mcp__raps__reality_process,mcp__raps__reality_status,mcp__raps__reality_result,mcp__raps__reality_delete,mcp__raps__reality_formats
```

### Webhooks (6 tools)
```
select:mcp__raps__webhook_list,mcp__raps__webhook_get,mcp__raps__webhook_create,mcp__raps__webhook_update,mcp__raps__webhook_delete,mcp__raps__webhook_events
```

### Compound Workflows (5 tools)
```
select:mcp__raps__workflow_setup_project,mcp__raps__workflow_prepare_for_viewing,mcp__raps__workflow_batch_translate,mcp__raps__workflow_compare_versions,mcp__raps__workflow_analyze_model
```

### Pipelines (4 tools)
```
select:mcp__raps__pipeline_list_templates,mcp__raps__pipeline_validate,mcp__raps__pipeline_dry_run,mcp__raps__pipeline_run
```

### Reports & Misc (4 tools)
```
select:mcp__raps__report_issues_summary,mcp__raps__report_rfi_summary,mcp__raps__api_request,mcp__raps__swarm_status
```

## Common Parameters

Most tools that operate within a project need these IDs found via discovery:

| Parameter | How to get it | Example |
|-----------|--------------|---------|
| `hub_id` | `hub_list` | `b.abc123-def456...` |
| `account_id` | `hub_list` (strip `b.` prefix) | `abc123-def456...` |
| `project_id` | `project_list` (needs hub_id) | `b.xyz789-...` |
| `folder_id` | `folder_list` (needs project_id) | `urn:adsk.wipprod:fs.folder:...` |
| `item_id` | `folder_contents` (needs project_id + folder_id) | `urn:adsk.wipprod:dm.lineage:...` |
| `bucket_key` | `bucket_list` or user-provided | `my-bucket-name` |
| `object_key` | `object_list` (needs bucket_key) | `model.rvt` |
| `urn` | `object_urn` (needs bucket_key + object_key) | base64 encoded |

**Discovery chain:** `hub_list` -> `project_list` -> `folder_list` -> `folder_contents`

## Auth Requirements

| Auth type | When needed | Tools |
|-----------|------------|-------|
| 2-legged | Most tools — set `APS_CLIENT_ID` + `APS_CLIENT_SECRET` env vars | All `bucket_*`, `object_*`, `translate_*`, `admin_*`, `da_*` |
| 3-legged | User-context operations — run `raps auth login` first | `hub_*`, `project_*`, `folder_*`, `item_*`, `issue_*`, `rfi_*` |

Run `auth_test` (2-legged) or `auth_status` (both) to verify before proceeding.

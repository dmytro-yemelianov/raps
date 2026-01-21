# Quickstart: MCP Project Management and Bulk Operations

**Feature**: 001-mcp-project-bulk-ops
**Date**: 2026-01-19

This guide shows how to use the new MCP tools with AI assistants like Claude, Cursor, or other MCP clients.

---

## Prerequisites

1. **RAPS installed** with MCP server capability
2. **APS credentials** configured:
   - `APS_CLIENT_ID` and `APS_CLIENT_SECRET` for 2-legged auth (object operations)
   - User logged in via `raps auth login` for 3-legged auth (project operations)

## Starting the MCP Server

```bash
raps serve
```

Configure your MCP client to connect to the RAPS server.

---

## Object Operations (2-Legged Auth)

### Upload a Single File

> "Upload the file /Users/me/models/building.rvt to my-bucket"

The AI will call `object_upload` with:
- `bucket_key`: "my-bucket"
- `file_path`: "/Users/me/models/building.rvt"

### Upload Multiple Files

> "Upload all the files in /Users/me/models/ to my-bucket: building.rvt, site.dwg, and specs.pdf"

The AI will call `object_upload_batch` with:
- `bucket_key`: "my-bucket"
- `file_paths`: ["/Users/me/models/building.rvt", "/Users/me/models/site.dwg", "/Users/me/models/specs.pdf"]

### Download a File

> "Download building.rvt from my-bucket to /tmp/download.rvt"

The AI will call `object_download` with:
- `bucket_key`: "my-bucket"
- `object_key`: "building.rvt"
- `output_path`: "/tmp/download.rvt"

### Get Object Details

> "What's the size and type of building.rvt in my-bucket?"

The AI will call `object_info` with:
- `bucket_key`: "my-bucket"
- `object_key`: "building.rvt"

### Copy an Object

> "Copy building.rvt from production-bucket to backup-bucket"

The AI will call `object_copy` with:
- `source_bucket`: "production-bucket"
- `source_key`: "building.rvt"
- `dest_bucket`: "backup-bucket"

### Delete Multiple Objects

> "Delete all the temp files from my-bucket: temp1.txt, temp2.txt, temp3.txt"

The AI will call `object_delete_batch` with:
- `bucket_key`: "my-bucket"
- `object_keys`: ["temp1.txt", "temp2.txt", "temp3.txt"]

---

## Project Operations (3-Legged Auth)

**Note**: You must be logged in with `raps auth login` first.

### Get Project Details

> "Show me details about my Hospital Project including its top folders"

The AI will call `project_info` with:
- `hub_id`: "b.abc123" (discovered from context)
- `project_id`: "b.project456"

### List Project Users

> "Who has access to the Hospital Project?"

The AI will call `project_users_list` with:
- `project_id`: "b.project456"
- `limit`: 50

### Browse Folder Contents

> "What's in the Project Files folder?"

The AI will call `folder_contents` with:
- `project_id`: "b.project456"
- `folder_id`: "urn:adsk.wiprod:fs.folder:co.abc123"

---

## ACC Project Admin (3-Legged Auth, ACC Only)

**Note**: Requires account admin privileges. ACC only (not BIM 360).

### Create a New Project

> "Create a new ACC project called 'New Hospital Wing' in my account"

The AI will call `project_create` with:
- `account_id`: "abc123"
- `name`: "New Hospital Wing"
- `products`: ["build", "docs"]

### Create from Template

> "Create a project called 'Tower B' using the 'Standard Hospital' template"

The AI will call `project_create` with:
- `account_id`: "abc123"
- `name`: "Tower B"
- `template_project_id`: "b.template123"

**Warning**: Template members are NOT automatically assigned to the new project.

### Add a User to a Project

> "Add john@company.com to the Hospital Project as a project admin"

The AI will call `project_user_add` with:
- `project_id`: "b.project456"
- `email`: "john@company.com"
- `role_id`: "project_admin"

### Bulk Import Users

> "Add these users to the project: alice@company.com, bob@company.com, charlie@company.com"

The AI will call `project_users_import` with:
- `project_id`: "b.project456"
- `users`: [
    {"email": "alice@company.com"},
    {"email": "bob@company.com"},
    {"email": "charlie@company.com"}
  ]

---

## Item Management (3-Legged Auth)

### Create Item from Storage

> "Add the uploaded building.rvt to the Project Files folder"

The AI will call `item_create` with:
- `project_id`: "b.project456"
- `folder_id`: "urn:adsk.wiprod:fs.folder:co.abc123"
- `display_name`: "building.rvt"
- `storage_id`: "urn:adsk.objects:os.object:my-bucket/building.rvt"

### Rename an Item

> "Rename 'building.rvt' to 'Hospital Building v2.rvt'"

The AI will call `item_rename` with:
- `project_id`: "b.project456"
- `item_id`: "urn:adsk.wipprod:dm.lineage:xyz789"
- `new_name`: "Hospital Building v2.rvt"

### Delete an Item

> "Remove the old model from the project"

The AI will call `item_delete` with:
- `project_id`: "b.project456"
- `item_id`: "urn:adsk.wipprod:dm.lineage:xyz789"

---

## Common Workflows

### Upload and Add to Project

> "Upload building.rvt from my desktop to my-bucket, then add it to the Hospital Project's Models folder"

1. AI calls `object_upload` to upload the file
2. AI calls `item_create` to link it to the project folder

### Batch Upload with Summary

> "Upload all the PDFs in /Users/me/documents/ to docs-bucket and tell me which ones succeeded"

AI calls `object_upload_batch` and presents the summary showing successes and failures.

### Project Setup

> "Create a new project called 'Tower C' and add the engineering team: alice@company.com, bob@company.com, charlie@company.com"

1. AI calls `project_create` to create the project
2. AI calls `project_users_import` to add all team members at once

---

## Error Handling

All tools return clear error messages:

**File not found**:
```
Error: File not found: /path/to/missing.rvt
```

**Not authenticated**:
```
Error: This operation requires 3-legged authentication.
Please run 'raps auth login' first.
```

**Object exists (copy)**:
```
Warning: Object 'model.rvt' already exists in 'backup-bucket' (skipped)
Delete it first if you want to overwrite.
```

**Partial batch failure**:
```
Batch upload complete: 4 succeeded, 1 failed

Results:
✓ file1.rvt (15.2 MB)
✓ file2.pdf (2.1 MB)
✓ file3.dwg (8.4 MB)
✓ file4.jpg (3.4 MB)
✗ file5.txt (File not found)
```

---

## Tips

1. **Use batch operations** for multiple files - it's faster and provides better summaries
2. **Check auth status** first if project operations fail
3. **Copy preserves metadata** but creates an independent copy
4. **ACC project creation is asynchronous** - the tool polls until activation (up to 60 seconds)
5. **Template cloning doesn't copy members** - you need to add users separately

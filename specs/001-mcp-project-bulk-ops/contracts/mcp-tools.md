# MCP Tool Contracts: Project Management and Bulk Operations

**Feature**: 001-mcp-project-bulk-ops
**Date**: 2026-01-19

This document defines the JSON Schema contracts for all 15 new MCP tools.

---

## Object Operations (2-Legged Auth)

### object_upload

Upload a single file to an OSS bucket.

```json
{
  "name": "object_upload",
  "description": "Upload a file to an OSS bucket. Automatically uses chunked upload for files > 100MB. Supports resume for interrupted uploads.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "bucket_key": {
        "type": "string",
        "description": "Target bucket key (3-128 chars, lowercase)"
      },
      "file_path": {
        "type": "string",
        "description": "Absolute path to the file to upload"
      },
      "object_key": {
        "type": "string",
        "description": "Optional object key (defaults to filename)"
      }
    },
    "required": ["bucket_key", "file_path"]
  }
}
```

**Response Format**:
```
Uploaded 'model.rvt' to 'my-bucket'
* Object Key: model.rvt
* Size: 15,234,567 bytes
* SHA1: abc123...
* URN: urn:adsk.objects:os.object:...
```

---

### object_upload_batch

Upload multiple files to an OSS bucket with 4-way concurrency.

```json
{
  "name": "object_upload_batch",
  "description": "Upload multiple files to an OSS bucket. Uses 4 parallel uploads. Returns summary with individual results.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "bucket_key": {
        "type": "string",
        "description": "Target bucket key"
      },
      "file_paths": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Array of absolute file paths to upload"
      }
    },
    "required": ["bucket_key", "file_paths"]
  }
}
```

**Response Format**:
```
Batch upload complete: 4 succeeded, 1 failed

Results:
✓ model.rvt (15.2 MB)
✓ plans.pdf (2.1 MB)
✓ specs.docx (512 KB)
✓ photo.jpg (3.4 MB)
✗ missing.txt (File not found)
```

---

### object_download

Download an object from an OSS bucket to a local file.

```json
{
  "name": "object_download",
  "description": "Download an object from OSS to a local file path.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "bucket_key": {
        "type": "string",
        "description": "Source bucket key"
      },
      "object_key": {
        "type": "string",
        "description": "Object key to download"
      },
      "output_path": {
        "type": "string",
        "description": "Local file path to save the downloaded file"
      }
    },
    "required": ["bucket_key", "object_key", "output_path"]
  }
}
```

**Response Format**:
```
Downloaded 'model.rvt' to '/tmp/model.rvt'
* Size: 15,234,567 bytes
```

---

### object_info

Get detailed metadata for an object without downloading it.

```json
{
  "name": "object_info",
  "description": "Get detailed metadata for an object including size, content type, SHA1 hash, and timestamps.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "bucket_key": {
        "type": "string",
        "description": "Bucket key"
      },
      "object_key": {
        "type": "string",
        "description": "Object key"
      }
    },
    "required": ["bucket_key", "object_key"]
  }
}
```

**Response Format**:
```
Object: model.rvt in my-bucket

* Size: 15,234,567 bytes (14.5 MB)
* Content-Type: application/octet-stream
* SHA1: abc123def456...
* Created: 2026-01-15T10:30:00Z
* Modified: 2026-01-18T14:22:00Z
* URN: urn:adsk.objects:os.object:my-bucket/model.rvt
```

---

### object_copy

Copy an object from one bucket to another.

```json
{
  "name": "object_copy",
  "description": "Copy an object from one bucket to another. If destination exists, returns existing object with warning (non-destructive).",
  "inputSchema": {
    "type": "object",
    "properties": {
      "source_bucket": {
        "type": "string",
        "description": "Source bucket key"
      },
      "source_key": {
        "type": "string",
        "description": "Source object key"
      },
      "dest_bucket": {
        "type": "string",
        "description": "Destination bucket key"
      },
      "dest_key": {
        "type": "string",
        "description": "Destination object key (defaults to source key)"
      }
    },
    "required": ["source_bucket", "source_key", "dest_bucket"]
  }
}
```

**Response Format (success)**:
```
Copied 'model.rvt' from 'bucket-a' to 'bucket-b'
* Size: 15,234,567 bytes
* New URN: urn:adsk.objects:os.object:bucket-b/model.rvt
```

**Response Format (exists)**:
```
Warning: Object 'model.rvt' already exists in 'bucket-b' (skipped)
* Existing size: 15,234,567 bytes
* Delete it first if you want to overwrite.
```

---

### object_delete_batch

Delete multiple objects from a bucket.

```json
{
  "name": "object_delete_batch",
  "description": "Delete multiple objects from an OSS bucket. Returns summary with individual results.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "bucket_key": {
        "type": "string",
        "description": "Bucket key"
      },
      "object_keys": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Array of object keys to delete"
      }
    },
    "required": ["bucket_key", "object_keys"]
  }
}
```

**Response Format**:
```
Batch delete complete: 3 deleted, 1 skipped, 0 failed

Results:
✓ model.rvt (deleted)
✓ plans.pdf (deleted)
✓ specs.docx (deleted)
○ missing.txt (not found, skipped)
```

---

## Project Management (3-Legged Auth)

### project_info

Get detailed project information including top-level folders.

```json
{
  "name": "project_info",
  "description": "Get project details including name, type, scopes, and top-level folders. Requires 3-legged auth.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "hub_id": {
        "type": "string",
        "description": "Hub ID (e.g., b.abc123)"
      },
      "project_id": {
        "type": "string",
        "description": "Project ID (e.g., b.project123)"
      }
    },
    "required": ["hub_id", "project_id"]
  }
}
```

**Response Format**:
```
Project: My Construction Project
* ID: b.project123
* Hub: b.abc123
* Scopes: b360project, b360project.document

Top Folders:
* Project Files (urn:adsk.wiprod:fs.folder:co.abc)
* Plans (urn:adsk.wiprod:fs.folder:co.def)
* Shared (urn:adsk.wiprod:fs.folder:co.ghi)
```

---

### project_users_list

List users with access to a project.

```json
{
  "name": "project_users_list",
  "description": "List users with access to a project with pagination. Requires 3-legged auth.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_id": {
        "type": "string",
        "description": "Project ID"
      },
      "limit": {
        "type": "integer",
        "description": "Max results per page (default: 50, max: 200)"
      },
      "offset": {
        "type": "integer",
        "description": "Starting index for pagination"
      }
    },
    "required": ["project_id"]
  }
}
```

**Response Format**:
```
Project Users (showing 1-50 of 127):

1. John Smith (john@example.com)
   * Role: Project Admin
   * Status: Active

2. Jane Doe (jane@example.com)
   * Role: Project User
   * Status: Active

...

Use offset=50 to see next page.
```

---

### folder_contents

List contents of a folder with pagination.

```json
{
  "name": "folder_contents",
  "description": "List all items and subfolders within a folder. Requires 3-legged auth.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_id": {
        "type": "string",
        "description": "Project ID"
      },
      "folder_id": {
        "type": "string",
        "description": "Folder ID (URN format)"
      },
      "limit": {
        "type": "integer",
        "description": "Max results per page (default: 50)"
      },
      "offset": {
        "type": "integer",
        "description": "Starting index"
      }
    },
    "required": ["project_id", "folder_id"]
  }
}
```

**Response Format**:
```
Folder Contents (showing 1-50 of 85):

Subfolders:
📁 2024 Revisions (urn:adsk.wiprod:fs.folder:co.xyz)
📁 Archive (urn:adsk.wiprod:fs.folder:co.abc)

Items:
📄 Building Model.rvt (v3, 45.2 MB)
📄 Site Plan.dwg (v2, 8.1 MB)
📄 Specifications.pdf (v1, 2.3 MB)

Use offset=50 to see next page.
```

---

## ACC Project Admin (3-Legged Auth, ACC Only)

### project_create

Create a new ACC project.

```json
{
  "name": "project_create",
  "description": "Create a new ACC project from scratch or from a template. ACC only (not BIM 360). Polls until project is activated. Requires 3-legged auth.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "account_id": {
        "type": "string",
        "description": "ACC account ID"
      },
      "name": {
        "type": "string",
        "description": "Project name"
      },
      "template_project_id": {
        "type": "string",
        "description": "Optional template project ID to clone from"
      },
      "products": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Products to enable (e.g., ['build', 'docs', 'model'])"
      }
    },
    "required": ["account_id", "name"]
  }
}
```

**Response Format**:
```
Created ACC project: My New Project
* ID: b.newproject123
* Account: abc123
* Status: Active
* Products: Build, Docs

Note: Template members are NOT auto-assigned when cloning.
```

---

### project_user_add

Add a user to an ACC project.

```json
{
  "name": "project_user_add",
  "description": "Add a user to an ACC project with optional role assignment. Requires 3-legged auth.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_id": {
        "type": "string",
        "description": "Project ID"
      },
      "email": {
        "type": "string",
        "description": "User email address"
      },
      "role_id": {
        "type": "string",
        "description": "Optional role ID to assign"
      }
    },
    "required": ["project_id", "email"]
  }
}
```

**Response Format**:
```
Added user to project:
* Email: user@example.com
* Name: John Smith
* Role: Project Admin
* Status: Active
```

---

### project_users_import

Bulk import users to an ACC project.

```json
{
  "name": "project_users_import",
  "description": "Import multiple users to an ACC project at once. Requires 3-legged auth.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_id": {
        "type": "string",
        "description": "Project ID"
      },
      "users": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "email": {"type": "string"},
            "role_id": {"type": "string"}
          },
          "required": ["email"]
        },
        "description": "Array of users to import"
      }
    },
    "required": ["project_id", "users"]
  }
}
```

**Response Format**:
```
User import complete: 4 imported, 1 failed

Results:
✓ alice@example.com (Project Admin)
✓ bob@example.com (Project User)
✓ charlie@example.com (Viewer)
✓ diana@example.com (Project User)
✗ invalid@notfound.com (User not found in account)
```

---

## Item Management (3-Legged Auth)

### item_create

Create an item in a folder from an OSS storage object.

```json
{
  "name": "item_create",
  "description": "Create a new item in a project folder by linking an OSS storage object. Requires 3-legged auth.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_id": {
        "type": "string",
        "description": "Project ID"
      },
      "folder_id": {
        "type": "string",
        "description": "Target folder ID (URN format)"
      },
      "display_name": {
        "type": "string",
        "description": "Display name for the item"
      },
      "storage_id": {
        "type": "string",
        "description": "OSS storage object URN"
      }
    },
    "required": ["project_id", "folder_id", "display_name", "storage_id"]
  }
}
```

**Response Format**:
```
Created item in project folder:
* Display Name: Building Model.rvt
* Item ID: urn:adsk.wipprod:dm.lineage:abc123
* Version: 1
* Folder: Project Files
```

---

### item_delete

Delete an item from a project.

```json
{
  "name": "item_delete",
  "description": "Delete an item from a project folder. Requires 3-legged auth.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_id": {
        "type": "string",
        "description": "Project ID"
      },
      "item_id": {
        "type": "string",
        "description": "Item ID to delete"
      }
    },
    "required": ["project_id", "item_id"]
  }
}
```

**Response Format**:
```
Deleted item from project:
* Item ID: urn:adsk.wipprod:dm.lineage:abc123
```

---

### item_rename

Rename an item's display name.

```json
{
  "name": "item_rename",
  "description": "Update an item's display name. Requires 3-legged auth.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "project_id": {
        "type": "string",
        "description": "Project ID"
      },
      "item_id": {
        "type": "string",
        "description": "Item ID"
      },
      "new_name": {
        "type": "string",
        "description": "New display name"
      }
    },
    "required": ["project_id", "item_id", "new_name"]
  }
}
```

**Response Format**:
```
Renamed item:
* Old Name: Building Model.rvt
* New Name: Building Model v2.rvt
* Item ID: urn:adsk.wipprod:dm.lineage:abc123
```

---

## Authentication Requirements Summary

| Tool | Auth Type | Notes |
|------|-----------|-------|
| `object_upload` | 2-Legged | Client credentials |
| `object_upload_batch` | 2-Legged | Client credentials |
| `object_download` | 2-Legged | Client credentials |
| `object_info` | 2-Legged | Client credentials |
| `object_copy` | 2-Legged | Client credentials |
| `object_delete_batch` | 2-Legged | Client credentials |
| `project_info` | 3-Legged | User login required |
| `project_users_list` | 3-Legged | User login required |
| `folder_contents` | 3-Legged | User login required |
| `project_create` | 3-Legged | ACC only, account admin |
| `project_user_add` | 3-Legged | Project admin |
| `project_users_import` | 3-Legged | Project admin |
| `item_create` | 3-Legged | User login required |
| `item_delete` | 3-Legged | User login required |
| `item_rename` | 3-Legged | User login required |

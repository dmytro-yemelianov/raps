# Data Model: MCP Project Management and Bulk Operations

**Feature**: 001-mcp-project-bulk-ops
**Date**: 2026-01-19

## Entities

### Core Domain Entities (Existing)

These entities already exist in workspace crates and are used by MCP tools.

#### ObjectInfo (raps-oss)

```rust
pub struct ObjectInfo {
    pub bucket_key: String,
    pub object_key: String,
    pub object_id: String,
    pub sha1: Option<String>,
    pub size: u64,
    pub location: Option<String>,
    pub content_type: Option<String>,
}
```

**Used by**: `object_upload`, `object_upload_batch`, `object_info`, `object_copy`

#### Project (raps-dm)

```rust
pub struct Project {
    pub id: String,
    pub name: String,
    pub scopes: Option<Vec<String>>,
}
```

**Used by**: `project_info`, `project_create`

#### Folder (raps-dm)

```rust
pub struct Folder {
    pub id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub create_time: Option<String>,
    pub last_modified_time: Option<String>,
}
```

**Used by**: `project_info`, `folder_contents`

#### Item (raps-dm)

```rust
pub struct Item {
    pub id: String,
    pub display_name: String,
    pub create_time: Option<String>,
    pub last_modified_time: Option<String>,
    pub extension: Option<serde_json::Value>,
}
```

**Used by**: `item_create`, `item_delete`, `item_rename`, `folder_contents`

#### ProjectUser (raps-acc)

```rust
pub struct ProjectUser {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub role_id: Option<String>,
    pub status: Option<String>,
    pub company_id: Option<String>,
    pub products: Vec<ProductAccess>,
}
```

**Used by**: `project_users_list`, `project_user_add`, `project_users_import`

---

### New Entities

#### ObjectDetails (raps-oss - New)

Extended object metadata returned by `object_info` tool.

```rust
pub struct ObjectDetails {
    pub bucket_key: String,
    pub object_key: String,
    pub object_id: String,
    pub sha1: String,
    pub size: u64,
    pub content_type: String,
    pub content_disposition: Option<String>,
    pub created_date: String,        // ISO 8601
    pub last_modified_date: String,  // ISO 8601
    pub location: Option<String>,
}
```

**Validation**:
- `bucket_key`: 3-128 chars, lowercase alphanumeric + dash/underscore
- `object_key`: URL-encoded file path
- `size`: >= 0

**State Transitions**: N/A (read-only)

---

#### BatchUploadResult (MCP - New)

Result of batch upload operation.

```rust
pub struct BatchUploadResult {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<UploadItemResult>,
}

pub struct UploadItemResult {
    pub file_path: String,
    pub object_key: Option<String>,
    pub success: bool,
    pub size: Option<u64>,
    pub error: Option<String>,
}
```

**Validation**:
- `total` = `successful` + `failed`
- `results.len()` = `total`

---

#### BatchDeleteResult (MCP - New)

Result of batch delete operation.

```rust
pub struct BatchDeleteResult {
    pub total: usize,
    pub deleted: usize,
    pub skipped: usize,  // Not found
    pub failed: usize,
    pub results: Vec<DeleteItemResult>,
}

pub struct DeleteItemResult {
    pub object_key: String,
    pub success: bool,
    pub skipped: bool,   // true if object didn't exist
    pub error: Option<String>,
}
```

---

#### ProjectCreationJob (raps-acc - New)

Returned when creating ACC project.

```rust
pub struct ProjectCreationJob {
    pub job_id: String,
    pub project_id: Option<String>,  // Available after polling
    pub status: ProjectCreationStatus,
}

pub enum ProjectCreationStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}
```

**State Transitions**:
```
Pending -> Processing -> Completed
                     \-> Failed
```

---

#### ImportUsersResult (raps-acc - New)

Result of bulk user import.

```rust
pub struct ImportUsersResult {
    pub total: usize,
    pub imported: usize,
    pub failed: usize,
    pub errors: Vec<ImportUserError>,
}

pub struct ImportUserError {
    pub email: String,
    pub error: String,
}
```

---

## Entity Relationships

```
┌─────────────────────────────────────────────────────────────────┐
│                        Object Storage                           │
├─────────────────────────────────────────────────────────────────┤
│  Bucket (1) ──────< Object (many)                               │
│     │                   │                                       │
│     │                   └─> ObjectInfo / ObjectDetails          │
│     │                                                           │
│     └─> BatchUploadResult (aggregates UploadItemResult)         │
│     └─> BatchDeleteResult (aggregates DeleteItemResult)         │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      Data Management                            │
├─────────────────────────────────────────────────────────────────┤
│  Hub (1) ──────< Project (many)                                 │
│                      │                                          │
│                      ├──────< Folder (many)                     │
│                      │           │                              │
│                      │           └──────< Item (many)           │
│                      │                      │                   │
│                      │                      └─> linked to Object│
│                      │                                          │
│                      └──────< ProjectUser (many)                │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      ACC Project Admin                          │
├─────────────────────────────────────────────────────────────────┤
│  Account (1) ──────< Project (many)                             │
│                          │                                      │
│                          └─> ProjectCreationJob (async)         │
│                          └──────< ProjectUser (import)          │
│                                      │                          │
│                                      └─> ImportUsersResult      │
└─────────────────────────────────────────────────────────────────┘
```

---

## MCP Tool Input/Output Mapping

### Object Operations

| Tool | Input | Output |
|------|-------|--------|
| `object_upload` | bucket_key, file_path | ObjectInfo (formatted string) |
| `object_upload_batch` | bucket_key, file_paths[] | BatchUploadResult (formatted string) |
| `object_download` | bucket_key, object_key, output_path | Success message with size |
| `object_info` | bucket_key, object_key | ObjectDetails (formatted string) |
| `object_copy` | src_bucket, src_key, dest_bucket, dest_key? | ObjectInfo or warning |
| `object_delete_batch` | bucket_key, object_keys[] | BatchDeleteResult (formatted string) |

### Project Management

| Tool | Input | Output |
|------|-------|--------|
| `project_info` | hub_id, project_id | Project + Folders (formatted) |
| `project_users_list` | project_id, limit?, offset? | ProjectUser[] (formatted) |
| `folder_contents` | project_id, folder_id, limit?, offset? | Folders + Items (formatted) |

### ACC Project Admin

| Tool | Input | Output |
|------|-------|--------|
| `project_create` | account_id, name, template_id?, products[] | Project (after activation) |
| `project_user_add` | project_id, email, role_id? | ProjectUser (formatted) |
| `project_users_import` | project_id, users[] | ImportUsersResult (formatted) |

### Item Management

| Tool | Input | Output |
|------|-------|--------|
| `item_create` | project_id, folder_id, display_name, storage_id | Item (formatted) |
| `item_delete` | project_id, item_id | Success message |
| `item_rename` | project_id, item_id, new_name | Item (formatted) |

---

## Validation Rules

### Object Keys
- URL-encoded path segments
- Max length: 1024 characters
- Cannot start with `/`

### Bucket Keys
- 3-128 characters
- Lowercase letters, digits, dash, underscore
- Must start with letter or digit
- Globally unique

### Project IDs
- Format: `b.{uuid}` (BIM 360/ACC)
- The `b.` prefix is optional - normalized internally

### File Paths (for upload)
- Must exist on filesystem
- Must be readable
- Size limit: 5GB

### User Emails
- Valid email format
- Must exist in Autodesk account

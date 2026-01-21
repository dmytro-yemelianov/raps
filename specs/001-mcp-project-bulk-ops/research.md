# Research: MCP Project Management and Bulk Operations

**Feature**: 001-mcp-project-bulk-ops
**Date**: 2026-01-19

## Summary

Research findings for implementing 15 new MCP tools. Analysis covers existing crate capabilities, API gaps requiring new methods, and implementation patterns.

---

## 1. Object Operations (FR-001 to FR-006)

### Existing Capabilities in raps-oss

| Method | Exists | Location | Notes |
|--------|--------|----------|-------|
| `upload_object()` | ✅ Yes | `raps-oss/src/lib.rs` | Auto-selects single/multipart based on size |
| `upload_object_with_options()` | ✅ Yes | `raps-oss/src/lib.rs` | Supports resume for interrupted uploads |
| `upload_multipart()` | ✅ Yes | `raps-oss/src/lib.rs` | 5MB chunks, 5 concurrent, state persistence |
| `download_object()` | ✅ Yes | `raps-oss/src/lib.rs` | Streams from signed S3 URL |
| `get_signed_download_url()` | ✅ Yes | `raps-oss/src/lib.rs` | For external download |
| `list_objects()` | ✅ Yes | `raps-oss/src/lib.rs` | With pagination |
| `delete_object()` | ✅ Yes | `raps-oss/src/lib.rs` | Single object |
| `get_urn()` | ✅ Yes | `raps-oss/src/lib.rs` | URN generation |
| Object info/metadata | ⚠️ Partial | `ObjectInfo` struct | Returned from upload, not queryable |
| Object copy | ❌ No | - | Not in APS OSS API |
| Batch delete | ❌ No | - | Must loop single deletes |

### Decisions for MCP Tools

| Tool | Decision | Rationale |
|------|----------|-----------|
| `object_upload` | Wrap `upload_object_with_options(resume=true)` | Existing method handles all cases |
| `object_upload_batch` | New MCP-level loop with 4-way semaphore | Existing upload + concurrency control |
| `object_download` | Wrap `download_object()` | Existing method with progress |
| `object_info` | New method: `get_object_details()` | Need HEAD request to OSS API |
| `object_copy` | Download + Upload pattern | OSS API lacks server-side copy |
| `object_delete_batch` | New MCP-level loop with semaphore | Loop `delete_object()` with summary |

### New Method Required

```rust
// raps-oss/src/lib.rs - new method needed
pub async fn get_object_details(
    &self,
    bucket_key: &str,
    object_key: &str,
) -> Result<ObjectInfo>
```
- Uses: `GET /oss/v2/buckets/{bucket_key}/objects/{object_key}/details`
- Returns: size, SHA1, content_type, created_date, last_modified

---

## 2. Project Management (FR-007 to FR-009)

### Existing Capabilities in raps-dm

| Method | Exists | Location | Notes |
|--------|--------|----------|-------|
| `get_project()` | ✅ Yes | `raps-dm/src/lib.rs` | Returns Project with attributes |
| `list_projects()` | ✅ Yes | `raps-dm/src/lib.rs` | Per hub |
| `get_top_folders()` | ✅ Yes | `raps-dm/src/lib.rs` | Top-level project folders |
| `list_folder_contents()` | ✅ Yes | `raps-dm/src/lib.rs` | Items and subfolders |
| `create_folder()` | ✅ Yes | `raps-dm/src/lib.rs` | In parent folder |
| `get_item()` | ✅ Yes | `raps-dm/src/lib.rs` | Item details |
| `get_item_versions()` | ✅ Yes | `raps-dm/src/lib.rs` | Version history |
| `create_item_from_storage()` | ✅ Yes | `raps-dm/src/lib.rs` | Links OSS object |

### Existing Capabilities in raps-acc

| Method | Exists | Location | Notes |
|--------|--------|----------|-------|
| `list_project_users()` | ✅ Yes | `raps-acc/src/users.rs` | Paginated list |
| `add_user()` | ✅ Yes | `raps-acc/src/users.rs` | Single user to project |

### Decisions for MCP Tools

| Tool | Decision | Rationale |
|------|----------|-----------|
| `project_info` | Combine `get_project()` + `get_top_folders()` | Rich project view |
| `project_users_list` | Wrap `list_project_users()` | Existing method |
| `folder_contents` | Wrap `list_folder_contents()` | With pagination params |

---

## 3. ACC Project Admin (FR-010 to FR-012)

### Existing Capabilities

| Feature | Status | Notes |
|---------|--------|-------|
| Project creation API | ❌ Not implemented | ACC Admin API supports it, not in raps |
| User import (bulk) | ❌ Not implemented | ACC supports `:import` endpoint |
| Single user add | ✅ Exists | `ProjectUsersClient::add_user()` |

### ACC Project Admin API Research

Based on user-provided information:

```
POST /construction/admin/v1/accounts/{accountId}/projects
```

**Capabilities:**
- Create project from scratch
- Create project from template
- Returns `jobId` (no status endpoint - must poll project)

**Limitations:**
- ACC only (not BIM 360)
- Requires 3-legged OR 2-legged with user impersonation
- Template members NOT auto-assigned

### Decisions

| Tool | Decision | Rationale |
|------|----------|-----------|
| `project_create` | New method in raps-acc | ACC Admin API POST with polling |
| `project_user_add` | Wrap existing `add_user()` | Already implemented |
| `project_users_import` | New method using `:import` endpoint | Bulk efficiency |

### New Methods Required

```rust
// raps-acc/src/lib.rs - new methods needed

/// Create a new ACC project
pub async fn create_project(
    &self,
    account_id: &str,
    name: &str,
    template_project_id: Option<&str>,
    products: Vec<String>,
) -> Result<ProjectCreationJob>

/// Poll project until activated
pub async fn wait_for_project_activation(
    &self,
    account_id: &str,
    project_id: &str,
    timeout: Duration,
) -> Result<Project>

/// Bulk import users to project
pub async fn import_users(
    &self,
    project_id: &str,
    users: Vec<ImportUserRequest>,
) -> Result<ImportUsersResult>
```

---

## 4. Item Management (FR-013 to FR-015)

### Existing Capabilities in raps-dm

| Method | Exists | Notes |
|--------|--------|-------|
| `create_item_from_storage()` | ✅ Yes | Links OSS to folder |
| Item delete | ❌ No | Not implemented |
| Item rename | ❌ No | Not implemented |

### APS Data Management API

```
DELETE /data/v1/projects/{project_id}/items/{item_id}
PATCH /data/v1/projects/{project_id}/items/{item_id}
```

### Decisions

| Tool | Decision | Rationale |
|------|----------|-----------|
| `item_create` | Wrap `create_item_from_storage()` | Existing method |
| `item_delete` | New method in raps-dm | DELETE API call |
| `item_rename` | New method in raps-dm | PATCH with displayName |

### New Methods Required

```rust
// raps-dm/src/lib.rs - new methods needed

/// Delete an item from a project
pub async fn delete_item(
    &self,
    project_id: &str,
    item_id: &str,
) -> Result<()>

/// Rename an item's display name
pub async fn rename_item(
    &self,
    project_id: &str,
    item_id: &str,
    new_display_name: &str,
) -> Result<Item>
```

---

## 5. MCP Implementation Patterns

### Tool Registration Pattern (from server.rs)

```rust
// 1. Async method implementation
async fn tool_name(&self, arg1: String, arg2: Option<i64>) -> String {
    // Get client (lazy-loaded, cached)
    let client = self.get_client().await;

    // Call underlying crate method
    match client.method(&arg1, arg2).await {
        Ok(result) => format!("Success: {}", result),
        Err(e) => format!("Error: {}", e),
    }
}

// 2. Dispatch case in dispatch_tool()
"tool_name" => {
    let arg1 = match Self::required_arg(&args, "arg1") {
        Ok(val) => val,
        Err(err) => return CallToolResult::success(vec![Content::text(err)]),
    };
    let arg2 = Self::optional_arg(&args, "arg2").and_then(|s| s.parse().ok());
    self.tool_name(arg1, arg2).await
}

// 3. Tool schema in get_tools()
Tool::new(
    "tool_name",
    "Description of what this tool does.",
    schema(
        json!({
            "arg1": {"type": "string", "description": "Required argument"},
            "arg2": {"type": "integer", "description": "Optional number"}
        }),
        &["arg1"],  // required fields
    ),
),
```

### Auth Guidance Pattern (from auth_guidance.rs)

```rust
// Add to get_tool_auth_requirement()
"object_upload" | "object_download" | "object_info" | "object_copy"
| "object_upload_batch" | "object_delete_batch" => AuthRequirement::TwoLegged,

"project_info" | "project_users_list" | "folder_contents"
| "project_create" | "project_user_add" | "project_users_import"
| "item_create" | "item_delete" | "item_rename" => AuthRequirement::ThreeLegged,
```

### Batch Operation Pattern

```rust
async fn object_upload_batch(
    &self,
    bucket_key: String,
    file_paths: Vec<String>,
) -> String {
    let client = self.get_oss_client().await;
    let semaphore = Arc::new(Semaphore::new(4));  // 4-way concurrency

    let results: Vec<_> = futures::stream::iter(file_paths)
        .map(|path| {
            let client = client.clone();
            let sem = semaphore.clone();
            let bucket = bucket_key.clone();
            async move {
                let _permit = sem.acquire().await.unwrap();
                let path = Path::new(&path);
                let key = path.file_name()?.to_str()?;
                match client.upload_object(&bucket, key, path).await {
                    Ok(info) => Some((key.to_string(), true, info.size)),
                    Err(e) => Some((path.display().to_string(), false, 0)),
                }
            }
        })
        .buffer_unordered(4)
        .collect()
        .await;

    // Format summary
    let success = results.iter().filter(|r| r.1).count();
    let failed = results.len() - success;
    format!("Uploaded {} files, {} failed\n\n{}", success, failed, details)
}
```

---

## 6. Implementation Summary

### New Crate Methods Required

| Crate | Method | Purpose |
|-------|--------|---------|
| raps-oss | `get_object_details()` | Object metadata via HEAD/GET |
| raps-dm | `delete_item()` | Delete item from project |
| raps-dm | `rename_item()` | Update item display name |
| raps-acc | `create_project()` | ACC project creation |
| raps-acc | `wait_for_project_activation()` | Poll project status |
| raps-acc | `import_users()` | Bulk user import |

### MCP Tools by Category

**Object Operations (6 tools):**
- `object_upload` - Wrap existing with MCP interface
- `object_upload_batch` - New with 4-way semaphore
- `object_download` - Wrap existing
- `object_info` - Needs new crate method
- `object_copy` - Download + Upload pattern
- `object_delete_batch` - Loop with semaphore

**Project Management (3 tools):**
- `project_info` - Combine existing methods
- `project_users_list` - Wrap existing
- `folder_contents` - Wrap existing with pagination

**ACC Project Admin (3 tools):**
- `project_create` - Needs new crate method
- `project_user_add` - Wrap existing
- `project_users_import` - Needs new crate method

**Item Management (3 tools):**
- `item_create` - Wrap existing
- `item_delete` - Needs new crate method
- `item_rename` - Needs new crate method

---

## 7. Alternatives Considered

### Object Copy

| Option | Rejected Because |
|--------|------------------|
| Server-side copy | APS OSS API doesn't support it |
| URN reference only | Doesn't create independent copy |
| **Download + Upload** | **Selected** - Only viable option |

### Batch Operations

| Option | Rejected Because |
|--------|------------------|
| Single sequential loop | Too slow for 10+ files |
| Unlimited concurrency | Risk of rate limiting |
| **4-way semaphore** | **Selected** - Matches CLI default, balanced |

### Project Creation Polling

| Option | Rejected Because |
|--------|------------------|
| Return jobId only | User must poll manually |
| Fixed timeout | May not match project complexity |
| **Poll with configurable timeout** | **Selected** - Better UX, 60s default |

# Data Model: Account Admin Bulk Management Tool

**Date**: 2026-01-16
**Feature**: 001-account-admin-management

## Overview

This document defines the data structures for the bulk user management feature. The model covers entities from both the APS API responses and internal state management.

---

## 1. Core Entities

### Account

Represents an Autodesk account (hub) containing projects and users.

```rust
pub struct Account {
    pub id: String,           // e.g., "b.account-uuid"
    pub name: String,
    pub region: Region,
}

pub enum Region {
    US,
    EMEA,
}
```

**Source**: ACC Account Admin API

---

### User

Represents a user within an Autodesk account.

```rust
pub struct User {
    pub id: String,           // Autodesk user ID
    pub email: String,        // Primary identifier for user input
    pub name: String,
    pub company_id: Option<String>,
    pub status: UserStatus,
}

pub enum UserStatus {
    Active,
    Pending,      // Invited but not accepted
    Inactive,
}
```

**Validation Rules**:
- `email` must be valid email format
- `email` must exist in the account (validated before operations)

---

### Project

Represents an ACC or BIM 360 project.

```rust
pub struct Project {
    pub id: String,           // e.g., "b.project-uuid"
    pub name: String,
    pub platform: Platform,
    pub status: ProjectStatus,
    pub account_id: String,
}

pub enum Platform {
    ACC,         // Autodesk Construction Cloud
    BIM360,      // BIM 360 (legacy)
}

pub enum ProjectStatus {
    Active,
    Inactive,
    Archived,
}
```

**Business Rules**:
- Bulk operations only target `Active` projects by default
- Write operations to BIM 360 projects require legacy API

---

### ProjectMember

Represents a user's membership in a specific project.

```rust
pub struct ProjectMember {
    pub user_id: String,
    pub project_id: String,
    pub roles: Vec<Role>,
    pub access_levels: Vec<AccessLevel>,
    pub added_at: DateTime<Utc>,
}

pub struct Role {
    pub id: String,
    pub name: String,          // e.g., "Project Admin", "Document Manager"
}

pub struct AccessLevel {
    pub product: String,       // e.g., "projectAdministration", "docs"
    pub level: String,         // e.g., "admin", "user"
}
```

**State Transitions**:
- `None` → `Member` (add_user operation)
- `Member` → `None` (remove_user operation)
- `Member(role_a)` → `Member(role_b)` (update_role operation)

---

### FolderPermission

Represents permissions on a folder within a project.

```rust
pub struct FolderPermission {
    pub folder_id: String,
    pub subject: PermissionSubject,
    pub actions: Vec<FolderAction>,
}

pub enum PermissionSubject {
    User(String),              // User ID
    Role(String),              // Role ID
    Company(String),           // Company ID
}

pub enum FolderAction {
    View,
    Download,
    Publish,
    Collaborate,
    Edit,
    Control,
}
```

**Predefined Permission Sets**:

| Level Name | Actions |
|------------|---------|
| ViewOnly | View, Collaborate |
| ViewDownload | View, Download, Collaborate |
| UploadOnly | Publish |
| ViewDownloadUpload | Publish, View, Download, Collaborate |
| ViewDownloadUploadEdit | Publish, View, Download, Collaborate, Edit |
| FolderControl | Publish, View, Download, Collaborate, Edit, Control |

---

## 2. Operation Entities

### BulkOperation

Represents a bulk operation in progress or completed.

```rust
pub struct BulkOperation {
    pub id: Uuid,
    pub operation_type: OperationType,
    pub status: OperationStatus,
    pub parameters: OperationParameters,
    pub progress: OperationProgress,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum OperationType {
    AddUser,
    RemoveUser,
    UpdateRole,
    UpdateFolderRights,
}

pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
    Failed,
}
```

---

### OperationParameters

Parameters for different operation types.

```rust
pub enum OperationParameters {
    AddUser {
        user_email: String,
        role: Option<String>,
        access_levels: Vec<AccessLevel>,
    },
    RemoveUser {
        user_email: String,
    },
    UpdateRole {
        user_email: String,
        new_role: String,
    },
    UpdateFolderRights {
        user_email: String,
        folder_type: FolderType,
        permission_level: PermissionLevel,
    },
}

pub enum FolderType {
    ProjectFiles,
    Plans,
    Custom(String),
}

pub enum PermissionLevel {
    ViewOnly,
    ViewDownload,
    UploadOnly,
    ViewDownloadUpload,
    ViewDownloadUploadEdit,
    FolderControl,
}
```

---

### OperationProgress

Tracks progress of a bulk operation.

```rust
pub struct OperationProgress {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub pending: usize,
}

impl OperationProgress {
    pub fn percentage(&self) -> f64 {
        if self.total == 0 { 0.0 }
        else { (self.completed + self.failed + self.skipped) as f64 / self.total as f64 * 100.0 }
    }

    pub fn is_complete(&self) -> bool {
        self.pending == 0
    }
}
```

---

### ProjectResult

Result of an operation on a single project.

```rust
pub struct ProjectResult {
    pub project_id: String,
    pub status: ResultStatus,
    pub message: Option<String>,
    pub attempts: u32,
    pub completed_at: Option<DateTime<Utc>>,
}

pub enum ResultStatus {
    Success,
    Skipped { reason: SkipReason },
    Failed { error: String },
}

pub enum SkipReason {
    AlreadyExists,
    UserNotInProject,
    ProjectArchived,
    InsufficientPermissions,
}
```

---

## 3. Filter Criteria

### ProjectFilter

Used for selecting target projects.

```rust
pub struct ProjectFilter {
    pub name_pattern: Option<String>,      // Glob pattern
    pub status: Option<ProjectStatus>,      // Default: Active
    pub platform: Option<Platform>,         // ACC, BIM360, or both
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub region: Option<Region>,
    pub include_ids: Option<Vec<String>>,   // Explicit include list
    pub exclude_ids: Option<Vec<String>>,   // Explicit exclude list
}
```

**Filter Application Order**:
1. Apply `status` filter (default: Active)
2. Apply `platform` filter if specified
3. Apply `name_pattern` if specified
4. Apply date range filters
5. Apply region filter
6. Include explicit IDs
7. Exclude explicit IDs

---

## 4. Configuration Entities

### BulkOperationConfig

Configuration for bulk operation execution.

```rust
pub struct BulkOperationConfig {
    pub concurrency: usize,           // Default: 10, Range: 1-50
    pub max_retries: usize,           // Default: 5
    pub retry_base_delay_ms: u64,     // Default: 1000
    pub dry_run: bool,                // Preview without executing
    pub continue_on_error: bool,      // Default: true
    pub export_results: Option<ExportFormat>,
}

pub enum ExportFormat {
    Json,
    Csv,
    Yaml,
}
```

---

## 5. Audit Entities

### AuditRecord

Immutable record of an operation for audit trail.

```rust
pub struct AuditRecord {
    pub id: Uuid,
    pub operation_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub action: AuditAction,
    pub actor_email: String,
    pub target_project_id: String,
    pub target_user_email: String,
    pub details: serde_json::Value,
    pub result: ResultStatus,
}

pub enum AuditAction {
    UserAdded,
    UserRemoved,
    RoleUpdated,
    FolderRightsUpdated,
}
```

**Retention**: Audit records are stored alongside operation state and preserved after completion.

---

## 6. Entity Relationships

```
Account (1) ─────────── (*) Project
    │                       │
    │                       │
    └─── (*) User           │
              │             │
              └─────────────┤
                            │
                   ProjectMember (*) ── (*) Role
                            │
                            │
                   FolderPermission (*)
                            │
                            │
BulkOperation (1) ───── (*) ProjectResult
```

---

## 7. Serialization Formats

All entities support serialization to/from JSON for:
- State persistence (operation state files)
- CLI output (`--output json|yaml|csv`)
- API request/response handling

Derive macros used:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
```

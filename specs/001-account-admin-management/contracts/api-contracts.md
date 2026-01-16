# API Contracts: Account Admin Bulk Management

**Date**: 2026-01-16
**Feature**: 001-account-admin-management

## Overview

This document specifies the internal Rust API contracts for the bulk management feature. These are the public interfaces exposed by the `raps-admin` and extended `raps-acc` crates.

---

## 1. raps-acc Crate Extensions

### AccountAdminClient

New client for Account Admin API operations.

```rust
// raps-acc/src/admin.rs

/// Client for ACC Account Admin API
pub struct AccountAdminClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl AccountAdminClient {
    /// Create a new Account Admin client
    pub fn new(config: Config, auth: AuthClient) -> Self;

    /// Create client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
    ) -> Self;

    /// List all users in an account (paginated)
    pub async fn list_users(
        &self,
        account_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<AccountUser>>;

    /// Search for a user by email
    pub async fn find_user_by_email(
        &self,
        account_id: &str,
        email: &str,
    ) -> Result<Option<AccountUser>>;

    /// List all projects in an account (paginated)
    pub async fn list_projects(
        &self,
        account_id: &str,
        filter: Option<&ProjectFilter>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<AccountProject>>;

    /// Get project details
    pub async fn get_project(
        &self,
        account_id: &str,
        project_id: &str,
    ) -> Result<AccountProject>;
}
```

---

### ProjectUsersClient

New client for Project Users API operations.

```rust
// raps-acc/src/users.rs

/// Client for ACC Project Users API
pub struct ProjectUsersClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl ProjectUsersClient {
    /// Create a new Project Users client
    pub fn new(config: Config, auth: AuthClient) -> Self;

    /// List members of a project
    pub async fn list_project_users(
        &self,
        project_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<PaginatedResponse<ProjectUser>>;

    /// Get a specific project user
    pub async fn get_project_user(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<ProjectUser>;

    /// Add a user to a project
    pub async fn add_user(
        &self,
        project_id: &str,
        request: AddProjectUserRequest,
    ) -> Result<ProjectUser>;

    /// Update a user's role in a project
    pub async fn update_user(
        &self,
        project_id: &str,
        user_id: &str,
        request: UpdateProjectUserRequest,
    ) -> Result<ProjectUser>;

    /// Remove a user from a project
    pub async fn remove_user(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<()>;

    /// Check if a user exists in a project
    pub async fn user_exists(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<bool>;
}

/// Request to add a user to a project
#[derive(Debug, Serialize)]
pub struct AddProjectUserRequest {
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub products: Vec<ProductAccess>,
}

/// Request to update a project user
#[derive(Debug, Serialize)]
pub struct UpdateProjectUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub products: Option<Vec<ProductAccess>>,
}

/// Product access configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct ProductAccess {
    pub key: String,      // e.g., "projectAdministration", "docs"
    pub access: String,   // e.g., "administrator", "member"
}
```

---

### FolderPermissionsClient

Client for folder permission operations.

```rust
// raps-acc/src/permissions.rs

/// Client for folder permissions API
pub struct FolderPermissionsClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl FolderPermissionsClient {
    /// Create a new Folder Permissions client
    pub fn new(config: Config, auth: AuthClient) -> Self;

    /// Get permissions for a folder
    pub async fn get_permissions(
        &self,
        project_id: &str,
        folder_id: &str,
    ) -> Result<Vec<FolderPermission>>;

    /// Create permissions (batch)
    pub async fn create_permissions(
        &self,
        project_id: &str,
        folder_id: &str,
        permissions: Vec<CreatePermissionRequest>,
    ) -> Result<Vec<FolderPermission>>;

    /// Update permissions (batch)
    pub async fn update_permissions(
        &self,
        project_id: &str,
        folder_id: &str,
        permissions: Vec<UpdatePermissionRequest>,
    ) -> Result<Vec<FolderPermission>>;

    /// Delete permissions (batch)
    pub async fn delete_permissions(
        &self,
        project_id: &str,
        folder_id: &str,
        permission_ids: Vec<String>,
    ) -> Result<()>;
}

#[derive(Debug, Serialize)]
pub struct CreatePermissionRequest {
    pub subject_id: String,
    pub subject_type: SubjectType,
    pub actions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdatePermissionRequest {
    pub id: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubjectType {
    User,
    Role,
    Company,
}
```

---

## 2. raps-admin Crate

### BulkExecutor

Core orchestration engine for bulk operations.

```rust
// raps-admin/src/bulk/executor.rs

/// Configuration for bulk execution
#[derive(Debug, Clone)]
pub struct BulkConfig {
    pub concurrency: usize,
    pub max_retries: usize,
    pub retry_base_delay: Duration,
    pub continue_on_error: bool,
}

impl Default for BulkConfig {
    fn default() -> Self {
        Self {
            concurrency: 10,
            max_retries: 5,
            retry_base_delay: Duration::from_secs(1),
            continue_on_error: true,
        }
    }
}

/// Bulk operation executor
pub struct BulkExecutor {
    config: BulkConfig,
    state_manager: StateManager,
}

impl BulkExecutor {
    /// Create a new executor
    pub fn new(config: BulkConfig) -> Self;

    /// Execute a bulk operation with progress callback
    pub async fn execute<T, F, Fut>(
        &self,
        operation_id: Uuid,
        items: Vec<T>,
        processor: F,
        on_progress: impl Fn(ProgressUpdate) + Send + Sync,
    ) -> Result<BulkOperationResult>
    where
        T: Send + Sync,
        F: Fn(T) -> Fut + Send + Sync,
        Fut: Future<Output = Result<ItemResult>> + Send;

    /// Resume an interrupted operation
    pub async fn resume<T, F, Fut>(
        &self,
        operation_id: Uuid,
        processor: F,
        on_progress: impl Fn(ProgressUpdate) + Send + Sync,
    ) -> Result<BulkOperationResult>
    where
        T: Send + Sync + DeserializeOwned,
        F: Fn(T) -> Fut + Send + Sync,
        Fut: Future<Output = Result<ItemResult>> + Send;

    /// Cancel an in-progress operation
    pub async fn cancel(&self, operation_id: Uuid) -> Result<()>;
}

/// Progress update for callbacks
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub current_item: Option<String>,
    pub estimated_remaining: Option<Duration>,
}

/// Result of processing a single item
#[derive(Debug, Clone)]
pub enum ItemResult {
    Success,
    Skipped { reason: String },
    Failed { error: String, retryable: bool },
}

/// Final result of bulk operation
#[derive(Debug)]
pub struct BulkOperationResult {
    pub operation_id: Uuid,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration: Duration,
    pub details: Vec<ItemDetail>,
}
```

---

### StateManager

Manages operation state persistence.

```rust
// raps-admin/src/bulk/state.rs

/// Manages persistent state for bulk operations
pub struct StateManager {
    state_dir: PathBuf,
}

impl StateManager {
    /// Create a new state manager
    pub fn new() -> Result<Self>;

    /// Create a new operation state
    pub async fn create_operation(
        &self,
        operation_type: OperationType,
        parameters: serde_json::Value,
        project_ids: Vec<String>,
    ) -> Result<Uuid>;

    /// Load an existing operation state
    pub async fn load_operation(&self, operation_id: Uuid) -> Result<OperationState>;

    /// Update operation state
    pub async fn update_state(
        &self,
        operation_id: Uuid,
        update: StateUpdate,
    ) -> Result<()>;

    /// Mark operation as complete
    pub async fn complete_operation(
        &self,
        operation_id: Uuid,
        result: BulkOperationResult,
    ) -> Result<()>;

    /// List all operations (optionally filter by status)
    pub async fn list_operations(
        &self,
        status: Option<OperationStatus>,
    ) -> Result<Vec<OperationSummary>>;

    /// Get the most recent incomplete operation
    pub async fn get_resumable_operation(&self) -> Result<Option<Uuid>>;
}

/// State update types
pub enum StateUpdate {
    ItemCompleted { project_id: String, result: ItemResult },
    StatusChanged { status: OperationStatus },
    ProgressUpdated { progress: ProgressUpdate },
}
```

---

### Bulk Operations

High-level operation implementations.

```rust
// raps-admin/src/operations/mod.rs

/// Add user to multiple projects
pub async fn bulk_add_user(
    admin_client: &AccountAdminClient,
    users_client: &ProjectUsersClient,
    account_id: &str,
    user_email: &str,
    role_id: Option<&str>,
    project_filter: &ProjectFilter,
    config: BulkConfig,
    on_progress: impl Fn(ProgressUpdate) + Send + Sync,
) -> Result<BulkOperationResult>;

/// Remove user from multiple projects
pub async fn bulk_remove_user(
    admin_client: &AccountAdminClient,
    users_client: &ProjectUsersClient,
    account_id: &str,
    user_email: &str,
    project_filter: &ProjectFilter,
    config: BulkConfig,
    on_progress: impl Fn(ProgressUpdate) + Send + Sync,
) -> Result<BulkOperationResult>;

/// Update user role across multiple projects
pub async fn bulk_update_role(
    admin_client: &AccountAdminClient,
    users_client: &ProjectUsersClient,
    account_id: &str,
    user_email: &str,
    new_role_id: &str,
    from_role_id: Option<&str>,
    project_filter: &ProjectFilter,
    config: BulkConfig,
    on_progress: impl Fn(ProgressUpdate) + Send + Sync,
) -> Result<BulkOperationResult>;

/// Update folder permissions across multiple projects
pub async fn bulk_update_folder_rights(
    admin_client: &AccountAdminClient,
    permissions_client: &FolderPermissionsClient,
    account_id: &str,
    user_email: &str,
    folder_type: FolderType,
    permission_level: PermissionLevel,
    project_filter: &ProjectFilter,
    config: BulkConfig,
    on_progress: impl Fn(ProgressUpdate) + Send + Sync,
) -> Result<BulkOperationResult>;
```

---

### Project Filter

Filter for selecting target projects.

```rust
// raps-admin/src/filter.rs

/// Filter criteria for selecting projects
#[derive(Debug, Clone, Default)]
pub struct ProjectFilter {
    pub name_pattern: Option<String>,
    pub status: Option<ProjectStatus>,
    pub platform: Option<Platform>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub region: Option<Region>,
    pub include_ids: Option<Vec<String>>,
    pub exclude_ids: Option<Vec<String>>,
}

impl ProjectFilter {
    /// Create a new empty filter
    pub fn new() -> Self;

    /// Parse filter from string expression
    pub fn from_expression(expr: &str) -> Result<Self>;

    /// Check if a project matches the filter
    pub fn matches(&self, project: &AccountProject) -> bool;

    /// Apply filter to a list of projects
    pub fn apply(&self, projects: Vec<AccountProject>) -> Vec<AccountProject>;
}
```

---

## 3. Response Types

### Paginated Response

Standard pagination wrapper.

```rust
/// Paginated API response
#[derive(Debug, Deserialize)]
pub struct PaginatedResponse<T> {
    pub results: Vec<T>,
    pub pagination: Pagination,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub limit: usize,
    pub offset: usize,
    pub total_results: usize,
}

impl<T> PaginatedResponse<T> {
    /// Check if there are more pages
    pub fn has_more(&self) -> bool {
        self.pagination.offset + self.results.len() < self.pagination.total_results
    }

    /// Get offset for next page
    pub fn next_offset(&self) -> usize {
        self.pagination.offset + self.pagination.limit
    }
}
```

---

## 4. Error Types

```rust
// raps-admin/src/error.rs

/// Errors from bulk operations
#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error("User not found in account: {email}")]
    UserNotFound { email: String },

    #[error("Invalid filter expression: {message}")]
    InvalidFilter { message: String },

    #[error("Operation not found: {id}")]
    OperationNotFound { id: Uuid },

    #[error("Operation cannot be resumed (status: {status})")]
    CannotResume { status: String },

    #[error("Rate limit exceeded, retry after {retry_after} seconds")]
    RateLimited { retry_after: u64 },

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("State persistence error: {0}")]
    StateError(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

---

## 5. Exit Codes

```rust
// raps-admin/src/exit.rs

/// Standard exit codes for bulk operations
pub enum ExitCode {
    Success = 0,           // All items processed successfully
    PartialSuccess = 1,    // Some items failed
    Failure = 2,           // Operation could not start
    Cancelled = 3,         // User cancelled
}
```

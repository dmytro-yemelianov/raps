# Research: Account Admin Bulk Management Tool

**Date**: 2026-01-16
**Feature**: 001-account-admin-management

## Executive Summary

This research documents the APS APIs required for bulk user management across ACC/BIM 360 projects. Key findings include API endpoint availability, authentication requirements, rate limits, and pagination patterns.

---

## 1. Account Admin API

### Decision: Use ACC Account Admin API v1

**Rationale**: The ACC Account Admin API provides comprehensive user management at the account level, which is required for validating users before bulk operations.

**Base URL**: `https://developer.api.autodesk.com/construction/admin/v1/accounts/{accountId}`

### Key Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/users` | GET | List all users in an account (paginated) |
| `/users/{userId}` | GET | Get specific user details |
| `/users/search` | POST | Search users by email address |
| `/projects` | GET | List all projects in an account |

### Authentication Requirements
- 3-legged OAuth token (user must be account admin)
- 2-legged OAuth with user impersonation (`x-user-id` header)
- Requires `account:read` scope for read operations

**Alternatives Considered**:
- BIM 360 Admin API (v1) - Deprecated in favor of ACC Admin API
- HQ API - Lower-level, less convenient for user management

---

## 2. Project Users API

### Decision: Use ACC Project Admin API for User Management

**Rationale**: The Project Admin API provides CRUD operations for project members with role assignment capabilities.

**Base URL**: `https://developer.api.autodesk.com/construction/admin/v1/projects/{projectId}`

### Key Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/users` | GET | List project members |
| `/users` | POST | Add single user to project |
| `/users:import` | POST | Bulk import users (async job) |
| `/users/{userId}` | PATCH | Update user role |
| `/users/{userId}` | DELETE | Remove user from project |

### Important Limitations

| Platform | Read (GET) | Write (POST/PATCH/DELETE) |
|----------|------------|---------------------------|
| ACC | ✅ Supported | ✅ Supported |
| BIM 360 | ✅ Supported | ❌ Not Supported |

**Critical**: Write operations only work for ACC projects. For BIM 360 projects, we must use the legacy BIM 360 Admin API.

### BIM 360 Admin API (for BIM 360 projects only)

**Base URL**: `https://developer.api.autodesk.com/bim360/admin/v1`

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/projects/{projectId}/users` | POST | Add user to BIM 360 project |
| `/projects/{projectId}/users/{userId}` | PATCH | Update user in BIM 360 project |
| `/projects/{projectId}/users/{userId}` | DELETE | Remove user from BIM 360 project |

**Alternatives Considered**:
- Using only ACC API - Would not support BIM 360 projects (excluded per FR-001 requirement)
- Using only BIM 360 API - ACC projects would not be supported

---

## 3. Folder Permissions API

### Decision: Use ACC Document Management Permissions API

**Rationale**: Provides batch operations for folder-level permissions, which is more efficient for bulk updates.

**Base URL**: `https://developer.api.autodesk.com/construction/admin/v1/projects/{project_id}/folders/{folder_id}`

### Key Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/permissions` | GET | List folder permissions |
| `/permissions:batch-create` | POST | Create permissions in bulk |
| `/permissions:batch-update` | POST | Update permissions in bulk |
| `/permissions:batch-delete` | POST | Delete permissions in bulk |

### Permission Levels (UI to API Mapping)

| UI Level | API Actions Array |
|----------|------------------|
| View Only | `["VIEW", "COLLABORATE"]` |
| View/Download | `["VIEW", "DOWNLOAD", "COLLABORATE"]` |
| Upload Only | `["PUBLISH"]` |
| View/Download+Upload | `["PUBLISH", "VIEW", "DOWNLOAD", "COLLABORATE"]` |
| View/Download+Upload+Edit | `["PUBLISH", "VIEW", "DOWNLOAD", "COLLABORATE", "EDIT"]` |
| Folder Control | `["PUBLISH", "VIEW", "DOWNLOAD", "COLLABORATE", "EDIT", "CONTROL"]` |

### Subject Types for Permissions
- `USER` - Individual user by ID
- `ROLE` - Role-based (applies to all users with that role)
- `COMPANY` - Company-based (applies to all users in company)

**Alternatives Considered**:
- Individual permission updates - Less efficient for bulk operations
- Using Data Management API v2 - Does not provide permission management

---

## 4. Rate Limits and Throttling

### Decision: Implement Configurable Concurrency with Adaptive Throttling

**Rationale**: APS rate limits vary by endpoint and account tier. A configurable approach with adaptive backoff provides flexibility.

### Known Rate Limits

| API Category | Estimated Rate Limit |
|--------------|---------------------|
| Account Admin API | ~600 requests/minute |
| Project Users API | ~600 requests/minute |
| Folder Permissions API | ~600 requests/minute |

### Rate Limit Headers to Monitor

```
X-RateLimit-Limit: 600
X-RateLimit-Remaining: 595
X-RateLimit-Reset: 1705401600
Retry-After: 25 (only on 429 responses)
```

### Implementation Strategy

1. **Default Concurrency**: 10 parallel requests
2. **Configurable Range**: 1-50 parallel requests
3. **Adaptive Throttling**: Reduce concurrency when `X-RateLimit-Remaining < 50`
4. **Exponential Backoff**: On 429 responses, wait `Retry-After` or `2^attempt * 1s`
5. **Maximum Retries**: 5 attempts per project

### Performance Calculations

With 10 concurrent requests at 600 req/min rate limit:
- **3,000 projects**: ~5 minutes (optimal), ~15 minutes (with failures/retries)
- **5,000 projects**: ~8 minutes (optimal), ~25 minutes (with failures/retries)

The spec requirement of 30 minutes for 3,000 projects is achievable with comfortable margin.

**Alternatives Considered**:
- Fixed rate limiting - Less adaptive to actual API behavior
- No concurrency (sequential) - Too slow for scale requirements

---

## 5. Pagination Patterns

### Decision: Use Offset-Based Pagination with Increased Page Size

**Rationale**: APS APIs use offset-based pagination. Increasing page size reduces total API calls.

### Standard Parameters

| Parameter | Default | Maximum | Description |
|-----------|---------|---------|-------------|
| `limit` | 20 | 200 | Items per page |
| `offset` | 0 | - | Starting index |

### Response Structure

```json
{
  "pagination": {
    "limit": 200,
    "offset": 0,
    "totalResults": 5000
  },
  "results": [...]
}
```

### Implementation Pattern (Rust)

```rust
async fn paginate_all<T>(
    fetch_page: impl Fn(usize, usize) -> Future<Output = Result<PaginatedResponse<T>>>
) -> Result<Vec<T>> {
    let mut all_items = Vec::new();
    let mut offset = 0;
    let limit = 200; // Maximum allowed

    loop {
        let response = fetch_page(offset, limit).await?;
        all_items.extend(response.results);

        if offset + response.results.len() >= response.pagination.total_results {
            break;
        }
        offset += limit;
    }

    Ok(all_items)
}
```

**Alternatives Considered**:
- Cursor-based pagination - Not supported by APS APIs
- Smaller page sizes - More API calls, slower performance

---

## 6. State Persistence for Resume

### Decision: JSON File in User Config Directory

**Rationale**: Simple, portable, no external dependencies. Aligns with existing RAPS patterns using `directories` crate.

### State File Location

```
Windows: %APPDATA%\raps\operations\{operation_id}.json
macOS:   ~/Library/Application Support/raps/operations/{operation_id}.json
Linux:   ~/.local/share/raps/operations/{operation_id}.json
```

### State File Structure

```json
{
  "operation_id": "uuid-v4",
  "operation_type": "add_user",
  "created_at": "2026-01-16T10:00:00Z",
  "updated_at": "2026-01-16T10:15:00Z",
  "status": "in_progress",
  "parameters": {
    "user_email": "newuser@example.com",
    "role": "Project Admin",
    "account_id": "acc-123"
  },
  "total_projects": 3000,
  "completed": 1500,
  "failed": 5,
  "pending": 1495,
  "results": {
    "project-id-1": { "status": "success", "completed_at": "..." },
    "project-id-2": { "status": "failed", "error": "...", "attempts": 3 },
    "project-id-3": { "status": "skipped", "reason": "already_exists" }
  }
}
```

**Alternatives Considered**:
- SQLite database - Overkill for single-user CLI tool
- Server-side storage - Adds external dependency, not aligned with CLI-first design
- In-memory only - Loses state on interruption

---

## 7. User Identification by Email

### Decision: Validate Email Against Account Users Before Operations

**Rationale**: Email is the most user-friendly identifier. Pre-validation prevents wasted API calls.

### Implementation Flow

1. User provides email address
2. Call `POST /accounts/{accountId}/users/search` with email
3. If found, extract `userId` for subsequent operations
4. If not found, return error with clear message

### Search Request

```json
POST /construction/admin/v1/accounts/{accountId}/users/search
{
  "email": "user@example.com"
}
```

### Response Handling

- **Found**: Extract `userId`, proceed with operations
- **Not Found**: Error: "User not found in account. Verify email or invite user first."

**Alternatives Considered**:
- Accept Autodesk ID directly - Less user-friendly
- Search by name - Ambiguous, may return multiple results

---

## 8. Duplicate Handling Strategy

### Decision: Skip Existing, Report as "Already Exists"

**Rationale**: Prevents errors, allows idempotent operations, provides clear reporting.

### Implementation Logic

```rust
match add_user_to_project(project_id, user_id, role).await {
    Ok(_) => OperationResult::Success,
    Err(e) if e.status() == 409 => OperationResult::Skipped("already_exists"),
    Err(e) if e.status() == 429 => retry_with_backoff(...),
    Err(e) => OperationResult::Failed(e.to_string()),
}
```

### Result Categories

| Category | Description | Count Towards |
|----------|-------------|---------------|
| `success` | User added/updated successfully | Completed |
| `skipped` | User already exists (add) or not found (remove) | Completed |
| `failed` | API error after retries exhausted | Failed |

**Alternatives Considered**:
- Fail on duplicates - Breaks idempotency, poor UX
- Overwrite existing - May cause unintended role changes

---

## Sources

- [ACC Admin API Field Guide](https://aps.autodesk.com/en/docs/acc/v1/overview/field-guide/admin/)
- [ACC Project Admin API Blog](https://aps.autodesk.com/blog/acc-project-admin-api-project-creation-and-user-management)
- [ACC Admin API Reference](https://aps.autodesk.com/en/docs/acc/v1/reference/http/)
- [Folder Permission API](https://aps.autodesk.com/blog/folder-permission-api-bim-360-released)
- [APS Rate Limits Best Practices](https://aps.autodesk.com/blog/autodesk-platform-services-aps-api-rate-limits-best-practices-developers)
- [ACC Rate Limits Documentation](https://aps.autodesk.com/en/docs/acc/v1/overview/rate-limits)

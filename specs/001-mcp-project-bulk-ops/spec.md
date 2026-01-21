# Feature Specification: MCP Project Management and Bulk Operations

**Feature Branch**: `001-mcp-project-bulk-ops`
**Created**: 2026-01-18
**Status**: Draft
**Input**: User description: "in raps to mcp add project management, including bulk, and other operations on objects that are reasonable but missing"

## Clarifications

### Session 2026-01-18

- Q: What is the maximum number of parallel uploads for batch operations? → A: 4 parallel uploads (balanced, matches CLI default)
- Q: When copying an object to a bucket where it already exists, what should happen? → A: Skip with warning (return existing object info with warning, non-destructive)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - AI Assistant Lists Project Details (Priority: P1)

An AI assistant (Claude, Cursor, etc.) connected via MCP needs to help a user explore their Autodesk Construction Cloud projects. The assistant should be able to retrieve project details including top folders, members, and settings without the user having to leave the conversation.

**Why this priority**: Project exploration is the foundation for all other project-related operations. Users need to understand what they're working with before performing any actions.

**Independent Test**: Can be fully tested by asking the AI to "show me details about project X" and receiving structured project information including folders and metadata.

**Acceptance Scenarios**:

1. **Given** a user is connected with valid 3-legged auth, **When** the AI calls `project_info` with a hub_id and project_id, **Then** it receives project name, type, scopes, and top-level folders.
2. **Given** an invalid project_id is provided, **When** the AI calls `project_info`, **Then** it receives a clear error message indicating the project was not found.

---

### User Story 2 - AI Assistant Performs Batch Object Operations (Priority: P1)

An AI assistant needs to help users upload multiple files to a bucket efficiently. Instead of making individual upload requests, the assistant should be able to batch operations for better performance and user experience.

**Why this priority**: Bulk operations significantly improve efficiency when working with multiple files, which is a common real-world scenario.

**Independent Test**: Can be tested by asking the AI to "upload these 5 files to my bucket" and verifying all files are uploaded with a summary of results.

**Acceptance Scenarios**:

1. **Given** multiple valid file paths, **When** the AI calls `object_upload_batch`, **Then** all files are uploaded and a summary showing success/failure for each file is returned.
2. **Given** a mix of valid and invalid file paths, **When** the AI calls `object_upload_batch`, **Then** valid files are uploaded, invalid files are reported as failed, and the operation continues for remaining files.

---

### User Story 3 - AI Assistant Copies Objects Between Buckets (Priority: P2)

An AI assistant needs to help users reorganize their object storage by copying objects between buckets without downloading and re-uploading.

**Why this priority**: Copy operations are common for reorganization and backup workflows, but can be done manually with upload/download as a workaround.

**Independent Test**: Can be tested by asking the AI to "copy model.rvt from bucket-a to bucket-b" and verifying the object exists in both buckets.

**Acceptance Scenarios**:

1. **Given** a source bucket/object and destination bucket, **When** the AI calls `object_copy`, **Then** the object is copied and the new object details are returned.
2. **Given** a non-existent source object, **When** the AI calls `object_copy`, **Then** a clear error is returned without affecting the destination bucket.

---

### User Story 4 - AI Assistant Retrieves Object Metadata (Priority: P2)

An AI assistant needs to retrieve detailed information about a specific object without downloading it, including size, content type, and timestamps.

**Why this priority**: Metadata inspection is essential for understanding objects before performing operations, but object list provides basic info.

**Independent Test**: Can be tested by asking the AI to "show me details about model.rvt in my bucket" and receiving metadata.

**Acceptance Scenarios**:

1. **Given** a valid bucket and object key, **When** the AI calls `object_info`, **Then** detailed metadata including size, content type, creation date, and SHA1 hash is returned.
2. **Given** a non-existent object, **When** the AI calls `object_info`, **Then** a clear "not found" error is returned.

---

### User Story 5 - AI Assistant Manages Folder Contents (Priority: P2)

An AI assistant needs to help users navigate and manage folders within projects, including listing contents with pagination and creating new folders.

**Why this priority**: Enhanced folder operations improve navigation in large projects with many items.

**Independent Test**: Can be tested by asking the AI to "list all items in the Plans folder" and receiving paginated results.

**Acceptance Scenarios**:

1. **Given** a valid folder_id, **When** the AI calls `folder_contents` with pagination, **Then** items and subfolders are returned with pagination metadata.
2. **Given** a folder with many items, **When** pagination is used, **Then** subsequent pages return different items with correct offset handling.

---

### User Story 6 - AI Assistant Creates Items in Project Folders (Priority: P3)

An AI assistant needs to help users create new items (files) in project folders from OSS objects, linking storage with Data Management.

**Why this priority**: Creating items is important for full workflow support, but requires understanding of the two-step process (OSS upload + item creation).

**Independent Test**: Can be tested by asking the AI to "add model.rvt to my project's Models folder" and verifying the item appears in the project.

**Acceptance Scenarios**:

1. **Given** a valid folder_id and OSS object URN, **When** the AI calls `item_create`, **Then** a new item is created in the folder and item details are returned.
2. **Given** an invalid folder_id, **When** the AI calls `item_create`, **Then** a clear error is returned without orphaning the OSS object.

---

### User Story 7 - AI Assistant Uploads Single File (Priority: P1)

An AI assistant needs to upload a single file to a bucket. This is the fundamental building block for object storage operations.

**Why this priority**: Single file upload is the most basic and frequently used object operation - essential for any storage workflow.

**Independent Test**: Can be tested by asking the AI to "upload model.rvt to my-bucket" and verifying the object exists.

**Acceptance Scenarios**:

1. **Given** a valid file path and bucket, **When** the AI calls `object_upload`, **Then** the file is uploaded and object details (key, size, SHA1) are returned.
2. **Given** a non-existent file path, **When** the AI calls `object_upload`, **Then** a clear error is returned indicating file not found.
3. **Given** a large file (>100MB), **When** the AI calls `object_upload`, **Then** resumable upload is used automatically and progress summary is provided.

---

### User Story 8 - AI Assistant Downloads Object (Priority: P2)

An AI assistant needs to download an object from a bucket to a local file path, enabling complete object retrieval workflows.

**Why this priority**: Download completes the CRUD cycle for objects. Currently only signed URLs are available, which requires user to manually download.

**Independent Test**: Can be tested by asking the AI to "download model.rvt from my-bucket to /tmp/model.rvt" and verifying the file exists locally.

**Acceptance Scenarios**:

1. **Given** a valid bucket/object and destination path, **When** the AI calls `object_download`, **Then** the file is downloaded and file path with size is returned.
2. **Given** a non-existent object, **When** the AI calls `object_download`, **Then** a clear "not found" error is returned.
3. **Given** a destination path that is not writable, **When** the AI calls `object_download`, **Then** a permission error is returned before starting download.

---

### User Story 9 - AI Assistant Bulk Deletes Objects (Priority: P2)

An AI assistant needs to delete multiple objects from a bucket efficiently, useful for cleanup operations and storage management.

**Why this priority**: Bulk delete significantly improves efficiency when cleaning up multiple files, complementing bulk upload.

**Independent Test**: Can be tested by asking the AI to "delete all .tmp files from my-bucket" and verifying objects are removed.

**Acceptance Scenarios**:

1. **Given** multiple valid object keys, **When** the AI calls `object_delete_batch`, **Then** all objects are deleted and a summary showing success/failure for each is returned.
2. **Given** a mix of existing and non-existent objects, **When** the AI calls `object_delete_batch`, **Then** existing objects are deleted, non-existent are reported as skipped, and operation continues.

---

### User Story 10 - AI Assistant Creates ACC Project (Priority: P2)

An AI assistant needs to help users create new ACC projects, either from scratch or by cloning a template project.

**Why this priority**: Project creation is a key administrative operation for setting up new construction projects.

**Independent Test**: Can be tested by asking the AI to "create a new project called Test Project in my ACC account" and verifying the project is created.

**Acceptance Scenarios**:

1. **Given** valid account_id and project name, **When** the AI calls `project_create` with from-scratch mode, **Then** a new project is created and project details are returned after polling for activation.
2. **Given** a template project name, **When** the AI calls `project_create` with template mode, **Then** a new project is cloned from the template (note: template members are NOT auto-assigned).
3. **Given** an invalid account_id, **When** the AI calls `project_create`, **Then** a clear authorization error is returned.

---

### User Story 11 - AI Assistant Adds Users to Project (Priority: P2)

An AI assistant needs to help administrators add users to ACC projects with appropriate roles.

**Why this priority**: User management is essential for project administration and access control.

**Independent Test**: Can be tested by asking the AI to "add user@example.com as project admin to Project X" and verifying user access.

**Acceptance Scenarios**:

1. **Given** valid project_id and user email, **When** the AI calls `project_user_add`, **Then** the user is added with specified role and confirmation is returned.
2. **Given** a user already in the project, **When** the AI calls `project_user_add`, **Then** the existing membership is returned with a note that user already exists.

---

### User Story 12 - AI Assistant Bulk Imports Users to Project (Priority: P3)

An AI assistant needs to help administrators add multiple users to a project at once.

**Why this priority**: Bulk user import is valuable for large project setups but can be done individually as workaround.

**Independent Test**: Can be tested by asking the AI to "add these 5 users to Project X" and verifying all users have access.

**Acceptance Scenarios**:

1. **Given** multiple user emails and roles, **When** the AI calls `project_users_import`, **Then** all users are added and a summary is returned.
2. **Given** a mix of valid and invalid emails, **When** the AI calls `project_users_import`, **Then** valid users are added, invalid are reported, operation continues.

---

### Edge Cases

- What happens when batch operations exceed rate limits? (Partial success with retry guidance returned)
- How does the system handle concurrent operations on the same object? (Last-write-wins behavior, tool returns latest state)
- What happens when copying an object to a bucket where it already exists? (Returns existing object info with warning)
- How are large folder listings handled? (Automatic pagination with configurable page size, default 50 items)
- How does project creation handle the asynchronous job? (Tool polls project status until activated, returns final state)
- What happens when cloning a project from template? (Template members are NOT auto-assigned - differs from UI behavior, tool returns warning)
- What happens when downloading to a path that already has a file? (Overwrite with warning, matching typical download behavior)
- How does upload handle files larger than 100MB? (Automatic chunked/resumable upload, no user configuration needed)

## Requirements *(mandatory)*

### Functional Requirements

**Object Operations (Complete CRUD):**
- **FR-001**: MCP server MUST provide an `object_upload` tool that uploads a single file to a bucket with automatic chunked upload for large files
- **FR-002**: MCP server MUST provide an `object_upload_batch` tool that accepts multiple file paths and uploads them with 4-way concurrency
- **FR-003**: MCP server MUST provide an `object_download` tool that downloads an object to a local file path
- **FR-004**: MCP server MUST provide an `object_info` tool that returns detailed metadata for a specific object
- **FR-005**: MCP server MUST provide an `object_copy` tool that copies an object from one bucket to another (skip with warning if exists)
- **FR-006**: MCP server MUST provide an `object_delete_batch` tool that deletes multiple objects with summary reporting

**Project Management Tools:**
- **FR-007**: MCP server MUST provide a `project_info` tool that returns project details including name, type, scopes, and top-level folders
- **FR-008**: MCP server MUST provide a `project_users_list` tool that lists users with access to a specific project
- **FR-009**: MCP server MUST provide a `folder_contents` tool that lists all items and subfolders within a folder with pagination support

**ACC Project Admin (ACC only, requires 3-legged auth or 2-legged with user impersonation):**
- **FR-010**: MCP server MUST provide a `project_create` tool that creates ACC projects from scratch or from template, polling until activated
- **FR-011**: MCP server MUST provide a `project_user_add` tool that adds a user to an ACC project with a specified role
- **FR-012**: MCP server MUST provide a `project_users_import` tool that bulk imports multiple users to an ACC project

**Item Management:**
- **FR-013**: MCP server MUST provide an `item_create` tool that creates a new item in a folder from an OSS storage object
- **FR-014**: MCP server MUST provide an `item_delete` tool that removes an item from a folder
- **FR-015**: MCP server MUST provide an `item_rename` tool that updates an item's display name

**Error Handling:**
- **FR-016**: All new tools MUST return structured error responses with error code, message, and suggested remediation
- **FR-017**: Batch operations MUST continue processing remaining items when individual items fail and return a summary

### Key Entities

- **Project**: An ACC/BIM 360 project containing folders, items, and users. Has name, type, scopes, and associated hub.
- **Folder**: A container within a project that holds items and subfolders. Has name, display name, and parent reference.
- **Item**: A file reference within a project folder, linked to OSS storage. Has name, display name, version history, and storage URN.
- **Object**: A file stored in OSS bucket. Has object key, size, content type, SHA1 hash, and timestamps.
- **Batch Result**: Summary of a bulk operation including success count, failure count, and per-item details.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: AI assistants can retrieve complete project information in a single tool call (no need for multiple sequential calls)
- **SC-002**: Batch uploads of 10 files complete within 30 seconds under normal network conditions
- **SC-003**: All new MCP tools return responses within 5 seconds for single-item operations
- **SC-004**: Batch operations report individual item status, enabling AI to communicate specific successes/failures to users
- **SC-005**: 100% of error cases include actionable guidance that the AI can relay to users
- **SC-006**: Folder listings support pagination, allowing navigation of folders with 1000+ items
- **SC-007**: Copy operations preserve object metadata including content type and custom attributes
- **SC-008**: Single file upload supports files up to 5GB with automatic chunking for files over 100MB
- **SC-009**: Project creation returns activated project within 60 seconds of initial request (polling included)
- **SC-010**: Bulk delete of 50 objects completes within 15 seconds under normal network conditions

## Assumptions

- Users have already authenticated via 3-legged OAuth for project operations (handled by existing auth flow)
- Project creation requires 3-legged auth OR 2-legged with user impersonation (pure 2-legged doesn't work)
- Project creation is ACC-only; these endpoints don't work for BIM 360
- ACC Project Admin API returns jobId but has no status endpoint; must poll project until activated
- Template cloning does NOT auto-assign template members (differs from ACC UI behavior)
- MCP clients (Claude, Cursor) will present results appropriately to end users
- File paths provided for batch uploads are accessible from the system where RAPS MCP server runs
- Rate limits are managed by the existing HTTP client with exponential backoff
- The OSS copy operation uses server-side copy functionality when available

## Out of Scope

- File upload from URLs (only local file paths supported)
- Recursive folder operations (copy entire folder trees)
- Real-time progress updates for batch operations (summary provided on completion)
- Version rollback operations
- Webhook management via MCP (already has separate CLI commands)
- Project update/delete operations (not available via APS API)
- BIM 360 project creation (ACC only - BIM 360 uses different admin workflows)

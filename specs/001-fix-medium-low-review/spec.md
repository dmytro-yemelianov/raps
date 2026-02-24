# Feature Specification: Fix MEDIUM and LOW Severity Review Findings

**Feature Branch**: `001-fix-medium-low-review`
**Created**: 2026-02-24
**Status**: Draft
**Input**: User description: "Fix MEDIUM and LOW severity issues found in the RAPS codebase deep review — 6 MEDIUM and 5 LOW findings across 7 workspace crates"

## Clarifications

### Session 2026-02-24

- Q: Should streaming photo uploads (finding #7) be in scope? → A: Excluded — only file-size display improvements are in scope. Streaming upload deferred to a future feature.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Safe Polling with Timeouts (Priority: P1)

Users running long-running translation or photogrammetry operations via the CLI can get stuck indefinitely if the remote service becomes unresponsive. Polling loops that wait for completion must have configurable timeouts so users are never left waiting forever.

**Why this priority**: Infinite polling directly impacts user experience — a stuck terminal requires manual termination, and the user receives no actionable feedback about what happened.

**Independent Test**: Can be tested by initiating a polling operation and verifying it terminates with a clear message after the timeout period, rather than running indefinitely.

**Acceptance Scenarios**:

1. **Given** a photogrammetry job is submitted, **When** the processing service does not complete within the timeout period, **Then** the CLI exits the polling loop with a clear timeout message and the photoscene ID for later manual check.
2. **Given** a model translation is submitted with `--wait`, **When** the service becomes unresponsive, **Then** the CLI times out and reports the translation URN so the user can check status later.
3. **Given** a polling timeout occurs, **When** the user checks the operation status manually afterward, **Then** the original operation is unaffected (timeout is client-side only).

---

### User Story 2 - Model Derivative Metadata and Properties Access (Priority: P1)

Users need to retrieve model metadata, object trees, and property data from translated models. These are core Model Derivative read operations that are currently unavailable, forcing users to rely on external tools or direct API calls.

**Why this priority**: Metadata and properties are the primary reason users translate models — without these endpoints, the translation workflow is incomplete.

**Independent Test**: Can be tested by translating a model, then querying its metadata, object tree, and properties through the CLI, verifying structured output is returned.

**Acceptance Scenarios**:

1. **Given** a model has been translated, **When** the user requests metadata for that model, **Then** the CLI returns the model's metadata including GUID and available views/viewables.
2. **Given** a translated model with an object tree, **When** the user requests the object tree, **Then** the CLI returns the hierarchical object structure.
3. **Given** a translated model, **When** the user requests properties for a specific object or viewable, **Then** the CLI returns the properties data in the requested output format.
4. **Given** a model that has not been translated, **When** the user requests metadata, **Then** the CLI returns a clear error indicating the model must be translated first.

---

### User Story 3 - OSS Batch Operations (Priority: P2)

Users managing many objects in cloud storage need batch operations (copy, rename, and batch-upload capabilities) to avoid repetitive individual commands when working with large numbers of files.

**Why this priority**: Batch operations improve efficiency for power users and automation scenarios, but individual object operations remain functional as a workaround.

**Independent Test**: Can be tested by performing batch copy or rename operations on multiple objects in a bucket and verifying all objects are affected correctly.

**Acceptance Scenarios**:

1. **Given** multiple objects exist in a bucket, **When** the user initiates a batch copy to a new bucket, **Then** all specified objects are copied and the CLI reports success/failure for each.
2. **Given** multiple objects need renaming, **When** the user provides a batch rename command with old/new key mappings, **Then** all objects are renamed and results are reported.
3. **Given** a batch operation where some objects fail, **When** the operation completes, **Then** the CLI reports which objects succeeded and which failed with reasons.

---

### User Story 4 - BIM360 Folder Support (Priority: P2)

Users working with BIM360 projects (as opposed to ACC projects) cannot create folders because the system uses the wrong folder type identifier. The system must detect the project type and use the correct folder extension automatically.

**Why this priority**: BIM360 is widely used, and folder creation failure blocks basic file organization workflows for those users.

**Independent Test**: Can be tested by creating a folder in a BIM360 project and verifying it succeeds, then creating a folder in an ACC project and verifying it also succeeds.

**Acceptance Scenarios**:

1. **Given** a BIM360 project, **When** the user creates a folder, **Then** the system automatically uses the correct BIM360 folder type and the folder is created successfully.
2. **Given** an ACC project, **When** the user creates a folder, **Then** the system uses the standard folder type and the folder is created successfully.
3. **Given** the user does not specify a project type, **When** folder creation is requested, **Then** the system auto-detects the project type from the project identifier.

---

### User Story 5 - Parallel User Imports (Priority: P2)

Administrators importing many users into ACC projects experience slow sequential processing. The system should support concurrent imports to reduce total processing time.

**Why this priority**: Large organizations may import hundreds of users, and sequential processing makes this impractically slow, but the feature still functions correctly albeit slowly.

**Independent Test**: Can be tested by importing a batch of users and measuring that total time is significantly less than sequential processing would take.

**Acceptance Scenarios**:

1. **Given** a batch of users to import, **When** the import is executed, **Then** multiple users are processed concurrently and total time is reduced compared to sequential.
2. **Given** concurrent import is running, **When** some individual imports fail, **Then** other imports continue and the system reports individual success/failure results.
3. **Given** concurrency is used, **When** API rate limits are approached, **Then** the system respects rate limits and does not cause request failures.

---

### User Story 6 - Webhook Validation and Clarity (Priority: P3)

Users creating webhooks can specify invalid event types that the API will reject. The system should validate event types before making the API call and provide clear feedback. Additionally, the authentication method used for webhook operations should be clearly documented.

**Why this priority**: Validation prevents wasted API calls and confusing error messages, improving developer experience, but the API itself provides error feedback as a fallback.

**Independent Test**: Can be tested by attempting to create a webhook with an invalid event type and verifying the system rejects it before making an API call, listing valid event types in the error message.

**Acceptance Scenarios**:

1. **Given** a user provides an invalid event type when creating a webhook, **When** the command is executed, **Then** the system rejects the input immediately with a list of valid event types.
2. **Given** a user provides a valid event type, **When** the webhook is created, **Then** the operation proceeds normally.
3. **Given** webhook operations are documented, **When** a developer reads the code, **Then** the authentication method (2-legged OAuth) is clearly stated.

---

### User Story 7 - Reality Capture Data Quality Improvements (Priority: P3)

Users viewing photogrammetry results see raw file sizes as opaque strings. The system should present file sizes in a human-readable format and provide programmatic access to file size as a numeric value.

**Why this priority**: Display improvements are cosmetic but improve readability of CLI output for all photogrammetry users.

**Independent Test**: Can be tested by viewing a completed photoscene result and verifying file size is displayed in a human-readable format (e.g., "52.4 MB" instead of "54935241").

**Acceptance Scenarios**:

1. **Given** a completed photoscene with a file size, **When** the user views the result, **Then** the file size is displayed in human-readable format (B, KB, MB, GB as appropriate).
2. **Given** a file size value that cannot be parsed, **When** the result is displayed, **Then** the raw value is shown as a fallback.

---

### User Story 8 - Design Automation App Bundle Upload (Priority: P3)

Users who create custom Design Automation app bundles need to upload them through the CLI. Currently, the upload step is not integrated, requiring manual API calls.

**Why this priority**: App bundle creation is a specialized workflow used by a smaller subset of users, and alternative upload methods exist.

**Independent Test**: Can be tested by creating an app bundle definition and then uploading a bundle archive, verifying the upload completes and the bundle is available.

**Acceptance Scenarios**:

1. **Given** an app bundle has been created, **When** the user uploads a bundle archive file, **Then** the file is uploaded to the provided upload URL and the CLI confirms success.
2. **Given** an invalid or missing file path, **When** the upload is attempted, **Then** the CLI returns a clear error message.

---

### Edge Cases

- What happens when polling timeout fires exactly as the operation completes on the server?
- How does the system handle Model Derivative metadata requests for models mid-translation?
- What happens during batch OSS operations if the source bucket is deleted mid-operation?
- How does parallel user import handle duplicate users in the same batch?
- What happens when a BIM360 project ID does not follow the expected prefix convention?
- How does webhook validation handle future event types not yet in the known list?
- What happens when a file size string contains non-numeric characters (e.g., "5.2 MB" vs "5242880")?

## Requirements *(mandatory)*

### Functional Requirements

**Polling & Timeouts:**
- **FR-001**: System MUST terminate polling loops after a maximum wait period and report the timeout with enough context for the user to check status manually.
- **FR-002**: System MUST apply timeouts to all polling operations (translations, photogrammetry, design automation jobs).

**Model Derivative Metadata:**
- **FR-003**: System MUST support retrieving model metadata (viewables/views) for a translated model.
- **FR-004**: System MUST support retrieving the object tree hierarchy for a translated model.
- **FR-005**: System MUST support retrieving object/viewable properties for a translated model.
- **FR-006**: System MUST support all standard output formats (table, JSON, YAML, CSV, plain) for metadata results.

**OSS Batch Operations:**
- **FR-007**: System MUST support batch copying of objects between buckets.
- **FR-008**: System MUST support batch renaming of objects within a bucket.
- **FR-009**: System MUST report per-object success/failure for batch operations.

**BIM360 Folder Support:**
- **FR-010**: System MUST automatically detect BIM360 vs ACC project type when creating folders.
- **FR-011**: System MUST use the correct folder extension type based on the detected project type.

**Parallel User Imports:**
- **FR-012**: System MUST support concurrent user imports when processing batches.
- **FR-013**: System MUST respect API rate limits during concurrent operations.
- **FR-014**: System MUST report individual import results (success/failure per user).

**Webhook Improvements:**
- **FR-015**: System MUST validate webhook event types against a known list before API submission.
- **FR-016**: System MUST display the list of valid event types when an invalid one is provided.
- **FR-017**: System MUST clearly document the authentication method used for webhook operations.

**Reality Capture Improvements:**
- **FR-018**: System MUST display file sizes in human-readable format (B, KB, MB, GB).
- **FR-019**: System MUST fall back to raw string display when file size cannot be parsed as a number.
- **FR-020**: System MUST provide a programmatic method to access file size as a numeric value.

**Design Automation App Bundle:**
- **FR-021**: System MUST support uploading app bundle archives to Design Automation.
- **FR-022**: System MUST validate the file exists and is accessible before attempting upload.

**Async I/O:**
- **FR-023**: Token storage file operations MUST document their synchronous nature and the rationale for not using asynchronous alternatives.

### Key Entities

- **Polling Operation**: A long-running server-side operation (translation, photogrammetry, design automation) that the CLI periodically checks for completion. Has a timeout period, current status, and identifiers for manual follow-up.
- **Model Metadata**: Structured data about a translated model including viewables, object trees, and properties. Retrieved by model URN and view GUID.
- **Batch Operation**: A collection of individual object operations (copy, rename) executed as a group with per-item result tracking.
- **App Bundle**: A Design Automation archive containing custom processing logic, uploaded to a pre-signed URL after bundle creation.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All polling operations terminate within their configured timeout period (no infinite loops) — 100% of polling paths have timeout protection.
- **SC-002**: Users can retrieve metadata, object tree, and properties for any translated model through the CLI — all 7 missing Model Derivative read endpoints are accessible.
- **SC-003**: Batch OSS operations complete and report per-item results — users can copy or rename 100+ objects in a single command.
- **SC-004**: Folder creation succeeds for both BIM360 and ACC projects — 100% of project types produce correct folder extension types.
- **SC-005**: Batch user imports complete in less than half the time of sequential processing for batches of 20+ users.
- **SC-006**: Invalid webhook event types are rejected before API submission with actionable error messages listing valid alternatives.
- **SC-007**: File sizes are displayed in human-readable format for all photoscene results that contain numeric file size data.
- **SC-008**: App bundle archive uploads complete successfully through the CLI when a valid file path and upload URL are provided.
- **SC-009**: All existing tests continue to pass after changes — zero test regressions.

## Assumptions

- Polling timeouts are client-side only and do not cancel server-side operations.
- Reasonable default timeout values can be inferred from typical operation durations: photogrammetry (4 hours), translations (2 hours), design automation (30 minutes already exists).
- BIM360 projects can be identified by their project ID prefix convention (`b.`).
- Parallel user imports should use a modest concurrency limit (e.g., 5-10 concurrent requests) to avoid API rate limiting.
- The known webhook events list is maintained as a compile-time constant and updated when new events are supported.
- The Model Derivative metadata endpoints follow standard APS API patterns (REST, bearer auth, JSON responses).
- OSS batch operations use existing single-object operations composed into batch workflows, not a dedicated batch API endpoint.
- DA app bundle upload uses the upload URL returned by the bundle creation/version endpoint.

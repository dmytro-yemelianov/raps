# Feature Specification: Fix API Alignment Bugs

**Feature Branch**: `001-fix-api-alignment-bugs`
**Created**: 2026-02-24
**Status**: Draft
**Input**: Fix all BLOCKING and HIGH severity issues found in the RAPS codebase review against APS OpenAPI specifications

## Clarifications

### Session 2026-02-24

- Q: How should the breaking change from force-translate default (true→false) be handled for existing users? → A: Change default with deprecation notice in changelog and CLI help text
- Q: What should happen when the API returns an empty page with a `links.next` pointer during pagination? → A: Continue paginating if `links.next` exists, even on empty pages (follow the link contract)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Complete Data Retrieval for Large Projects (Priority: P1)

A user with an Autodesk account containing more than 200 projects runs `raps project list` to see all their projects. Currently, results are silently truncated at the first page (~200 items). The user expects to receive the complete list regardless of how many projects, folder items, or item versions exist.

**Why this priority**: Data truncation is the most severe class of bug — users receive incomplete data with no warning, leading to incorrect decisions and missing files in automated workflows.

**Independent Test**: Can be fully tested by creating a mock server with >200 items in list responses and verifying the CLI retrieves all pages. Delivers correct, complete data retrieval.

**Acceptance Scenarios**:

1. **Given** an account with 500 projects, **When** the user runs `raps project list`, **Then** all 500 projects are returned
2. **Given** a folder with 300 items, **When** the user runs `raps folder contents`, **Then** all 300 items are returned
3. **Given** an item with 250 versions, **When** the user runs `raps item versions`, **Then** all 250 versions are returned
4. **Given** the API returns paginated responses with `links.next` pointers, **When** the client fetches a list, **Then** it follows all pagination links until no more pages remain
5. **Given** a project with exactly 200 items (boundary), **When** the user lists contents, **Then** the system correctly determines whether more pages exist

---

### User Story 2 - Region-Aware Model Translation (Priority: P1)

A user in Europe stores their design files in the EMEA data center. When they run `raps translate start`, the translation job should be routed to their configured region, not hardcoded to the US data center. Additionally, users should be able to control whether a translation job overwrites an existing manifest or preserves it.

**Why this priority**: Using the wrong region means data may be processed in a non-compliant jurisdiction, violating data residency requirements. Forced manifest deletion causes data loss.

**Independent Test**: Can be tested by configuring a non-US region and verifying the correct region header/URL is used in API requests. Force-translate behavior can be tested by checking the `x-ads-force` header value.

**Acceptance Scenarios**:

1. **Given** a user whose configuration specifies the EMEA region, **When** they submit a translation job, **Then** the API request includes the correct region parameter
2. **Given** a user with no region configured, **When** they submit a translation job, **Then** the US region is used as default
3. **Given** a user who wants to preserve an existing manifest, **When** they submit a translation without the force flag, **Then** the existing manifest is preserved
4. **Given** a user who wants to re-translate, **When** they submit a translation with the force flag, **Then** the existing manifest is replaced

---

### User Story 3 - Consistent Project ID Handling Across Modules (Priority: P2)

An administrator uses RAPS to manage users across ACC projects. They pass a project ID to one command (e.g., `raps admin user-add`) and then use the same ID in another command (e.g., `raps folder permissions`). Currently, different modules apply opposite normalization logic to project IDs (one strips the "b." prefix, the other adds it), causing commands to fail or target the wrong project.

**Why this priority**: Inconsistent ID handling causes silent data corruption or access to wrong projects — a security and data integrity concern.

**Independent Test**: Can be tested by passing the same project ID (with and without "b." prefix) to admin and permissions commands and verifying both resolve to the correct project.

**Acceptance Scenarios**:

1. **Given** a project ID with "b." prefix, **When** used in admin commands and permission commands, **Then** both resolve to the same underlying project
2. **Given** a project ID without "b." prefix, **When** used in admin commands and permission commands, **Then** both resolve to the same underlying project
3. **Given** a BIM 360 project ID format, **When** normalized, **Then** the format matches what the respective API expects
4. **Given** an ACC project ID format, **When** normalized, **Then** the format matches what the respective API expects

---

### User Story 4 - Reliable Concurrent Authentication (Priority: P2)

A user runs multiple RAPS commands in parallel (e.g., in a CI/CD pipeline with concurrent jobs). When a 3-legged OAuth token expires during parallel execution, one refresh should succeed and all concurrent requests should use the new token. Currently, multiple concurrent requests can race to refresh, causing unnecessary token clearing and authentication failures.

**Why this priority**: Race conditions in authentication cause intermittent failures that are hard to diagnose, especially in automated pipelines.

**Independent Test**: Can be tested by simulating concurrent requests with an expired token and verifying only one refresh occurs while all requests eventually succeed.

**Acceptance Scenarios**:

1. **Given** an expired 3-legged token and 5 concurrent requests, **When** all requests detect the expired token, **Then** only one refresh is performed
2. **Given** a successful token refresh in progress, **When** other requests arrive, **Then** they wait for the refresh to complete and use the new token
3. **Given** a failed token refresh, **When** other requests are waiting, **Then** they all receive the refresh failure error without clearing valid cached tokens

---

### User Story 5 - Upload Non-JPEG Photos for Reality Capture (Priority: P3)

A user wants to process a set of photos for photogrammetry that includes RAW, TIFF, and PNG formats alongside JPEG. Currently, all uploads are hardcoded with an "image/jpeg" MIME type, which may cause the API to reject or misprocess non-JPEG files.

**Why this priority**: Limiting uploads to JPEG restricts the tool's usefulness for professional photogrammetry workflows that commonly use RAW and TIFF formats.

**Independent Test**: Can be tested by uploading files with various image extensions and verifying the correct MIME type is sent for each.

**Acceptance Scenarios**:

1. **Given** a PNG file, **When** uploaded for Reality Capture, **Then** the request uses "image/png" as the MIME type
2. **Given** a TIFF file, **When** uploaded for Reality Capture, **Then** the request uses "image/tiff" as the MIME type
3. **Given** a JPEG file, **When** uploaded for Reality Capture, **Then** the request uses "image/jpeg" as the MIME type
4. **Given** a file with an unrecognized extension, **When** uploaded for Reality Capture, **Then** the request uses "application/octet-stream" as a fallback
5. **Given** a mixed set of JPEG, PNG, and TIFF files, **When** uploaded in a single batch, **Then** each file uses its correct MIME type

---

### Edge Cases

- **Empty pagination page**: If the API returns a page with zero items but includes a `links.next` pointer, the system continues paginating (follows the link contract). Only stops when `links.next` is absent.
- **Unrecognized region value**: The system rejects the value at configuration time with a clear error listing supported regions.
- **Unknown project ID format**: If a project ID is neither BIM 360 ("b.") nor ACC ("a.") format, the system passes it through unchanged and lets the API validate it.
- **Already-expired refreshed token**: If a newly refreshed token is already expired, the system treats it as a refresh failure and reports an authentication error.
- **Rejected MIME type**: If the Reality Capture API rejects an auto-detected MIME type, the error is surfaced to the user with the detected type shown for debugging.
- **Items added/deleted during pagination**: The system follows the cursor contract faithfully — it may include or skip items modified during traversal, consistent with eventual consistency behavior of the APS APIs.

## Requirements *(mandatory)*

### Functional Requirements

**Pagination (raps-dm):**

- **FR-001**: System MUST follow all pagination links when listing projects, returning complete results regardless of total count
- **FR-002**: System MUST follow all pagination links when listing folder contents, returning complete results
- **FR-003**: System MUST follow all pagination links when listing item versions, returning complete results
- **FR-004**: System MUST stop paginating when no further pagination link is present in the response
- **FR-005**: System MUST enforce a maximum page limit (e.g., 100 pages) to prevent infinite loops from malformed API responses
- **FR-021**: System MUST continue paginating when a page contains zero items but a `links.next` pointer is present (follow the link contract, not item count)

**Region Support (raps-derivative):**

- **FR-006**: System MUST use the configured region when submitting Model Derivative translation jobs
- **FR-007**: System MUST default to the US region when no region is configured
- **FR-008**: System MUST support all APS regions: US, EMEA, AUS, CAN, DEU, IND, JPN, GBR

**Force-Translate Control (raps-derivative):**

- **FR-009**: System MUST NOT set the force-translate flag by default when submitting translation jobs
- **FR-010**: System MUST provide a user-facing option (e.g., `--force` flag) to enable force-translate when explicitly desired
- **FR-020**: The default behavior change (force-translate off) MUST be accompanied by a deprecation notice in the changelog and updated CLI help text

**Project ID Normalization (raps-acc):**

- **FR-011**: System MUST use a single, shared normalization function for project IDs across all ACC modules
- **FR-012**: The normalization function MUST produce the correct format for each target API (add prefix for Data Management API, strip prefix for Construction Admin API)
- **FR-013**: System MUST handle BIM 360 ("b.{uuid}") and ACC ("a.{base64}") ID formats correctly

**Token Refresh Safety (raps-kernel):**

- **FR-014**: System MUST ensure only one token refresh attempt occurs at a time, even under concurrent access
- **FR-015**: System MUST allow concurrent requests to wait for an in-progress refresh rather than triggering their own
- **FR-016**: System MUST NOT clear valid cached tokens when a concurrent refresh attempt fails

**MIME Type Detection (raps-reality):**

- **FR-017**: System MUST detect the correct MIME type from file extensions when uploading photos for Reality Capture
- **FR-018**: System MUST support at minimum: JPEG, PNG, TIFF, BMP, and WebP image formats
- **FR-019**: System MUST fall back to "application/octet-stream" for unrecognized file extensions

### Key Entities

- **Pagination Cursor**: Represents the position in a paginated result set. Contains a reference to the next page (URL or token). Used by list operations to iterate through all pages.
- **Region**: Represents an APS data center location (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR). Determines where API requests are routed. Configured per profile.
- **Project ID**: Identifier for an Autodesk project. Comes in multiple formats depending on the platform (BIM 360 vs ACC) and the target API. Must be normalized per-API.
- **Token Refresh Lock**: Coordination mechanism ensuring only one refresh occurs at a time. Concurrent callers wait for the result rather than initiating their own refresh.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All list operations return complete data sets — verified by testing against mock servers returning 500+ items across multiple pages
- **SC-002**: Translation jobs in non-US regions route correctly — verified by checking API request headers/URLs contain the configured region
- **SC-003**: Existing translation manifests are preserved by default — verified by submitting a translation without force flag and confirming no manifest deletion
- **SC-004**: The same project ID produces correct API calls regardless of which module is used — verified by integration tests with both admin and permissions commands
- **SC-005**: Under 10 concurrent requests with an expired token, exactly 1 refresh attempt occurs — verified by instrumented tests counting refresh calls
- **SC-006**: Reality Capture uploads use correct MIME types for all supported image formats — verified by checking request content-type headers per file

### Assumptions

- The APS pagination contract follows JSON:API conventions with `links.next` for REST endpoints in the Data Management API
- Region configuration already exists in raps-kernel's Config struct (confirmed: `base_url` is configurable, region headers are supported)
- The token refresh race condition can be resolved with a mutex/once-cell pattern without changing the public AuthClient API
- The Reality Capture API accepts standard MIME types for supported image formats (JPEG, PNG, TIFF)
- The `x-ads-force` header defaulting to false (or being absent) means the API will return an existing manifest if one exists

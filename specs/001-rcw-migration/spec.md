# Feature Specification: RCW Migration Automation

**Feature Branch**: `001-rcw-migration`
**Created**: 2026-01-23
**Status**: Draft
**Input**: User description: "Implement Revit Cloud Worksharing (RCW) migration automation within RAPS CLI, allowing users to migrate RCW models from BIM 360 to ACC Docs using Design Automation"

## Overview

Revit Cloud Worksharing (RCW) models stored in BIM 360 need to be migrated to Autodesk Construction Cloud (ACC) Docs as organizations transition between platforms. Currently, this requires manual effort using Autodesk's web-based migration tool or desktop Revit. This feature enables automated, scriptable migration of RCW models directly from the command line, supporting batch operations and CI/CD integration.

### Business Value

- **Reduced Manual Effort**: Migrate hundreds of RCW models without opening Revit desktop or using web UI
- **Automation Ready**: Integrate migration into scripts, pipelines, and scheduled jobs
- **Consistency**: Ensure all models are migrated with the same settings and target Revit version
- **Visibility**: Track migration progress and status for all files in a batch

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Migrate Single RCW Model (Priority: P1)

As a BIM Manager, I want to migrate a single RCW model from BIM 360 to ACC Docs so that I can move critical project files to the new platform without using desktop Revit.

**Why this priority**: This is the core value proposition - enabling migration without manual intervention. All other features build on this foundation.

**Independent Test**: Can be fully tested by selecting one RCW model and migrating it to a destination folder, then verifying the model opens correctly in ACC.

**Acceptance Scenarios**:

1. **Given** I have authenticated with RAPS and have access to both source and destination projects, **When** I run the migration command with a source file and destination folder, **Then** the system initiates a migration job and returns a tracking ID
2. **Given** a migration job is running, **When** I query the status, **Then** I see the current progress (pending, processing, completed, or failed)
3. **Given** a migration job completes successfully, **When** I access the destination folder, **Then** the migrated model is available as a new RCW in ACC Docs
4. **Given** a migration job fails, **When** I query the status, **Then** I see a clear error message explaining why it failed

---

### User Story 2 - List RCW Models in a Folder (Priority: P1)

As a BIM Manager, I want to discover all RCW models in a BIM 360 folder so that I can identify which files need to be migrated.

**Why this priority**: Users need to identify eligible files before migration. This enables planning and is a prerequisite for batch operations.

**Independent Test**: Can be tested by pointing to a folder with mixed content and verifying only RCW models are listed.

**Acceptance Scenarios**:

1. **Given** I have access to a BIM 360 project folder, **When** I run the list command, **Then** I see all RCW models (C4RModel type) with their names and sizes
2. **Given** a folder contains non-RCW files (PDFs, DWGs, regular RVT files), **When** I list RCW models, **Then** only C4RModel files are shown
3. **Given** a folder contains nested subfolders with RCW models, **When** I list with recursive option, **Then** I see models from all nested levels

---

### User Story 3 - Configure Migration Environment (Priority: P1)

As a system administrator, I want to set up the migration automation environment once so that subsequent migrations work without additional configuration.

**Why this priority**: The migration requires a pre-configured automation environment (AppBundle and Activity). This is a one-time setup that enables all migrations.

**Independent Test**: Can be tested by running the configure command and verifying the automation components are created successfully.

**Acceptance Scenarios**:

1. **Given** I have valid automation credentials, **When** I run the configure command with a target Revit version, **Then** the system creates the required automation components
2. **Given** automation components already exist, **When** I run configure again, **Then** the system updates to a new version without disruption
3. **Given** I specify an invalid Revit version, **When** I run configure, **Then** I receive a clear error listing valid options (2025, 2026)

---

### User Story 4 - Batch Migration (Priority: P2)

As a BIM Manager, I want to migrate all RCW models from a source folder to a destination folder so that I can move entire project directories efficiently.

**Why this priority**: Batch operations are essential for production use but depend on single-file migration working correctly.

**Independent Test**: Can be tested by migrating a folder with multiple RCW files and verifying all appear in the destination.

**Acceptance Scenarios**:

1. **Given** a folder contains 5 RCW models, **When** I run batch migration, **Then** the system queues all 5 files and returns tracking IDs for each
2. **Given** a batch migration is in progress, **When** I check status, **Then** I see aggregate progress (e.g., "3 of 5 completed")
3. **Given** some files in a batch fail, **When** migration completes, **Then** I receive a summary showing which succeeded and which failed with reasons

---

### User Story 5 - Cancel Migration (Priority: P3)

As a BIM Manager, I want to cancel a running migration so that I can stop accidental or incorrect migrations before completion.

**Why this priority**: Important for error recovery but less frequently needed than core migration functionality.

**Independent Test**: Can be tested by starting a migration, then cancelling before completion.

**Acceptance Scenarios**:

1. **Given** a migration job is in "pending" or "processing" state, **When** I run the cancel command, **Then** the job is terminated and marked as cancelled
2. **Given** a migration job has already completed, **When** I attempt to cancel, **Then** I receive a message that the job cannot be cancelled

---

### User Story 6 - Check Migration Status (Priority: P2)

As a BIM Manager, I want to check the status of my migration jobs so that I can monitor progress and identify issues.

**Why this priority**: Status monitoring is essential for tracking async operations, especially in automation scenarios.

**Independent Test**: Can be tested by querying status of known job IDs.

**Acceptance Scenarios**:

1. **Given** I have a migration job ID, **When** I query its status, **Then** I see current state, progress percentage, and timing information
2. **Given** a migration has failed, **When** I query status, **Then** I see error details and a link to detailed logs
3. **Given** I want to monitor multiple jobs, **When** I list recent migrations, **Then** I see all jobs from my current session with their statuses

---

### Edge Cases

- What happens when the source RCW model is corrupted or locked by another user?
- How does the system handle network interruptions during migration?
- What happens when the destination folder doesn't exist or user lacks write permission?
- How are linked models handled (models with external references)?
- What happens when the target Revit version is older than the source model version?
- How does the system handle very large models (>1GB)?
- What happens if the user's authentication token expires mid-migration?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST authenticate users with both 2-legged (for automation service) and 3-legged (for data access) credentials
- **FR-002**: System MUST list all RCW models (C4RModel type) in a specified BIM 360 folder
- **FR-003**: System MUST filter out non-RCW files (standard RVT, RFA, RTE, PDFs, etc.) when listing migration candidates
- **FR-004**: System MUST create and manage automation components (AppBundle and Activity) for migration
- **FR-005**: System MUST support Revit 2025 and Revit 2026 as target migration versions
- **FR-006**: System MUST initiate migration jobs that download source RCW, process through automation, and publish to destination
- **FR-007**: System MUST preserve the original filename when migrating unless user specifies otherwise
- **FR-008**: System MUST provide real-time status updates for running migration jobs
- **FR-009**: System MUST allow cancellation of pending or in-progress migration jobs
- **FR-010**: System MUST report clear error messages when migrations fail, including actionable remediation steps
- **FR-011**: System MUST support batch migration of multiple files in a single command
- **FR-012**: System MUST validate user permissions on both source and destination before starting migration
- **FR-013**: System MUST support recursive listing of RCW models in nested folder structures
- **FR-014**: System MUST log all migration operations with timestamps for audit purposes

### Key Entities

- **RCW Model**: A Revit Cloud Worksharing model (C4RModel type) stored in BIM 360 or ACC. Contains collaborative Revit data that can be accessed by multiple users simultaneously.
- **Migration Job**: A unit of work representing the migration of one RCW model. Has a unique ID, status, progress, and timing information.
- **Automation Environment**: The pre-configured AppBundle and Activity required to run migrations. Created once per Revit version.
- **Source Location**: A BIM 360 project folder containing RCW models to be migrated.
- **Destination Location**: An ACC Docs project folder where migrated models will be published.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can migrate a single RCW model in under 5 command invocations (configure once, list, migrate, check status)
- **SC-002**: Batch migration of 10 files completes with same total duration as using web-based migration tool
- **SC-003**: 95% of migration attempts on valid, accessible RCW models complete successfully
- **SC-004**: Users receive status updates within 10 seconds of job state changes
- **SC-005**: Error messages provide actionable guidance in 100% of failure cases
- **SC-006**: Migration commands integrate into shell scripts without interactive prompts (fully automatable)
- **SC-007**: System correctly identifies and lists RCW models with 100% accuracy (no false positives or negatives)
- **SC-008**: Users can cancel pending migrations within 30 seconds of requesting cancellation

## Assumptions

- Users have valid Autodesk Platform Services credentials with appropriate scopes
- Source BIM 360 projects and destination ACC projects are accessible to the authenticated user
- The Revit Design Automation service is available and operational
- Users understand the difference between RCW (C4RModel) and standard Revit files
- Migrated models will be new RCW instances (not updates to existing files)
- The automation AppBundle/Activity will be hosted under the user's own APS application

## Out of Scope

- Migration of linked models (external references) - each model is migrated independently
- Automatic version upgrade of Revit models beyond the target version
- Two-way sync between BIM 360 and ACC
- Migration of non-RCW Revit files (standard .rvt, .rfa, .rte)
- Desktop Revit integration or UI components
- Webhook-based notifications (status polling only for initial release)

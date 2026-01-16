# Feature Specification: Account Admin Bulk Management Tool

**Feature Branch**: `001-account-admin-management`
**Created**: 2026-01-16
**Status**: Draft
**Input**: User description: "Manage tool for account admins who manage users, rights and roles. For example, when a new colleague joins, they sometimes have to be added to 3,000 active projects. Or when role permissions change (due to a change of function), those permissions have to be adjusted across all (more than 5,000) active projects. Create a tool to manage active projects and in particular users, roles, folder rights for account admins"

## Clarifications

### Session 2026-01-16

- Q: Which Autodesk platforms should be supported? → A: Both ACC and BIM 360 projects
- Q: How should bulk operations handle concurrency? → A: Configurable parallel limit (default 10 concurrent requests)
- Q: How should admins identify target users? → A: Email address (validated against account users)
- Q: How to handle duplicate user assignments? → A: Skip existing assignments, report them as "already exists"
- Q: Where should operation progress be stored for resume? → A: Local file in user's config directory

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Bulk Add User to Multiple Projects (Priority: P1)

An account admin needs to onboard a new colleague by adding them to thousands of active projects at once. Currently, this requires manual repetitive work across each project individually, which is time-consuming and error-prone.

**Why this priority**: This is the most frequently mentioned pain point - onboarding new colleagues requires adding them to potentially 3,000+ projects. This directly addresses the primary use case and delivers immediate time savings.

**Independent Test**: Can be fully tested by adding a single user to a filtered set of projects and verifying their access appears correctly in each project.

**Acceptance Scenarios**:

1. **Given** an account admin is authenticated with admin privileges, **When** they select a user and choose multiple projects (up to 5,000), **Then** the user is added to all selected projects with the specified role
2. **Given** an account admin initiates a bulk add operation, **When** the operation is in progress, **Then** the admin can see real-time progress and status for each project
3. **Given** a bulk add operation completes, **When** the admin reviews results, **Then** they see a summary showing successful additions, failures, and reasons for any failures
4. **Given** some projects fail during bulk add, **When** the admin reviews failures, **Then** they can retry failed projects without re-processing successful ones

---

### User Story 2 - Bulk Update User Roles Across Projects (Priority: P1)

When an employee changes function or receives a promotion, their role permissions need to be updated across all projects they're assigned to. This change must propagate consistently to potentially thousands of projects.

**Why this priority**: Role changes due to function changes are explicitly mentioned as a core use case affecting 5,000+ projects. This is equally critical to onboarding.

**Independent Test**: Can be tested by changing a user's role from one type to another across a subset of projects and verifying the new role applies correctly.

**Acceptance Scenarios**:

1. **Given** an account admin selects a user who exists in multiple projects, **When** they update the user's role, **Then** the role change is applied to all projects where that user is a member
2. **Given** an account admin wants to update roles selectively, **When** they filter projects by criteria (status, type, date), **Then** role changes only apply to the filtered project set
3. **Given** a bulk role update is in progress, **When** the admin monitors progress, **Then** they see which projects have been updated and which are pending
4. **Given** role updates fail for some projects, **When** the admin reviews the operation, **Then** they receive clear error messages and can take corrective action

---

### User Story 3 - Bulk Remove User from Projects (Priority: P2)

When an employee leaves the organization or changes departments, their access must be revoked from all relevant projects efficiently and completely.

**Why this priority**: While not explicitly mentioned, user removal is the logical counterpart to user addition and is essential for security and access management.

**Independent Test**: Can be tested by removing a user from multiple projects and verifying they no longer appear in project member lists.

**Acceptance Scenarios**:

1. **Given** an account admin needs to offboard a user, **When** they select the user and initiate bulk removal, **Then** the user is removed from all selected projects
2. **Given** a user is removed from projects, **When** the operation completes, **Then** an audit record is created documenting the removal
3. **Given** some removals fail, **When** the admin reviews results, **Then** they can identify which projects still have the user and why removal failed

---

### User Story 4 - View and Filter Active Projects (Priority: P2)

Account admins need to view all active projects under their account and filter them by various criteria before performing bulk operations.

**Why this priority**: This is a foundational capability that enables all bulk operations - admins must be able to select and filter projects before acting on them.

**Independent Test**: Can be tested by listing projects and applying various filters, verifying the result set matches filter criteria.

**Acceptance Scenarios**:

1. **Given** an account admin accesses the tool, **When** they request a list of projects, **Then** they see all active projects in their account with key metadata
2. **Given** an admin views the project list, **When** they apply filters (by name, status, date range, region), **Then** the list updates to show only matching projects
3. **Given** an admin has a large project list, **When** they export the list, **Then** they receive a file with all project details for offline review

---

### User Story 5 - Bulk Manage Folder Rights (Priority: P2)

Account admins need to adjust folder-level permissions for users across multiple projects, ensuring consistent access rights to project folders.

**Why this priority**: Folder rights management is explicitly mentioned in the feature description as a key capability for account admins.

**Independent Test**: Can be tested by updating folder permissions for a user across multiple projects and verifying the new permissions apply.

**Acceptance Scenarios**:

1. **Given** an account admin selects a user and target folders, **When** they assign folder rights, **Then** the rights are applied across all selected projects containing those folder types
2. **Given** folder rights vary by project, **When** the admin reviews current permissions, **Then** they see a consolidated view of the user's folder rights across all projects
3. **Given** an admin updates folder rights, **When** some updates fail, **Then** they receive detailed error information and can retry failed updates

---

### User Story 6 - Preview and Dry Run Operations (Priority: P3)

Before executing bulk operations that affect thousands of projects, admins want to preview what changes will be made without actually applying them.

**Why this priority**: Given the scale of operations (3,000-5,000+ projects), a preview capability reduces risk of errors and builds admin confidence.

**Independent Test**: Can be tested by running a dry-run operation and verifying no actual changes are made while a preview report is generated.

**Acceptance Scenarios**:

1. **Given** an admin configures a bulk operation, **When** they request a preview, **Then** they see a summary of affected projects without changes being applied
2. **Given** a preview is generated, **When** the admin reviews it, **Then** they can proceed with execution or modify the operation parameters
3. **Given** a preview shows potential issues, **When** the admin reviews warnings, **Then** they understand which projects may have problems and why

---

### Edge Cases

- What happens when a project is archived or deleted during a bulk operation?
- How does the system handle projects where the admin lacks sufficient permissions?
- What happens when rate limits are encountered during large-scale operations?
- How does the system handle duplicate requests (same user already in project)? → Skip existing assignments and report them as "already exists" in the results summary
- What happens if network connectivity is lost mid-operation? → Operation state is persisted locally; admin can resume from last successful point when connectivity is restored
- How are projects handled that have custom permission models?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST support both ACC (Autodesk Construction Cloud) and BIM 360 projects, handling platform-specific API differences transparently
- **FR-002**: System MUST identify target users by email address, validated against account users before operations begin
- **FR-003**: System MUST allow account admins to add a user to multiple projects (up to 5,000) in a single operation
- **FR-004**: System MUST skip projects where the user already exists during bulk add operations and report them as "already exists" in the results
- **FR-005**: System MUST allow account admins to update user roles across multiple projects in a single operation
- **FR-006**: System MUST allow account admins to remove users from multiple projects in a single operation
- **FR-007**: System MUST provide filtering capabilities for projects by name, status, date range, and other relevant criteria
- **FR-008**: System MUST display real-time progress during bulk operations showing completed, pending, and failed items
- **FR-009**: System MUST generate detailed reports after bulk operations showing successes, failures, and error reasons
- **FR-010**: System MUST allow retrying failed operations without re-processing successful ones
- **FR-011**: System MUST provide a preview/dry-run mode for all bulk operations
- **FR-012**: System MUST allow account admins to view and modify folder-level permissions for users across projects
- **FR-013**: System MUST create audit records for all bulk operations performed
- **FR-014**: System MUST handle rate limiting gracefully with automatic retry and backoff
- **FR-015**: System MUST provide a configurable concurrency limit for parallel processing (default: 10 concurrent requests) to balance speed and API rate limits
- **FR-016**: System MUST validate admin permissions before starting bulk operations
- **FR-017**: System MUST allow exporting project lists and operation results
- **FR-018**: System MUST support cancellation of in-progress bulk operations
- **FR-019**: System MUST resume interrupted operations from the last successful point
- **FR-020**: System MUST persist operation state to a local file in the user's config directory to enable cross-session resume

### Key Entities

- **Account Admin**: A user with administrative privileges at the account level, authorized to manage users and permissions across all projects in the account
- **Project**: An ACC (Autodesk Construction Cloud) or BIM 360 project within the account, containing members, roles, and folder structures
- **User**: An individual who can be assigned to projects with specific roles and permissions; identified by email address within the account
- **Role**: A named set of permissions that defines what actions a user can perform within a project (e.g., Project Admin, Editor, Viewer)
- **Folder Rights**: Specific permissions assigned to users for accessing and modifying folders within projects
- **Bulk Operation**: A single administrative action applied across multiple projects, tracked as a unit with progress and results
- **Operation Result**: The outcome record of a bulk operation containing success/failure status per project and error details

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Account admins can add a user to 3,000 projects in under 30 minutes (compared to hours/days of manual work)
- **SC-002**: Account admins can update user roles across 5,000 projects in under 45 minutes
- **SC-003**: 95% of bulk operations complete successfully without requiring manual intervention
- **SC-004**: Admins can identify and resolve failed operations within 5 minutes using the provided error reports
- **SC-005**: Admins can preview the impact of any bulk operation before execution
- **SC-006**: System provides clear progress indication, showing percentage complete and estimated time remaining
- **SC-007**: All bulk operations are auditable with complete records of what changed, when, and by whom
- **SC-008**: Admins can filter and select target projects in under 2 minutes for any bulk operation

## Assumptions

- Account admins have appropriate permissions at the account level to manage users and roles across all projects
- The Autodesk Platform Services APIs support the required operations for user management, role assignment, and folder permissions
- Projects are accessible via existing authentication mechanisms (2-legged or 3-legged OAuth)
- The tool will operate within APS API rate limits, implementing appropriate throttling and retry logic
- Standard roles (Project Admin, Editor, Viewer, etc.) are used; custom role definitions are out of scope for initial release
- The tool targets both ACC (Autodesk Construction Cloud) and BIM 360 projects; the system will handle platform-specific API differences internally; other Autodesk products are out of scope

## Scope Boundaries

### In Scope

- Bulk user addition to projects
- Bulk user removal from projects
- Bulk role updates across projects
- Bulk folder rights management
- Project filtering and selection
- Operation preview/dry-run
- Progress monitoring and reporting
- Audit logging
- Export of results

### Out of Scope

- Creating new projects
- Managing project settings beyond user permissions
- Custom role definition and management
- Company-level user management (inviting users to account)
- Integration with external identity providers
- Automated scheduling of bulk operations
- Cross-account operations

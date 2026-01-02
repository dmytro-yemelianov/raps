# Feature Specification: GitLab CLI Orchestration and Monitoring

**Feature Branch**: `001-gitlab-orchestration`  
**Created**: 2025-12-30  
**Status**: Draft  
**Input**: User description: "create cli orchestration/monitoring using gitlab. you have GL_PAT in .env file"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Monitor GitLab Pipeline Status (Priority: P1)

As a developer or DevOps engineer, I want to check the status of GitLab CI/CD pipelines from the command line so I can quickly see if my builds are passing or failing without opening a browser.

**Why this priority**: This is the most fundamental monitoring capability - users need to see pipeline status before they can take any action. It's independently valuable and can be tested immediately.

**Independent Test**: Can be fully tested by running `raps gitlab pipeline status <project>` and verifying it displays pipeline status. Delivers immediate value by showing build health.

**Acceptance Scenarios**:

1. **Given** a GitLab project with pipelines, **When** I run `raps gitlab pipeline status <project-id>`, **Then** I see a list of recent pipelines with their status (success, failed, running, pending)
2. **Given** a GitLab project with no pipelines, **When** I run `raps gitlab pipeline status <project-id>`, **Then** I see a message indicating no pipelines found
3. **Given** I have GL_PAT set in .env file, **When** I run any gitlab command, **Then** authentication succeeds automatically
4. **Given** GL_PAT is missing or invalid, **When** I run a gitlab command, **Then** I see a clear error message explaining how to set GL_PAT

---

### User Story 2 - Trigger GitLab Pipeline Execution (Priority: P2)

As a developer, I want to trigger GitLab pipelines from the command line so I can start builds programmatically without navigating to the GitLab web interface.

**Why this priority**: Orchestration requires the ability to trigger actions, not just monitor. This enables automation workflows and CI/CD integration.

**Independent Test**: Can be fully tested by running `raps gitlab pipeline trigger <project> --ref main` and verifying a new pipeline is created. Delivers value by enabling automated pipeline execution.

**Acceptance Scenarios**:

1. **Given** a GitLab project with CI/CD configuration, **When** I run `raps gitlab pipeline trigger <project-id> --ref main`, **Then** a new pipeline is created and I see its ID and status
2. **Given** I want to pass variables to a pipeline, **When** I run `raps gitlab pipeline trigger <project-id> --ref main --var KEY=value`, **Then** the pipeline runs with those variables set
3. **Given** I trigger a pipeline with an invalid ref, **When** the command executes, **Then** I see an error message explaining the ref doesn't exist

---

### User Story 3 - Monitor GitLab Job Execution and Logs (Priority: P2)

As a developer debugging a failed build, I want to view GitLab job logs from the command line so I can quickly identify what went wrong without switching contexts.

**Why this priority**: Monitoring pipelines is incomplete without seeing job-level details and logs. This is essential for debugging and understanding build failures.

**Independent Test**: Can be fully tested by running `raps gitlab job logs <project-id> <job-id>` and verifying logs are displayed. Delivers value by enabling quick failure diagnosis.

**Acceptance Scenarios**:

1. **Given** a GitLab pipeline with jobs, **When** I run `raps gitlab job list <project-id> <pipeline-id>`, **Then** I see all jobs in the pipeline with their status
2. **Given** a running or completed job, **When** I run `raps gitlab job logs <project-id> <job-id>`, **Then** I see the job's log output
3. **Given** a job that is still running, **When** I run `raps gitlab job logs <project-id> <job-id> --follow`, **Then** logs stream in real-time until the job completes
4. **Given** a job that failed, **When** I view its logs, **Then** error messages are clearly highlighted or at the end of the output

---

### User Story 4 - List and Search GitLab Projects (Priority: P3)

As a user managing multiple GitLab projects, I want to list and search my accessible projects from the command line so I can quickly find the project I need to work with.

**Why this priority**: Users need to discover projects before they can monitor or orchestrate them. This is foundational but lower priority than core monitoring/triggering.

**Independent Test**: Can be fully tested by running `raps gitlab project list` and verifying accessible projects are displayed. Delivers value by enabling project discovery.

**Acceptance Scenarios**:

1. **Given** I have access to multiple GitLab projects, **When** I run `raps gitlab project list`, **Then** I see a list of all projects I can access with their names, IDs, and paths
2. **Given** I want to find a specific project, **When** I run `raps gitlab project list --search <term>`, **Then** I see only projects matching the search term
3. **Given** I want project details, **When** I run `raps gitlab project get <project-id>`, **Then** I see comprehensive project information including description, visibility, and default branch

---

### Edge Cases

- What happens when GL_PAT expires or is revoked? System should detect authentication failure and provide clear guidance
- How does system handle GitLab API rate limiting? Should respect rate limits and provide helpful messages
- What happens when a project doesn't exist or user lacks access? Should show clear permission/not found errors
- How does system handle network timeouts or GitLab API unavailability? Should retry with backoff and show clear error messages
- What happens when pipeline trigger fails due to invalid CI/CD configuration? Should show configuration error details
- How does system handle very long job logs? Should support pagination or streaming for large outputs
- What happens when multiple pipelines are running simultaneously? Should handle concurrent operations gracefully

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST authenticate with GitLab API using GL_PAT from .env file
- **FR-002**: System MUST load GL_PAT from .env file in current or parent directories
- **FR-003**: System MUST support GitLab API v4 (current stable version)
- **FR-004**: System MUST allow users to list pipelines for a GitLab project
- **FR-005**: System MUST display pipeline status (success, failed, running, pending, canceled, skipped)
- **FR-006**: System MUST allow users to trigger new pipelines for a GitLab project
- **FR-007**: System MUST support passing variables to triggered pipelines
- **FR-008**: System MUST allow users to list jobs within a pipeline
- **FR-009**: System MUST allow users to view job logs
- **FR-010**: System MUST support streaming job logs for running jobs (--follow flag)
- **FR-011**: System MUST allow users to list accessible GitLab projects
- **FR-012**: System MUST support searching/filtering projects by name or path
- **FR-013**: System MUST allow users to get detailed information about a specific project
- **FR-014**: System MUST handle GitLab API errors gracefully with user-friendly messages
- **FR-015**: System MUST respect GitLab API rate limits and provide helpful messages when rate limited
- **FR-016**: System MUST support pagination for list operations (pipelines, jobs, projects)
- **FR-017**: System MUST allow users to specify GitLab instance URL (defaults to gitlab.com)
- **FR-018**: System MUST support output formats: table (default), json, yaml for all commands
- **FR-019**: System MUST validate GL_PAT format before making API calls (basic format check)
- **FR-020**: System MUST provide clear error messages when GL_PAT is missing, invalid, or lacks required permissions

### Key Entities *(include if feature involves data)*

- **GitLab Project**: Represents a GitLab repository/project with attributes: ID, name, path, visibility, default branch, description
- **Pipeline**: Represents a CI/CD pipeline execution with attributes: ID, status, ref (branch/tag), SHA, created date, duration
- **Job**: Represents a single job within a pipeline with attributes: ID, name, status, stage, duration, artifacts, log output
- **Pipeline Variable**: Represents a key-value pair passed to a pipeline with attributes: key, value, protected flag

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can check pipeline status for any accessible project in under 3 seconds
- **SC-002**: Users can trigger a pipeline and receive confirmation in under 5 seconds
- **SC-003**: Users can view job logs for completed jobs in under 2 seconds
- **SC-004**: System handles GitLab API rate limits gracefully without crashing (retries with exponential backoff)
- **SC-005**: 95% of GitLab API operations complete successfully on first attempt (excluding rate limits)
- **SC-006**: Users can discover and list all accessible projects in under 5 seconds
- **SC-007**: Error messages clearly explain the issue and how to resolve it (no cryptic API error codes shown directly to users)
- **SC-008**: System supports projects from both gitlab.com and self-hosted GitLab instances

## Assumptions

- GitLab API v4 is the target API version (current stable)
- GL_PAT token has at least "read_api" scope for monitoring, "api" scope for triggering pipelines
- Users have basic familiarity with GitLab concepts (projects, pipelines, jobs)
- .env file follows standard format: `GL_PAT=your_token_here`
- Default GitLab instance is gitlab.com unless specified otherwise
- Users may work with both public and private projects (depending on token permissions)
- Network connectivity to GitLab API is available
- GitLab API response times are reasonable (<2s for most operations)

## Dependencies

- GitLab API v4 availability and stability
- Network access to GitLab instance (gitlab.com or self-hosted)
- Valid GL_PAT token with appropriate scopes
- .env file support (already exists in RAPS via dotenvy crate)

## Out of Scope

- GitLab repository management (clone, push, pull operations)
- GitLab merge request management
- GitLab issue management
- GitLab user/group management
- GitLab CI/CD configuration file editing
- GitLab webhook management
- GitLab container registry operations
- Multi-factor authentication flows
- GitLab OAuth authentication (only PAT tokens supported)

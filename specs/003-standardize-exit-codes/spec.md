# Feature Specification: Standardize Exit Codes

**Feature Branch**: `003-standardize-exit-codes`
**Created**: 2026-01-15
**Status**: Draft
**Input**: User description: "Standardize exit codes: 0 success, 2 invalid args, 3 auth failure, 4 not found, 5 remote error, 6 internal error. Document in docs/cli/exit-codes.md."

## User Scenarios & Testing

### User Story 1 - Scriptable Error Handling (Priority: P1)

As a DevOps engineer writing automation scripts, I need the CLI to return specific exit codes for different failure types so that my script can decide whether to retry (e.g. on remote error) or fail fast (e.g. on invalid args).

**Why this priority**: Essential for "Automation-First" principle; currently all errors return generic code 1.

**Independent Test**:
- Run a command with invalid args -> verify exit code 2
- Run a command requiring auth without login -> verify exit code 3
- Run a command targeting non-existent resource -> verify exit code 4

**Acceptance Scenarios**:

1. **Given** a command invoked with unknown flags, **When** executed, **Then** it prints help to stderr and exits with code 2.
2. **Given** no active session, **When** `raps bucket list` is run, **Then** it prints "Not logged in" to stderr and exits with code 3.
3. **Given** a valid session, **When** `raps bucket info non-existent-bucket` is run, **Then** it exits with code 4.
4. **Given** network disconnection, **When** `raps bucket list` is run, **Then** it exits with code 5.

---

### User Story 2 - Documentation (Priority: P2)

As a developer, I need a reference for exit codes so that I can write correct error handling logic in my wrapper scripts.

**Why this priority**: Without documentation, the standardized codes are undiscoverable.

**Independent Test**:
- Verify `docs/cli/exit-codes.md` exists and lists all codes.

**Acceptance Scenarios**:

1. **Given** the documentation site, **When** I navigate to CLI reference, **Then** I find a page defining exit codes 0-6.

## Requirements

### Functional Requirements

- **FR-001**: The CLI MUST return exit code 0 for successful execution.
- **FR-002**: The CLI MUST return exit code 2 for invalid arguments or validation failures (clap errors).
- **FR-003**: The CLI MUST return exit code 3 for authentication or authorization failures (401/403 from API).
- **FR-004**: The CLI MUST return exit code 4 for resource not found errors (404 from API).
- **FR-005**: The CLI MUST return exit code 5 for remote/API errors (5xx, timeouts, connection refused).
- **FR-006**: The CLI MUST return exit code 6 for internal application errors (panics, IO errors not covered above).
- **FR-007**: All error messages MUST be printed to STDERR.

### Key Entities

- **ExitCode**: Enum mapping error types to integer codes.

## Success Criteria

### Measurable Outcomes

- **SC-001**: 100% of CLI commands return non-1 exit codes for specific error categories.
- **SC-002**: CI pipelines can successfully distinguish between transient errors (code 5) and permanent errors (code 2/4).
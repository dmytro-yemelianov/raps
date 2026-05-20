# Feature Specification: CLI Command Validation & Test Coverage

**Feature Branch**: `008-cli-validation`
**Created**: 2026-03-13
**Status**: Active
**Input**: Manual testing session revealed 5 bugs in v5.7.0; need systematic validation of all 250+ commands and automated test gap analysis.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Offline CLI Validation (Priority: P1)

Commands that can be validated without APS credentials: help text, argument parsing, error messages, output formatting, local-only operations.

**Why this priority**: These tests are deterministic, fast, and catch the most common regressions (clap struct mismatches, missing flags, wrong defaults).

**Independent Test**: Run each command with `--help`, missing args, invalid args, and verify exit codes + output.

**Acceptance Scenarios**:

1. **Given** no credentials, **When** `raps <cmd> --help`, **Then** exit 0 and help text contains all documented flags
2. **Given** no credentials, **When** required positional arg missing, **Then** exit 2 and error mentions the missing argument
3. **Given** no credentials, **When** `--output json` flag used, **Then** exit code unchanged, output format respected
4. **Given** invalid input (injection, empty string), **When** command invoked, **Then** rejected with clear error, no panic

---

### User Story 2 - Authenticated Read Operations (Priority: P2)

Commands that read data from APS APIs using 2-legged (client credentials) or 3-legged (OAuth) auth.

**Why this priority**: Validates API integration, response parsing, and the ACC/BIM 360 fallback pattern without risk of data mutation.

**Independent Test**: With valid credentials, run list/get/info/status commands and verify output is parseable.

**Acceptance Scenarios**:

1. **Given** 2-legged auth, **When** `raps bucket list`, **Then** exit 0 and output contains bucket data
2. **Given** 3-legged auth, **When** `raps hub list`, **Then** exit 0 and output contains hub data
3. **Given** valid auth + `--output json`, **When** any read command, **Then** output is valid JSON
4. **Given** expired/invalid token, **When** any read command, **Then** clear auth error, not a panic

---

### User Story 3 - Authenticated Write Operations (Priority: P3)

Commands that create, update, or delete resources via APS APIs.

**Why this priority**: Highest risk (mutates data) but essential for verifying full CRUD lifecycle.

**Independent Test**: In a test account, run create/update/delete lifecycle for each resource type.

**Acceptance Scenarios**:

1. **Given** valid auth + test account, **When** CRUD lifecycle executed, **Then** resource created, read, updated, deleted
2. **Given** valid auth, **When** create with missing required field, **Then** clear error from API, not 500/panic
3. **Given** valid auth, **When** delete non-existent resource, **Then** 404 with clear message

---

### Edge Cases

- What happens when positional args conflict with stdin (`-`) marker?
- How does the system handle BIM 360 vs ACC API fallback?
- What happens with empty bucket/project IDs?
- How are special characters in object keys handled?
- What happens when temp files escape the sandbox?
- How does the lint secret scanner handle source code vs real secrets?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every command/subcommand MUST have a `--help` test that exits 0
- **FR-002**: Every command with required args MUST have a missing-arg test that exits 2
- **FR-003**: Every command MUST accept `--output json` without error (where applicable)
- **FR-004**: Commands with default values MUST have tests verifying the defaults match API expectations
- **FR-005**: Commands with interactive prompts MUST have non-interactive fallback tests
- **FR-006**: Secret scanning MUST NOT false-positive on source code patterns (get_token, self.auth, etc.)
- **FR-007**: Temp files MUST stay within working directory sandbox

### Key Entities

- **Command**: A top-level CLI verb (e.g., `object`, `admin`, `issue`)
- **Subcommand**: A second-level verb (e.g., `object upload`, `admin user list`)
- **Test Category**: help, args-validation, output-format, non-interactive, error-handling, live-api

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of commands have --help tests
- **SC-002**: 100% of commands with required args have missing-arg tests
- **SC-003**: All 5 bugs from manual testing have regression tests
- **SC-004**: Lint L020 false positive rate drops from 533 to <10 on the raps workspace
- **SC-005**: All default values (priority casing, status fields) match API expectations

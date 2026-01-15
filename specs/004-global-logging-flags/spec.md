# Feature Specification: Global Logging Flags

**Feature Branch**: `004-global-logging-flags`
**Created**: 2026-01-15
**Status**: Draft
**Input**: User description: "Global logging flags: --no-color, --quiet, --verbose, --debug. Redact secrets in debug logs."

## User Scenarios & Testing

### User Story 1 - Clean CI Output (Priority: P1)

As a CI engineer, I want to disable colors and suppress non-essential output so that my build logs are readable and not cluttered.

**Why this priority**: Essential for CI/CD adoption.

**Independent Test**:
- Run `raps bucket list --no-color` -> verify no ANSI codes.
- Run `raps bucket list --quiet` -> verify only JSON/Table data, no "Fetching..." logs.

**Acceptance Scenarios**:

1. **Given** a command with `--no-color`, **When** executed, **Then** output contains no escape sequences.
2. **Given** a command with `--quiet`, **When** executed, **Then** only the result payload is printed (no info logs).

---

### User Story 2 - Troubleshooting (Priority: P2)

As a developer debugging an issue, I want to see verbose request logs or full debug traces (with secrets redacted) to understand what's happening.

**Why this priority**: Critical for support and self-diagnosis.

**Independent Test**:
- Run `raps auth test --verbose` -> verify "GET https://..." log.
- Run `raps auth test --debug` -> verify "GET https://..." log AND redacted headers/secrets if applicable.

**Acceptance Scenarios**:

1. **Given** `--verbose`, **When** a command runs, **Then** it prints method and URL for every HTTP request to stderr.
2. **Given** `--debug`, **When** a command runs, **Then** it prints headers and bodies (optional/future) OR just more detailed trace.
3. **CRITICAL**: **Given** any debug output containing a secret (e.g. `client_secret=...`), **When** displayed, **Then** the secret value is replaced with `[REDACTED]`.

## Requirements

### Functional Requirements

- **FR-001**: `--no-color` MUST disable all ANSI color codes in stdout/stderr.
- **FR-002**: `--quiet` MUST suppress all `info` level logs (e.g. "Creating bucket..."), printing only the final result (stdout) and errors (stderr).
- **FR-003**: `--verbose` MUST enable logging of HTTP request method, URL, and response status code.
- **FR-004**: `--debug` MUST enable verbose logging AND include redaction logic for any printed message.
- **FR-005**: Secrets (client_secret, access_token, refresh_token, api_key) MUST be redacted in all log output.

### Key Entities

- **Logger**: Global state manager for verbosity.
- **SecretRedactor**: Utility to sanitize strings.

## Success Criteria

### Measurable Outcomes

- **SC-001**: CI logs are 100% free of ANSI codes when `--no-color` is used.
- **SC-002**: No secret leak in logs during a full regression test suite run with `--debug`.
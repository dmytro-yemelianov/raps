# Feature Specification: Non-interactive Mode

**Feature Branch**: `005-non-interactive-mode`
**Created**: 2026-01-15
**Status**: Draft
**Input**: User description: "Non-interactive mode and bypass prompts: commands must accept all params via flags and fail if prompts required."

## User Scenarios & Testing

### User Story 1 - Parameterized Creation (Priority: P1)

As a script author, I want to create resources (buckets, translations, issues) by passing all parameters as flags, so that the command runs without pausing for input.

**Why this priority**: Core requirement for automation.

**Independent Test**:
- `raps bucket create --key my-bucket --policy transient --region US` (should succeed without prompts)
- `raps translate start --urn <urn> --format svf2` (should succeed without prompts)

**Acceptance Scenarios**:

1. **Given** a command with all required flags, **When** executed, **Then** it completes without user interaction.
2. **Given** a command missing a required flag in `--non-interactive` mode, **When** executed, **Then** it fails with exit code 2 and an error message like "Missing required argument '--key'".

---

### User Story 2 - Auto-Confirmation (Priority: P2)

As a script author, I want to delete resources using `--yes` to bypass "Are you sure?" prompts.

**Why this priority**: Destructive actions currently block automation.

**Independent Test**:
- `raps bucket delete my-bucket --yes` (should delete immediately)

**Acceptance Scenarios**:

1. **Given** `raps bucket delete <key> --yes`, **When** executed, **Then** it deletes the bucket without asking for confirmation.
2. **Given** `raps bucket delete <key>` (without --yes), **When** executed in `--non-interactive` mode, **Then** it fails with "Confirmation required (use --yes)".

## Requirements

### Functional Requirements

- **FR-001**: All "create" commands MUST support flags for all mandatory parameters.
- **FR-002**: If a required parameter is missing and `--non-interactive` is set (or not a TTY), the command MUST fail with a clear error.
- **FR-003**: If a required parameter is missing and interactive mode is active, the command MAY prompt (current behavior).
- **FR-004**: Destructive commands (delete) MUST support `--yes` to bypass confirmation.
- **FR-005**: Destructive commands MUST fail in `--non-interactive` mode if `--yes` is not provided.

### Key Entities

- **Interactive Check**: Logic to determine if prompts are allowed (`interactive::confirm`, `prompts::input`).

## Success Criteria

### Measurable Outcomes

- **SC-001**: 100% of "create" commands (bucket, issue, folder, webhook, reality) can be executed non-interactively.
- **SC-002**: 100% of "delete" commands support `--yes`.
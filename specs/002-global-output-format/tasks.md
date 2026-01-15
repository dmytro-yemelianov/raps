---
description: "Task list template for feature implementation"
---

# Tasks: Global Output Format Standardization

**Input**: Design documents from `/specs/002-global-output-format/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md

**Tests**: Tests are MANDATORY for all new features per Constitution Principle V.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [x] T001 Create output module directory structure in raps-cli/src/output/
- [x] T002 [P] Add `serde_yaml` to raps-cli/Cargo.toml (if not present) and verify workspace dependencies
- [x] T003 [P] Create `OutputFormat` enum in raps-cli/src/output/mod.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Implement `OutputFormatter` struct and `print_output` generic function in raps-cli/src/output/formatter.rs
- [x] T005 Implement `clap` global `--output` flag in raps-cli/src/main.rs (or cli.rs)
- [x] T006 Implement TTY detection logic using `std::io::IsTerminal` in raps-cli/src/output/mod.rs
- [x] T007 Create basic unit tests for `OutputFormatter` (JSON/Table) in raps-cli/src/output/tests.rs

**Checkpoint**: Foundation ready - global flag exists and formatter structure is in place

---

## Phase 3: User Story 1 - YAML Output for Configuration (Priority: P1) 🎯 MVP

**Goal**: Enable YAML output for all commands to support DevOps workflows

**Independent Test**: `raps bucket list --output yaml` returns valid YAML

### Tests for User Story 1 (MANDATORY) ⚠️

- [x] T008 [P] [US1] Create integration test for YAML output in tests/output_format_test.rs

### Implementation for User Story 1

- [x] T009 [US1] Implement `serde_yaml` serialization logic in `OutputFormatter` (raps-cli/src/output/formatter.rs)
- [x] T010 [US1] Wire up global flag to pass format selection to `OutputFormatter` in raps-cli/src/main.rs
- [x] T011 [US1] Update a representative command (e.g., `bucket list`) to use the new `print_output` function
- [x] T012 [US1] Verify error handling: ensure serialization errors return non-zero exit code

**Checkpoint**: Users can now get YAML output from updated commands

---

## Phase 4: User Story 2 - Reliable CI/CD Parsing (Priority: P2)

**Goal**: Ensure consistent JSON output and auto-detection for CI pipelines

**Independent Test**: Piping `raps bucket list` to `jq` works without flags

### Tests for User Story 2 (MANDATORY) ⚠️

- [x] T013 [P] [US2] Create integration test for TTY detection and default JSON behavior in tests/output_tty_test.rs
- [x] T014 [P] [US2] Create JSON schema validation test for core resources in tests/json_schema_test.rs

### Implementation for User Story 2

- [x] T015 [US2] Refine `OutputFormatter` to default to JSON when `!is_terminal()` and no flag provided (raps-cli/src/output/mod.rs)
- [x] T016 [US2] Ensure all log/status messages are directed to stderr, keeping stdout clean for JSON data
- [x] T017 [US2] Update `auth status`, `da engines`, and other core commands to use `print_output`

**Checkpoint**: CI/CD pipelines can reliably consume RAPS output

---

## Phase 5: User Story 3 - Global Consistency (Priority: P3)

**Goal**: Apply formatting standardization to ALL CLI commands

**Independent Test**: Verify `--output` works on `auth`, `config`, and edge case commands

### Tests for User Story 3 (MANDATORY) ⚠️

- [x] T018 [P] [US3] Create comprehensive test suite checking `--output` support across all subcommands in tests/output_consistency_test.rs

### Implementation for User Story 3

- [x] T019 [US3] Refactor `raps-auth` commands to use `print_output`
- [x] T020 [US3] Refactor `raps-config` commands to use `print_output`
- [x] T021 [US3] Refactor `raps-da` commands to use `print_output`
- [x] T022 [US3] Refactor remaining modules (`oss`, `dm`, `derivative`, etc.) to use `print_output`
- [x] T023 [US3] Handle edge cases: Empty lists return `[]`, not "No items found" text

**Checkpoint**: All commands behave consistently

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T024 [P] Update README.md with global `--output` flag documentation
- [x] T025 [P] Update command-specific help text to mention output formats
- [x] T026 Audit code for any remaining `println!` calls that should be `eprintln!` or formatted output
- [x] T027 Run full test suite and `cargo clippy` to ensure quality gates pass

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - US1 (YAML) is the MVP
  - US2 (CI/CD) refines the behavior
  - US3 (Consistency) scales it to the whole app

### Parallel Opportunities

- T002 and T003 in Setup
- Tests (T008, T013, T014, T018) can be written in parallel with implementation
- Refactoring commands in US3 (T019-T022) can be parallelized across developers

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Setup & Foundation (T001-T007)
2. Implement YAML support (T009) & Wire flag (T010)
3. Convert ONE command (`bucket list`) to prove it works (T011)
4. **STOP and VALIDATE**: Verify `raps bucket list --output yaml` works.

### Incremental Delivery

1. **Iteration 1**: MVP (YAML support + 1 command)
2. **Iteration 2**: CI/CD Logic (Auto-JSON + stderr logging)
3. **Iteration 3**: Rollout to all commands (The "Grind")

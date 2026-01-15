---
description: "Task list template for feature implementation"
---

# Tasks: Standardize Exit Codes

**Input**: Design documents from `/specs/003-standardize-exit-codes/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md

**Tests**: Mandatory per Constitution.

## Phase 1: Setup

- [x] T001 Define `ExitCode` enum in raps-kernel/src/error.rs
- [x] T002 Implement `ExitCode::from_error` logic mapping `anyhow::Error` to codes in raps-kernel/src/error.rs

## Phase 2: User Story 1 - Scriptable Error Handling

**Goal**: Standardize CLI exit behavior

- [x] T003 [US1] Create integration test verifying exit codes in tests/exit_codes_test.rs
- [x] T004 [US1] Update raps-cli/src/main.rs to use `ExitCode::from_error` and `exit()` on error
- [x] T005 [US1] Verify `reqwest` errors map correctly to 3 (Auth), 4 (NotFound), 5 (Remote)

## Phase 3: User Story 2 - Documentation

**Goal**: Document codes for users

- [x] T006 [P] [US2] Create docs/cli/exit-codes.md with table of codes

## Phase 4: Polish

- [x] T007 Run `cargo test` to verify exit code behavior

---
description: "Task list template for feature implementation"
---

# Tasks: Global Logging Flags

**Input**: Design documents from `/specs/004-global-logging-flags/`
**Prerequisites**: plan.md, spec.md, research.md

**Tests**: Mandatory per Constitution.

## Phase 1: Core Implementation

- [x] T001 Update `raps-kernel/src/logging.rs` to use `redact_secrets` in `log_verbose`, `log_debug`, `log_request`, and `log_response`
- [x] T002 Update `raps-kernel/src/logging.rs` `redact_secrets` to be public (remove dead_code)

## Phase 2: User Story 1 - Clean CI Output

- [x] T003 [US1] Create integration test verifying `--quiet` and `--no-color` in tests/logging_test.rs
- [x] T004 [US1] Verify `main.rs` passes flags to `logging::init` (already done, but verify)

## Phase 3: User Story 2 - Troubleshooting

- [x] T005 [US2] Create test ensuring secrets are redacted in debug output (tests/redaction_test.rs)
- [x] T006 [US2] Audit usages of `println!` vs `eprintln!` vs `logging::log_verbose`

## Phase 4: Polish

- [x] T007 Run `cargo test`

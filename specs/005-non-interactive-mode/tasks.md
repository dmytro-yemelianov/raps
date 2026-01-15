---
description: "Task list template for feature implementation"
---

# Tasks: Non-interactive Mode

**Input**: Design documents from `/specs/005-non-interactive-mode/`
**Prerequisites**: plan.md, spec.md, research.md

**Tests**: Mandatory per Constitution.

## Phase 1: Core Logic

- [x] T001 Update `raps-kernel/src/prompts.rs` to check `interactive::is_interactive()` and fail-fast with `ExitCode::InvalidArguments` (or descriptive error) if prompting would be required but is not allowed.

## Phase 2: User Story 1 - Parameterized Creation

**Goal**: Ensure create commands accept flags

- [x] T002 [US1] Audit `raps-cli/src/commands/bucket.rs` (create) - Ensure all prompts have flag fallbacks.
- [x] T003 [US1] Audit `raps-cli/src/commands/translate.rs` (start)
- [x] T004 [US1] Audit `raps-cli/src/commands/issue.rs` (create)
- [x] T005 [US1] Audit `raps-cli/src/commands/reality.rs` (create)
- [x] T006 [US1] Audit `raps-cli/src/commands/folder.rs` (create)
- [x] T007 [US1] Audit `raps-cli/src/commands/webhook.rs` (create)
- [x] T008 [US1] Create integration test verifying non-interactive failure when args missing (tests/non_interactive_test.rs)

## Phase 3: User Story 2 - Auto-Confirmation

**Goal**: Delete without prompt

- [x] T009 [US2] Audit `raps-cli/src/commands/bucket.rs` (delete) - Ensure `confirm_destructive` respects `yes` flag and non-interactive mode.
- [x] T010 [US2] Audit other delete commands (object, folder, webhook, etc.)

## Phase 4: Polish

- [x] T011 Run `cargo test`

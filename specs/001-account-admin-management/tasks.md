# Tasks: Account Admin Bulk Management Tool

**Input**: Design documents from `/specs/001-account-admin-management/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are MANDATORY per Constitution Principle V. Each user story includes test tasks.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md structure:
- **raps-admin/**: New crate for bulk operations
- **raps-acc/**: Extended existing crate for API clients
- **raps-cli/**: CLI command implementations

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and crate structure creation

- [x] T001 Create raps-admin crate directory structure per plan.md in raps-admin/
- [x] T002 Initialize raps-admin/Cargo.toml with workspace dependencies (tokio, serde, uuid, directories, thiserror, indicatif)
- [x] T003 [P] Create raps-admin/src/lib.rs with module declarations and re-exports
- [x] T004 [P] Add raps-admin to workspace members in Cargo.toml
- [x] T005 [P] Create raps-admin/src/error.rs with AdminError enum per contracts/api-contracts.md

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Core Types and Pagination (Shared by all stories)

- [x] T006 [P] Create core types in raps-acc/src/types.rs (AccountUser, AccountProject, ProjectUser, PaginatedResponse, Pagination)
- [x] T007 [P] Create raps-admin/src/types.rs with BulkConfig, OperationType, OperationStatus, OperationProgress, ItemResult
- [x] T008 [P] Create raps-admin/src/filter.rs with ProjectFilter struct and from_expression parser

### Account Admin API Client (Required for user lookup)

- [x] T009 Create AccountAdminClient struct in raps-acc/src/admin.rs with new() and new_with_http_config()
- [x] T010 Implement find_user_by_email() in raps-acc/src/admin.rs for email-to-userId lookup
- [x] T011 Implement list_projects() with pagination in raps-acc/src/admin.rs
- [x] T012 Add unit tests for AccountAdminClient in raps-acc/src/admin.rs

### Bulk Execution Engine (Required for all bulk operations)

- [x] T013 Create StateManager struct in raps-admin/src/bulk/state.rs with state directory initialization
- [x] T014 Implement create_operation() and load_operation() in raps-admin/src/bulk/state.rs
- [x] T015 Implement update_state() for tracking item completion in raps-admin/src/bulk/state.rs
- [x] T016 Create BulkExecutor struct in raps-admin/src/bulk/executor.rs with BulkConfig
- [x] T017 Implement execute() with configurable concurrency using tokio semaphore in raps-admin/src/bulk/executor.rs
- [x] T018 Implement retry logic with exponential backoff in raps-admin/src/bulk/retry.rs
- [x] T019 [P] Create ProgressUpdate struct and progress callback infrastructure in raps-admin/src/bulk/executor.rs
- [x] T020 Add unit tests for StateManager in raps-admin/tests/state_tests.rs
- [x] T021 Add unit tests for BulkExecutor in raps-admin/tests/bulk_tests.rs

### CLI Foundation

- [x] T022 Create raps-cli/src/commands/admin.rs with AdminCommands enum and subcommand structure
- [x] T023 Integrate admin commands into raps-cli/src/commands/mod.rs and main CLI

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Bulk Add User to Multiple Projects (Priority: P1) 🎯 MVP

**Goal**: Enable account admins to add a user to thousands of projects in a single operation with progress tracking and retry logic

**Independent Test**: Add a user to 10 mock projects and verify success/skip/fail results are correctly reported

### Tests for User Story 1

- [x] T024 [P] [US1] Integration test for bulk_add_user with mock API in raps-admin/tests/integration/add_user_tests.rs
- [x] T025 [P] [US1] Unit test for ProjectUsersClient.add_user in raps-acc/src/users.rs

### Implementation for User Story 1

- [x] T026 [P] [US1] Create ProjectUsersClient struct in raps-acc/src/users.rs with new() constructor
- [x] T027 [US1] Implement add_user() in raps-acc/src/users.rs (POST /projects/{projectId}/users)
- [x] T028 [US1] Implement user_exists() in raps-acc/src/users.rs for duplicate detection
- [x] T029 [US1] Create add_user.rs in raps-admin/src/operations/ with bulk_add_user function
- [x] T030 [US1] Implement duplicate handling (skip if exists, report as "already_exists") in raps-admin/src/operations/add_user.rs
- [x] T031 [US1] Implement `raps admin user add` CLI command in raps-cli/src/commands/admin.rs
- [x] T032 [US1] Add --role, --filter, --concurrency, --dry-run flags to user add command
- [x] T033 [US1] Implement progress bar display with indicatif in raps-cli/src/commands/admin.rs
- [x] T034 [US1] Add JSON/YAML/CSV output format support for user add results

**Checkpoint**: User Story 1 complete - bulk add user is fully functional with CLI, progress tracking, and retry

---

## Phase 4: User Story 2 - Bulk Update User Roles Across Projects (Priority: P1)

**Goal**: Enable account admins to update a user's role across all assigned projects in a single operation

**Independent Test**: Update a user's role from "Viewer" to "Editor" across 10 mock projects and verify role changes

### Tests for User Story 2

- [x] T035 [P] [US2] Integration test for bulk_update_role with mock API in raps-admin/tests/integration/update_role_tests.rs
- [x] T036 [P] [US2] Unit test for ProjectUsersClient.update_user in raps-acc/src/users.rs

### Implementation for User Story 2

- [x] T037 [US2] Implement update_user() in raps-acc/src/users.rs (PATCH /projects/{projectId}/users/{userId})
- [x] T038 [US2] Implement list_project_users() with pagination in raps-acc/src/users.rs
- [x] T039 [US2] Create update_role.rs in raps-admin/src/operations/ with bulk_update_role function
- [x] T040 [US2] Implement --from-role filter for selective role updates in raps-admin/src/operations/update_role.rs
- [x] T041 [US2] Implement `raps admin user update` CLI command in raps-cli/src/commands/admin.rs
- [x] T042 [US2] Add --role (required), --from-role (optional), --filter flags to user update command

**Checkpoint**: User Story 2 complete - bulk role update is fully functional

---

## Phase 5: User Story 3 - Bulk Remove User from Projects (Priority: P2)

**Goal**: Enable account admins to remove a user from all selected projects for offboarding

**Independent Test**: Remove a user from 10 mock projects and verify they no longer appear in project member lists

### Tests for User Story 3

- [ ] T043 [P] [US3] Integration test for bulk_remove_user with mock API in raps-admin/tests/integration/remove_user_tests.rs
- [ ] T044 [P] [US3] Unit test for ProjectUsersClient.remove_user in raps-acc/src/users.rs

### Implementation for User Story 3

- [ ] T045 [US3] Implement remove_user() in raps-acc/src/users.rs (DELETE /projects/{projectId}/users/{userId})
- [ ] T046 [US3] Create remove_user.rs in raps-admin/src/operations/ with bulk_remove_user function
- [ ] T047 [US3] Handle "user not in project" as skip (not error) in raps-admin/src/operations/remove_user.rs
- [ ] T048 [US3] Implement `raps admin user remove` CLI command in raps-cli/src/commands/admin.rs

**Checkpoint**: User Story 3 complete - bulk user removal is fully functional

---

## Phase 6: User Story 4 - View and Filter Active Projects (Priority: P2)

**Goal**: Enable account admins to list and filter projects before performing bulk operations

**Independent Test**: List projects and apply name/status/platform filters, verify filtered results match criteria

### Tests for User Story 4

- [ ] T049 [P] [US4] Unit test for ProjectFilter.matches() and from_expression() in raps-admin/src/filter.rs
- [ ] T050 [P] [US4] Integration test for list_projects with filters in raps-acc/src/admin.rs

### Implementation for User Story 4

- [ ] T051 [US4] Implement get_project() in raps-acc/src/admin.rs for single project lookup
- [ ] T052 [US4] Implement ProjectFilter.matches() with glob pattern matching in raps-admin/src/filter.rs
- [ ] T053 [US4] Implement `raps admin project list` CLI command in raps-cli/src/commands/admin.rs
- [ ] T054 [US4] Add --filter, --status, --platform, --limit flags to project list command
- [ ] T055 [US4] Implement CSV export for project lists (--output csv)

**Checkpoint**: User Story 4 complete - project listing and filtering is fully functional

---

## Phase 7: User Story 5 - Bulk Manage Folder Rights (Priority: P2)

**Goal**: Enable account admins to update folder-level permissions across multiple projects

**Independent Test**: Update folder permissions for a user across 10 mock projects and verify permission changes

### Tests for User Story 5

- [ ] T056 [P] [US5] Integration test for bulk_update_folder_rights with mock API in raps-admin/tests/integration/folder_rights_tests.rs
- [ ] T057 [P] [US5] Unit test for FolderPermissionsClient methods in raps-acc/src/permissions.rs

### Implementation for User Story 5

- [ ] T058 [P] [US5] Create FolderPermissionsClient struct in raps-acc/src/permissions.rs
- [ ] T059 [US5] Implement get_permissions() in raps-acc/src/permissions.rs (GET /folders/{folderId}/permissions)
- [ ] T060 [US5] Implement update_permissions() batch operation in raps-acc/src/permissions.rs
- [ ] T061 [US5] Create folder_rights.rs in raps-admin/src/operations/ with bulk_update_folder_rights function
- [ ] T062 [US5] Implement permission level mapping (ViewOnly → actions array) in raps-admin/src/operations/folder_rights.rs
- [ ] T063 [US5] Implement `raps admin folder rights` CLI command in raps-cli/src/commands/admin.rs
- [ ] T064 [US5] Add --level, --folder flags with predefined permission levels

**Checkpoint**: User Story 5 complete - bulk folder rights management is fully functional

---

## Phase 8: User Story 6 - Preview and Dry Run Operations (Priority: P3)

**Goal**: Enable admins to preview bulk operations before execution to reduce risk

**Independent Test**: Run --dry-run on a bulk add operation and verify no actual API calls are made while a preview report is generated

### Tests for User Story 6

- [ ] T065 [P] [US6] Integration test for dry-run mode verifying no API mutations in raps-admin/tests/integration/dry_run_tests.rs

### Implementation for User Story 6

- [ ] T066 [US6] Add dry_run flag to BulkConfig in raps-admin/src/types.rs
- [ ] T067 [US6] Implement dry-run bypass in BulkExecutor.execute() in raps-admin/src/bulk/executor.rs
- [ ] T068 [US6] Generate preview report showing affected projects without executing in raps-admin/src/report.rs
- [ ] T069 [US6] Implement --dry-run flag for all bulk commands in raps-cli/src/commands/admin.rs

**Checkpoint**: User Story 6 complete - dry-run preview is fully functional

---

## Phase 9: Operation Management (Cross-cutting)

**Goal**: Enable resume, cancel, and status checking for long-running operations

### Tests

- [ ] T070 [P] Unit test for StateManager.get_resumable_operation() in raps-admin/tests/state_tests.rs
- [ ] T071 [P] Integration test for operation resume functionality in raps-admin/tests/integration/resume_tests.rs

### Implementation

- [ ] T072 Implement resume() in BulkExecutor to continue from last successful point in raps-admin/src/bulk/executor.rs
- [ ] T073 Implement cancel() to gracefully stop in-progress operations in raps-admin/src/bulk/executor.rs
- [ ] T074 Implement list_operations() and get_resumable_operation() in StateManager in raps-admin/src/bulk/state.rs
- [ ] T075 Implement `raps admin operation status` CLI command in raps-cli/src/commands/admin.rs
- [ ] T076 Implement `raps admin operation resume` CLI command in raps-cli/src/commands/admin.rs
- [ ] T077 Implement `raps admin operation cancel` CLI command in raps-cli/src/commands/admin.rs

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T078 [P] Create report.rs with result formatting (table, JSON, YAML, CSV) in raps-admin/src/report.rs
- [ ] T079 [P] Add verbose/debug logging for all API calls using existing raps-kernel logging
- [ ] T080 Implement rate limit header monitoring (X-RateLimit-Remaining) in raps-acc API clients
- [ ] T081 Add adaptive throttling when rate limits are approaching in raps-admin/src/bulk/executor.rs
- [ ] T082 [P] Update raps-acc/src/lib.rs to export new clients (AccountAdminClient, ProjectUsersClient, FolderPermissionsClient)
- [ ] T083 [P] Add CLI help text and examples for all admin commands
- [ ] T084 Run quickstart.md validation scenarios manually
- [ ] T085 Run cargo fmt and cargo clippy --all-features on all new code
- [ ] T086 Run cargo test --workspace to verify all tests pass

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-8)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3)
- **Operation Management (Phase 9)**: Can run in parallel with P2/P3 stories after Phase 2
- **Polish (Phase 10)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - Reuses ProjectUsersClient from US1
- **User Story 3 (P2)**: Can start after Foundational (Phase 2) - Reuses ProjectUsersClient from US1/US2
- **User Story 4 (P2)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 5 (P2)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 6 (P3)**: Can start after Foundational (Phase 2) - Enhances all other stories

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- API client methods before bulk operation functions
- Bulk operation functions before CLI commands
- Core implementation before output formatting

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes:
  - US1 and US4 can run in parallel (no shared components)
  - US2 and US3 should follow US1 (share ProjectUsersClient)
  - US5 can run in parallel (separate FolderPermissionsClient)
  - US6 can integrate after any story

---

## Parallel Example: Foundational Phase

```bash
# Launch all parallel foundational tasks together:
Task: "Create core types in raps-acc/src/types.rs"
Task: "Create raps-admin/src/types.rs with BulkConfig..."
Task: "Create raps-admin/src/filter.rs with ProjectFilter..."
```

## Parallel Example: User Story 1

```bash
# Launch tests first (should fail):
Task: "Integration test for bulk_add_user..."
Task: "Unit test for ProjectUsersClient.add_user..."

# After tests fail, launch implementation:
Task: "Create ProjectUsersClient struct in raps-acc/src/users.rs..."
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Bulk Add User)
4. **STOP and VALIDATE**: Test bulk add with real ACC account
5. Deploy/demo if ready - this alone delivers significant value

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test → Demo (MVP!)
3. Add User Story 2 → Test → Demo (Role updates)
4. Add User Story 4 → Test → Demo (Project filtering)
5. Add User Story 3 → Test → Demo (User removal)
6. Add User Story 5 → Test → Demo (Folder rights)
7. Add User Story 6 → Test → Demo (Dry-run preview)

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 + User Story 2 (user operations)
   - Developer B: User Story 4 + User Story 5 (project/folder operations)
3. Then:
   - Developer A: User Story 3 (removal)
   - Developer B: User Story 6 (dry-run)
4. Polish phase together

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- BIM 360 write operations require legacy API - handle platform detection in ProjectUsersClient

# Tasks: MCP Server Native Authentication Support

**Input**: Design documents from `/specs/001-mcp-native-auth/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Per Constitution Principle V, new features require tests.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

- **Rust workspace**: `raps-cli/src/mcp/` for MCP module
- **Tests**: `raps-cli/tests/` for integration tests

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create new module and update module structure

- [x] T001 Create `auth_guidance.rs` module file in raps-cli/src/mcp/auth_guidance.rs
- [x] T002 Add `mod auth_guidance;` declaration in raps-cli/src/mcp/mod.rs
- [x] T003 [P] Add `AuthRequirement` enum in raps-cli/src/mcp/auth_guidance.rs
- [x] T004 [P] Add `AuthState` struct in raps-cli/src/mcp/auth_guidance.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T005 Implement `SETUP_INSTRUCTIONS` constant with full onboarding guide in raps-cli/src/mcp/auth_guidance.rs
- [x] T006 [P] Implement `MISSING_CLIENT_ID` constant in raps-cli/src/mcp/auth_guidance.rs
- [x] T007 [P] Implement `MISSING_CLIENT_SECRET` constant in raps-cli/src/mcp/auth_guidance.rs
- [x] T008 [P] Implement `THREE_LEGGED_PROMPT` constant in raps-cli/src/mcp/auth_guidance.rs
- [x] T009 Implement `get_tool_auth_requirement()` function with static tool-to-auth mapping in raps-cli/src/mcp/auth_guidance.rs
- [x] T010 Implement `get_auth_state()` helper function to compute current auth status in raps-cli/src/mcp/auth_guidance.rs
- [x] T011 Implement `format_error_guidance()` function for error-to-actionable-message conversion in raps-cli/src/mcp/auth_guidance.rs

**Checkpoint**: Foundation ready - auth guidance module complete, user story implementation can begin

---

## Phase 3: User Story 1 - First-Time Setup Guidance (Priority: P1)

**Goal**: Provide clear, actionable instructions when credentials are missing or invalid

**Independent Test**: Start MCP server without credentials, call `auth_status`, verify helpful guidance is returned

### Tests for User Story 1

- [ ] T012 [P] [US1] Create test for `auth_status` with no credentials in raps-cli/tests/mcp_auth_tests.rs
- [ ] T013 [P] [US1] Create test for `auth_status` with partial credentials (ID only) in raps-cli/tests/mcp_auth_tests.rs
- [ ] T014 [P] [US1] Create test for `auth_test` with invalid credentials in raps-cli/tests/mcp_auth_tests.rs

### Implementation for User Story 1

- [x] T015 [US1] Enhance `auth_status()` to detect missing credentials and return setup instructions in raps-cli/src/mcp/server.rs
- [x] T016 [US1] Enhance `auth_status()` to identify specific missing credential (ID vs secret) in raps-cli/src/mcp/server.rs
- [x] T017 [US1] Enhance `auth_test()` to return user-friendly error messages with troubleshooting steps in raps-cli/src/mcp/server.rs
- [x] T018 [US1] Update `auth_status` tool description in raps-cli/src/mcp/server.rs

**Checkpoint**: User Story 1 complete - first-time users see clear setup guidance

---

## Phase 4: User Story 2 - Proactive 3-Legged Auth Suggestions (Priority: P1)

**Goal**: Suggest 3-legged auth when user calls tools that require it without being logged in

**Independent Test**: Call `hub_list` without 3-legged token, verify response includes auth suggestion

### Tests for User Story 2

- [ ] T019 [P] [US2] Create test for 3-legged tool (hub_list) without token in raps-cli/tests/mcp_auth_tests.rs
- [ ] T020 [P] [US2] Create test for 3-legged tool with expired token in raps-cli/tests/mcp_auth_tests.rs

### Implementation for User Story 2

- [x] T021 [US2] Create `check_3leg_auth_required()` helper that wraps 3-legged tools with auth guidance in raps-cli/src/mcp/server.rs
- [x] T022 [US2] Update `hub_list()` to use auth check helper and return guidance on auth failure in raps-cli/src/mcp/server.rs
- [x] T023 [US2] Update `project_list()` to use auth check helper in raps-cli/src/mcp/server.rs
- [x] T024 [US2] Update `folder_list()`, `folder_create()` to use auth check helper in raps-cli/src/mcp/server.rs
- [x] T025 [US2] Update `item_info()`, `item_versions()` to use auth check helper in raps-cli/src/mcp/server.rs
- [x] T026 [US2] Update `issue_*` tools to use auth check helper in raps-cli/src/mcp/server.rs
- [x] T027 [US2] Update `rfi_*` and `acc_*` tools to use auth check helper in raps-cli/src/mcp/server.rs
- [x] T028 [US2] Enhance `auth_status()` to suggest 3-legged login when not logged in in raps-cli/src/mcp/server.rs

**Checkpoint**: User Story 2 complete - all 3-legged tools provide auth guidance on failure

---

## Phase 5: User Story 3 - Native 3-Legged Auth Initiation (Priority: P2)

**Goal**: Provide `auth_login` tool that initiates browser OAuth or returns device code for headless

**Independent Test**: Call `auth_login`, verify browser opens or device code is returned

### Tests for User Story 3

- [ ] T029 [P] [US3] Create test for `auth_login` returning device code in headless mode in raps-cli/tests/mcp_auth_tests.rs
- [ ] T030 [P] [US3] Create test for `auth_login` when already logged in in raps-cli/tests/mcp_auth_tests.rs
- [ ] T031 [P] [US3] Create test for `auth_logout` clearing tokens in raps-cli/tests/mcp_auth_tests.rs

### Implementation for User Story 3

- [x] T032 [US3] Implement `auth_login()` tool function with browser detection in raps-cli/src/mcp/server.rs
- [x] T033 [US3] Add browser-based OAuth flow initiation in `auth_login()` using existing AuthClient in raps-cli/src/mcp/server.rs
- [x] T034 [US3] Add device code fallback when browser unavailable in `auth_login()` in raps-cli/src/mcp/server.rs
- [x] T035 [US3] Implement `auth_logout()` tool function to clear stored tokens in raps-cli/src/mcp/server.rs
- [x] T036 [US3] Add `auth_login` tool definition to `get_tools()` in raps-cli/src/mcp/server.rs
- [x] T037 [US3] Add `auth_logout` tool definition to `get_tools()` in raps-cli/src/mcp/server.rs
- [x] T038 [US3] Add dispatch entries for `auth_login` and `auth_logout` in raps-cli/src/mcp/server.rs
- [x] T039 [US3] Update TOOLS constant count in raps-cli/src/mcp/tools.rs (35 → 37)

**Checkpoint**: User Story 3 complete - users can initiate 3-legged auth from MCP tools

---

## Phase 6: User Story 4 - Auth Scope Awareness (Priority: P3)

**Goal**: Show users which tool categories are accessible based on current auth state

**Independent Test**: Call `auth_status` with 2-legged only, verify it shows tool availability

### Tests for User Story 4

- [ ] T040 [P] [US4] Create test for `auth_status` showing tool categories with 2-legged only in raps-cli/tests/mcp_auth_tests.rs
- [ ] T041 [P] [US4] Create test for `auth_status` showing full access with both auth types in raps-cli/tests/mcp_auth_tests.rs

### Implementation for User Story 4

- [x] T042 [US4] Implement `get_tool_availability_summary()` function in raps-cli/src/mcp/auth_guidance.rs
- [x] T043 [US4] Enhance `auth_status()` to include tool availability section based on auth state in raps-cli/src/mcp/server.rs
- [x] T044 [US4] Add category labels (OSS, Derivative, DM, ACC, Admin) to availability summary in raps-cli/src/mcp/auth_guidance.rs

**Checkpoint**: User Story 4 complete - users can self-diagnose tool access issues

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T045 Update MCP server info text to mention native auth support in raps-cli/src/mcp/server.rs
- [x] T046 [P] Run `cargo fmt` on all modified files
- [x] T047 [P] Run `cargo clippy` and fix any warnings
- [x] T048 Run all MCP auth tests with `cargo test -p raps-cli mcp_auth`
- [x] T049 [P] Update raps-website MCP documentation to reflect new auth tools
- [ ] T050 Run quickstart.md validation manually testing all auth flows

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - US1 and US2 are both P1 priority and can proceed in parallel
  - US3 (P2) can start after Foundational, independent of US1/US2
  - US4 (P3) can start after Foundational, independent of others
- **Polish (Phase 7)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: No dependencies on other stories
- **User Story 2 (P1)**: No dependencies on other stories (can run parallel with US1)
- **User Story 3 (P2)**: No dependencies on other stories
- **User Story 4 (P3)**: No dependencies on other stories

### Within Each User Story

- Tests MUST be written first and verify they compile (may not fail without mock setup)
- Helper functions before tool implementations
- Tool implementations before dispatch entries
- Update tool count after adding new tools

### Parallel Opportunities

**Phase 1 (Setup)**:
- T003 and T004 can run in parallel

**Phase 2 (Foundational)**:
- T006, T007, T008 can run in parallel (all constants)

**Phase 3-6 (User Stories)**:
- All test tasks marked [P] can run in parallel within their story
- US1 and US2 can be worked on simultaneously by different developers
- US3 and US4 can be worked on simultaneously

**Phase 7 (Polish)**:
- T046, T047, T049 can run in parallel

---

## Parallel Example: User Stories 1 & 2 Simultaneously

```bash
# Developer A: User Story 1 tests
Task: "Create test for auth_status with no credentials"
Task: "Create test for auth_status with partial credentials"
Task: "Create test for auth_test with invalid credentials"

# Developer B: User Story 2 tests (in parallel)
Task: "Create test for 3-legged tool without token"
Task: "Create test for 3-legged tool with expired token"
```

---

## Implementation Strategy

### MVP First (User Stories 1 + 2)

1. Complete Phase 1: Setup (T001-T004)
2. Complete Phase 2: Foundational (T005-T011)
3. Complete Phase 3: User Story 1 (T012-T018)
4. Complete Phase 4: User Story 2 (T019-T028)
5. **STOP and VALIDATE**: Test basic auth guidance flow
6. Ship MVP with enhanced auth_status and 3-legged suggestions

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add User Story 1 → First-time users get guidance (quick win)
3. Add User Story 2 → All tools provide auth help (major UX improvement)
4. Add User Story 3 → Full native auth flow (complete feature)
5. Add User Story 4 → Self-service diagnostics (polish)

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Run `cargo check` frequently to catch compilation errors early
- MCP server must remain functional throughout - no breaking changes

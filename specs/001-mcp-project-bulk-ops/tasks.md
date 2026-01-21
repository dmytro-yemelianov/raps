# Tasks: MCP Project Management and Bulk Operations

**Input**: Design documents from `/specs/001-mcp-project-bulk-ops/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Tests are included per Constitution Principle V (Quality & Reliability).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths in descriptions

## Path Conventions (Rust Workspace)

- **raps-cli/src/mcp/**: MCP server implementation (server.rs, tools.rs, auth_guidance.rs)
- **raps-oss/src/**: OSS client (lib.rs)
- **raps-dm/src/**: Data Management client (lib.rs)
- **raps-acc/src/**: ACC client and users module (lib.rs, users.rs)
- **raps-cli/tests/**: Integration tests

---

## Phase 1: Setup

**Purpose**: Branch setup and test infrastructure verification

- [ ] T001 Verify feature branch `001-mcp-project-bulk-ops` is active
- [ ] T002 Run `cargo check --all-features` to verify workspace compiles
- [ ] T003 [P] Run `cargo test` to verify existing tests pass

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: New crate methods required before MCP tools can be implemented

**⚠️ CRITICAL**: These methods are used by multiple user stories and must be complete first

### New Crate Methods

- [ ] T004 [P] Add `get_object_details()` method in raps-oss/src/lib.rs (returns ObjectDetails with size, SHA1, content_type, timestamps)
- [ ] T005 [P] Add `delete_item()` method in raps-dm/src/lib.rs (DELETE /data/v1/projects/{project_id}/items/{item_id})
- [ ] T006 [P] Add `rename_item()` method in raps-dm/src/lib.rs (PATCH /data/v1/projects/{project_id}/items/{item_id})
- [ ] T007 [P] Add `create_project()` method in raps-acc/src/lib.rs (POST /construction/admin/v1/accounts/{accountId}/projects)
- [ ] T008 [P] Add `wait_for_project_activation()` method in raps-acc/src/lib.rs (polls project until status=active)
- [ ] T009 [P] Add `import_users()` method in raps-acc/src/users.rs (POST /projects/{projectId}/users:import)

### Data Types

- [ ] T010 [P] Add `ObjectDetails` struct in raps-oss/src/lib.rs per data-model.md
- [ ] T011 [P] Add `ProjectCreationJob` and `ProjectCreationStatus` in raps-acc/src/lib.rs
- [ ] T012 [P] Add `ImportUsersResult` and `ImportUserError` in raps-acc/src/users.rs

### Tests for Foundational Methods

- [ ] T013 [P] Add unit test for `get_object_details()` in raps-oss/src/lib.rs (use raps-mock)
- [ ] T014 [P] Add unit test for `delete_item()` in raps-dm/src/lib.rs
- [ ] T015 [P] Add unit test for `rename_item()` in raps-dm/src/lib.rs
- [ ] T016 [P] Add unit test for `create_project()` in raps-acc/src/lib.rs
- [ ] T017 [P] Add unit test for `import_users()` in raps-acc/src/users.rs

**Checkpoint**: Foundation ready - MCP tool implementation can now begin

---

## Phase 3: User Story 7 - Single File Upload (Priority: P1) 🎯 MVP

**Goal**: AI assistant can upload a single file to an OSS bucket via MCP

**Independent Test**: Ask AI to "upload model.rvt to my-bucket" and verify object exists

### Implementation for US7

- [ ] T018 [US7] Add `object_upload()` async method in raps-cli/src/mcp/server.rs (wrap OssClient::upload_object_with_options)
- [ ] T019 [US7] Add dispatch case for "object_upload" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T020 [US7] Add Tool schema for object_upload in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T021 [US7] Add "object_upload" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T022 [US7] Add auth mapping for object_upload (TwoLegged) in raps-cli/src/mcp/auth_guidance.rs

### Test for US7

- [ ] T023 [US7] Add integration test for object_upload tool in raps-cli/tests/mcp_tools_test.rs

**Checkpoint**: object_upload tool functional and testable

---

## Phase 4: User Story 2 - Batch Object Operations (Priority: P1) 🎯 MVP

**Goal**: AI assistant can upload multiple files with 4-way concurrency

**Independent Test**: Ask AI to "upload these 5 files to my bucket" and verify all uploaded with summary

### Implementation for US2

- [ ] T024 [US2] Add `object_upload_batch()` async method in raps-cli/src/mcp/server.rs (4-way semaphore loop)
- [ ] T025 [US2] Add dispatch case for "object_upload_batch" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T026 [US2] Add Tool schema for object_upload_batch in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T027 [US2] Add "object_upload_batch" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T028 [US2] Add auth mapping for object_upload_batch (TwoLegged) in raps-cli/src/mcp/auth_guidance.rs

### Test for US2

- [ ] T029 [US2] Add integration test for object_upload_batch tool in raps-cli/tests/mcp_tools_test.rs

**Checkpoint**: object_upload_batch tool functional with parallel uploads

---

## Phase 5: User Story 1 - Project Info (Priority: P1) 🎯 MVP

**Goal**: AI assistant can retrieve complete project information in a single call

**Independent Test**: Ask AI to "show me details about project X" and receive project info with folders

### Implementation for US1

- [ ] T030 [US1] Add `project_info()` async method in raps-cli/src/mcp/server.rs (combine get_project + get_top_folders)
- [ ] T031 [US1] Add dispatch case for "project_info" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T032 [US1] Add Tool schema for project_info in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T033 [US1] Add "project_info" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T034 [US1] Add auth mapping for project_info (ThreeLegged) in raps-cli/src/mcp/auth_guidance.rs

### Test for US1

- [ ] T035 [US1] Add integration test for project_info tool in raps-cli/tests/mcp_tools_test.rs

**Checkpoint**: P1 MVP Complete - All P1 stories functional

---

## Phase 6: User Story 4 - Object Metadata (Priority: P2)

**Goal**: AI assistant can retrieve detailed object metadata without downloading

**Independent Test**: Ask AI to "show me details about model.rvt in my bucket" and receive metadata

### Implementation for US4

- [ ] T036 [US4] Add `object_info()` async method in raps-cli/src/mcp/server.rs (wrap get_object_details)
- [ ] T037 [US4] Add dispatch case for "object_info" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T038 [US4] Add Tool schema for object_info in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T039 [US4] Add "object_info" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T040 [US4] Add auth mapping for object_info (TwoLegged) in raps-cli/src/mcp/auth_guidance.rs

### Test for US4

- [ ] T041 [US4] Add integration test for object_info tool in raps-cli/tests/mcp_tools_test.rs

**Checkpoint**: object_info tool functional

---

## Phase 7: User Story 8 - Download Object (Priority: P2)

**Goal**: AI assistant can download objects to local file paths

**Independent Test**: Ask AI to "download model.rvt to /tmp/model.rvt" and verify file exists

### Implementation for US8

- [ ] T042 [US8] Add `object_download()` async method in raps-cli/src/mcp/server.rs (wrap OssClient::download_object)
- [ ] T043 [US8] Add dispatch case for "object_download" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T044 [US8] Add Tool schema for object_download in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T045 [US8] Add "object_download" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T046 [US8] Add auth mapping for object_download (TwoLegged) in raps-cli/src/mcp/auth_guidance.rs

### Test for US8

- [ ] T047 [US8] Add integration test for object_download tool in raps-cli/tests/mcp_tools_test.rs

**Checkpoint**: object_download tool functional

---

## Phase 8: User Story 3 - Copy Objects (Priority: P2)

**Goal**: AI assistant can copy objects between buckets (download + upload pattern)

**Independent Test**: Ask AI to "copy model.rvt from bucket-a to bucket-b" and verify in both buckets

### Implementation for US3

- [ ] T048 [US3] Add `object_copy()` async method in raps-cli/src/mcp/server.rs (download to temp, upload, cleanup)
- [ ] T049 [US3] Add skip-with-warning logic for existing destination objects in object_copy()
- [ ] T050 [US3] Add dispatch case for "object_copy" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T051 [US3] Add Tool schema for object_copy in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T052 [US3] Add "object_copy" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T053 [US3] Add auth mapping for object_copy (TwoLegged) in raps-cli/src/mcp/auth_guidance.rs

### Test for US3

- [ ] T054 [US3] Add integration test for object_copy tool in raps-cli/tests/mcp_tools_test.rs

**Checkpoint**: object_copy tool functional with skip-on-exists behavior

---

## Phase 9: User Story 9 - Bulk Delete (Priority: P2)

**Goal**: AI assistant can delete multiple objects efficiently

**Independent Test**: Ask AI to "delete all .tmp files from my-bucket" and verify removed

### Implementation for US9

- [ ] T055 [US9] Add `object_delete_batch()` async method in raps-cli/src/mcp/server.rs (loop with semaphore, summary)
- [ ] T056 [US9] Add dispatch case for "object_delete_batch" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T057 [US9] Add Tool schema for object_delete_batch in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T058 [US9] Add "object_delete_batch" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T059 [US9] Add auth mapping for object_delete_batch (TwoLegged) in raps-cli/src/mcp/auth_guidance.rs

### Test for US9

- [ ] T060 [US9] Add integration test for object_delete_batch tool in raps-cli/tests/mcp_tools_test.rs

**Checkpoint**: object_delete_batch tool functional

---

## Phase 10: User Story 5 - Folder Contents (Priority: P2)

**Goal**: AI assistant can list folder contents with pagination

**Independent Test**: Ask AI to "list all items in the Plans folder" and receive paginated results

### Implementation for US5

- [ ] T061 [US5] Add `folder_contents()` async method in raps-cli/src/mcp/server.rs (wrap list_folder_contents with pagination)
- [ ] T062 [US5] Add dispatch case for "folder_contents" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T063 [US5] Add Tool schema for folder_contents in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T064 [US5] Add "folder_contents" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T065 [US5] Add auth mapping for folder_contents (ThreeLegged) in raps-cli/src/mcp/auth_guidance.rs

### Test for US5

- [ ] T066 [US5] Add integration test for folder_contents tool in raps-cli/tests/mcp_tools_test.rs

**Checkpoint**: folder_contents tool functional with pagination

---

## Phase 11: User Story 10 - Create ACC Project (Priority: P2)

**Goal**: AI assistant can create new ACC projects from scratch or template

**Independent Test**: Ask AI to "create a new project called Test Project" and verify created

### Implementation for US10

- [ ] T067 [US10] Add `project_create()` async method in raps-cli/src/mcp/server.rs (calls create_project + wait_for_activation)
- [ ] T068 [US10] Add template member warning in project_create response format
- [ ] T069 [US10] Add dispatch case for "project_create" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T070 [US10] Add Tool schema for project_create in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T071 [US10] Add "project_create" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T072 [US10] Add auth mapping for project_create (ThreeLegged) in raps-cli/src/mcp/auth_guidance.rs

### Test for US10

- [ ] T073 [US10] Add integration test for project_create tool in raps-cli/tests/mcp_tools_test.rs

**Checkpoint**: project_create tool functional with polling

---

## Phase 12: User Story 11 - Add Users to Project (Priority: P2)

**Goal**: AI assistant can add users to ACC projects with roles

**Independent Test**: Ask AI to "add user@example.com as project admin" and verify access

### Implementation for US11

- [ ] T074 [US11] Add `project_user_add()` async method in raps-cli/src/mcp/server.rs (wrap ProjectUsersClient::add_user)
- [ ] T075 [US11] Add dispatch case for "project_user_add" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T076 [US11] Add Tool schema for project_user_add in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T077 [US11] Add "project_user_add" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T078 [US11] Add auth mapping for project_user_add (ThreeLegged) in raps-cli/src/mcp/auth_guidance.rs

### Additional Project Management

- [ ] T079 [US11] Add `project_users_list()` async method in raps-cli/src/mcp/server.rs (wrap list_project_users)
- [ ] T080 [US11] Add dispatch case for "project_users_list" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T081 [US11] Add Tool schema for project_users_list in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T082 [US11] Add "project_users_list" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T083 [US11] Add auth mapping for project_users_list (ThreeLegged) in raps-cli/src/mcp/auth_guidance.rs

### Tests for US11

- [ ] T084 [US11] Add integration test for project_user_add tool in raps-cli/tests/mcp_tools_test.rs
- [ ] T085 [US11] Add integration test for project_users_list tool in raps-cli/tests/mcp_tools_test.rs

**Checkpoint**: P2 user management tools functional

---

## Phase 13: User Story 6 - Item Management (Priority: P3)

**Goal**: AI assistant can create, delete, and rename items in project folders

**Independent Test**: Ask AI to "add model.rvt to my project's Models folder" and verify item appears

### Implementation for US6

- [ ] T086 [US6] Add `item_create()` async method in raps-cli/src/mcp/server.rs (wrap create_item_from_storage)
- [ ] T087 [US6] Add dispatch case for "item_create" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T088 [US6] Add Tool schema for item_create in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T089 [US6] Add "item_create" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T090 [US6] Add auth mapping for item_create (ThreeLegged) in raps-cli/src/mcp/auth_guidance.rs

- [ ] T091 [US6] Add `item_delete()` async method in raps-cli/src/mcp/server.rs (wrap delete_item)
- [ ] T092 [US6] Add dispatch case for "item_delete" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T093 [US6] Add Tool schema for item_delete in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T094 [US6] Add "item_delete" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T095 [US6] Add auth mapping for item_delete (ThreeLegged) in raps-cli/src/mcp/auth_guidance.rs

- [ ] T096 [US6] Add `item_rename()` async method in raps-cli/src/mcp/server.rs (wrap rename_item)
- [ ] T097 [US6] Add dispatch case for "item_rename" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T098 [US6] Add Tool schema for item_rename in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T099 [US6] Add "item_rename" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T100 [US6] Add auth mapping for item_rename (ThreeLegged) in raps-cli/src/mcp/auth_guidance.rs

### Tests for US6

- [ ] T101 [US6] Add integration test for item_create tool in raps-cli/tests/mcp_tools_test.rs
- [ ] T102 [US6] Add integration test for item_delete tool in raps-cli/tests/mcp_tools_test.rs
- [ ] T103 [US6] Add integration test for item_rename tool in raps-cli/tests/mcp_tools_test.rs

**Checkpoint**: item management tools functional

---

## Phase 14: User Story 12 - Bulk Import Users (Priority: P3)

**Goal**: AI assistant can bulk import multiple users to a project

**Independent Test**: Ask AI to "add these 5 users to Project X" and verify all have access

### Implementation for US12

- [ ] T104 [US12] Add `project_users_import()` async method in raps-cli/src/mcp/server.rs (wrap import_users)
- [ ] T105 [US12] Add dispatch case for "project_users_import" in dispatch_tool() in raps-cli/src/mcp/server.rs
- [ ] T106 [US12] Add Tool schema for project_users_import in get_tools() in raps-cli/src/mcp/server.rs
- [ ] T107 [US12] Add "project_users_import" to TOOL_LIST in raps-cli/src/mcp/tools.rs
- [ ] T108 [US12] Add auth mapping for project_users_import (ThreeLegged) in raps-cli/src/mcp/auth_guidance.rs

### Test for US12

- [ ] T109 [US12] Add integration test for project_users_import tool in raps-cli/tests/mcp_tools_test.rs

**Checkpoint**: All P3 user stories complete

---

## Phase 15: Polish & Cross-Cutting Concerns

**Purpose**: Final validation, documentation, and quality assurance

- [ ] T110 [P] Update MCP server instructions in raps-cli/src/mcp/server.rs (add new tools to intro text)
- [ ] T111 [P] Update docs/commands/mcp.md with all 15 new tools and examples
- [ ] T112 Run `cargo fmt --all` to format all code
- [ ] T113 Run `cargo clippy --all-features -- -D warnings` and fix any warnings
- [ ] T114 Run `cargo test --workspace` to verify all tests pass
- [ ] T115 Validate quickstart.md scenarios work with MCP server
- [ ] T116 Update raps-cli/Cargo.toml if any new dependencies were added

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup - BLOCKS all user stories
- **User Stories (Phases 3-14)**: All depend on Foundational phase completion
  - Within P1: US7 → US2 → US1 (can run in parallel but suggested order)
  - P2 stories can run in parallel after P1
  - P3 stories can run in parallel after P2
- **Polish (Phase 15)**: Depends on all user stories being complete

### User Story Dependencies

| Story | Priority | Can Start After | Notes |
|-------|----------|-----------------|-------|
| US7 (Single Upload) | P1 | Phase 2 | No dependencies |
| US2 (Batch Upload) | P1 | Phase 2 | Uses US7 pattern |
| US1 (Project Info) | P1 | Phase 2 | No dependencies |
| US4 (Object Info) | P2 | T004 (get_object_details) | Needs foundational method |
| US8 (Download) | P2 | Phase 2 | Uses existing OssClient method |
| US3 (Copy) | P2 | US8 complete | Uses download internally |
| US9 (Bulk Delete) | P2 | Phase 2 | No dependencies |
| US5 (Folder Contents) | P2 | Phase 2 | Uses existing DmClient method |
| US10 (Create Project) | P2 | T007, T008 (create_project) | Needs foundational methods |
| US11 (Add Users) | P2 | Phase 2 | Uses existing add_user |
| US6 (Item Management) | P3 | T005, T006 (delete/rename_item) | Needs foundational methods |
| US12 (Bulk Import) | P3 | T009 (import_users) | Needs foundational method |

### Parallel Opportunities

**Within Phase 2 (Foundational)**:
- T004-T012 (all crate methods and types) can run in parallel
- T013-T017 (foundational tests) can run in parallel after methods

**After Phase 2 (User Stories)**:
- All P1 stories (US7, US2, US1) can run in parallel
- All P2 stories can run in parallel after P1
- All P3 stories can run in parallel after P2

---

## Parallel Example: P1 MVP Stories

```bash
# After Phase 2, launch all P1 implementation in parallel:
Task: "Add object_upload() async method in raps-cli/src/mcp/server.rs"
Task: "Add object_upload_batch() async method in raps-cli/src/mcp/server.rs"
Task: "Add project_info() async method in raps-cli/src/mcp/server.rs"
```

---

## Implementation Strategy

### MVP First (P1 Only)

1. Complete Phase 1: Setup (verify branch, run checks)
2. Complete Phase 2: Foundational (6 new crate methods + types)
3. Complete Phases 3-5: P1 User Stories (object_upload, object_upload_batch, project_info)
4. **STOP and VALIDATE**: Test all 3 P1 tools work independently
5. Deploy/demo MVP with 3 core tools

### Incremental Delivery

1. P1 MVP → 3 tools: object_upload, object_upload_batch, project_info
2. Add P2 Object Tools → +6 tools: object_info, object_download, object_copy, object_delete_batch, folder_contents
3. Add P2 Admin Tools → +3 tools: project_create, project_user_add, project_users_list
4. Add P3 Tools → +3 tools: item_create, item_delete, item_rename, project_users_import
5. Final: 15 total new MCP tools

### Parallel Team Strategy

With 3 developers after Foundational phase:
- Developer A: All Object tools (US7, US2, US4, US8, US3, US9) - 6 tools
- Developer B: All Project tools (US1, US5, US10, US11, US12) - 5 tools (+ project_users_list)
- Developer C: All Item tools (US6) - 3 tools + Polish phase

---

## Summary

| Metric | Count |
|--------|-------|
| **Total Tasks** | 116 |
| **Phase 1 (Setup)** | 3 |
| **Phase 2 (Foundational)** | 14 |
| **P1 User Stories** | 18 (3 stories) |
| **P2 User Stories** | 63 (8 stories) |
| **P3 User Stories** | 24 (2 stories) |
| **Polish** | 7 |
| **New MCP Tools** | 15 |
| **New Crate Methods** | 6 |

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- All tests use raps-mock for API mocking per existing patterns

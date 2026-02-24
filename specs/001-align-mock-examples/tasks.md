# Tasks: Align Mock Server and Examples with v4.13.0

**Input**: Design documents from `/specs/001-align-mock-examples/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/
**Branch**: `001-align-mock-examples`
**Date**: 2026-02-24

**Tests**: Integration tests included per Constitution Principle V.

**Organization**: Tasks grouped by user story. US1 and US2 are P1 (mock handlers). US3 and US4 are P2 (example tests depending on mock handlers). US5 and US6 are P3 (DA upload mock + examples).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths in descriptions

---

## Phase 1: Setup

**Purpose**: Verify repositories are buildable and understand current state

- [X] T001 Run `cargo check` in raps-mock/ to verify existing codebase compiles cleanly
- [X] T002 Run `cargo test` in raps-mock/ to verify existing tests pass (baseline)
- [X] T003 [P] Run `pytest --mock --mock-port 3000 -x` in raps-examples/ to verify existing example tests pass against mock

**Checkpoint**: Both repositories compile and test clean — new implementation can begin

---

## Phase 2: Foundational (Schema Extension)

**Purpose**: Extend the mock database schema to support metadata storage — MUST complete before US1 handlers

- [X] T004 Add `metadata_json TEXT`, `object_tree_json TEXT`, `properties_json TEXT` columns to translations table schema in raps-mock/src/state/db.rs
- [X] T005 Add `generate_mock_metadata()` function to TranslationState that creates synthetic metadata/tree/properties JSON when a translation reaches Success status in raps-mock/src/state/translations.rs
- [X] T006 Call `generate_mock_metadata()` from `simulate_progress()` and `update_job_status()` when status becomes Success in raps-mock/src/state/translations.rs

**Checkpoint**: Schema extended, mock metadata auto-generated on translation success

---

## Phase 3: User Story 1 — Mock Model Derivative Metadata Endpoints (Priority: P1) 🎯 MVP

**Goal**: Add 4 mock handlers for metadata, object tree, properties, and query-properties endpoints

**Independent Test**: Start mock server, create a translation, GET metadata/tree/properties — all return valid JSON responses.

### Implementation for User Story 1

- [X] T007 [US1] Add `get_metadata(urn) -> Option<Value>` method to TranslationState that reads metadata_json from DB in raps-mock/src/state/translations.rs
- [X] T008 [P] [US1] Add `get_object_tree(urn, guid) -> Option<Value>` method to TranslationState in raps-mock/src/state/translations.rs
- [X] T009 [P] [US1] Add `get_properties(urn, guid) -> Option<Value>` method to TranslationState in raps-mock/src/state/translations.rs
- [X] T010 [US1] Add `handle_get_metadata(state, urn)` handler function in raps-mock/src/handlers/routes.rs returning MetadataResponse JSON per contracts/mock-metadata-endpoints.md
- [X] T011 [P] [US1] Add `handle_get_object_tree(state, urn, guid)` handler function in raps-mock/src/handlers/routes.rs
- [X] T012 [P] [US1] Add `handle_get_properties(state, urn, guid)` handler function in raps-mock/src/handlers/routes.rs
- [X] T013 [US1] Add `handle_query_properties(state, urn, guid, body)` handler function that filters properties by object IDs from POST body in raps-mock/src/handlers/routes.rs
- [X] T014 [US1] Register 4 metadata routes in `register_hardcoded_routes()` in raps-mock/src/server/router.rs: GET /metadata, GET /metadata/{guid}, GET /metadata/{guid}/properties, POST /metadata/{guid}/properties:query
- [X] T015 [US1] Add integration test for metadata endpoints in raps-mock/tests/metadata_test.rs: create translation, verify metadata/tree/properties responses

**Checkpoint**: All 4 metadata endpoints return valid mock responses

---

## Phase 4: User Story 2 — Mock OSS Server-Side Copy (Priority: P1)

**Goal**: Add mock handler for object copy via x-ads-copy-from header

**Independent Test**: Upload object to mock bucket, PUT with copy-from header — object appears at destination.

### Implementation for User Story 2

- [X] T016 [US2] Add `copy_object(src_bucket, src_key, dest_bucket, dest_key) -> Option<ObjectInfo>` method to ObjectState in raps-mock/src/state/objects.rs
- [X] T017 [US2] Add `handle_copy_object(state, dest_bucket, dest_key, headers)` handler function that reads x-ads-copy-from header and calls copy_object in raps-mock/src/handlers/routes.rs
- [X] T018 [US2] Register copy route or extend existing PUT /buckets/{bucket}/objects/{key} handler to check for x-ads-copy-from header in raps-mock/src/server/router.rs
- [X] T019 [US2] Add integration test for copy endpoint in raps-mock/tests/copy_test.rs: upload object, copy it, verify destination exists with same content

**Checkpoint**: OSS server-side copy works in mock — batch operations can be tested

---

## Phase 5: User Story 3 — Example Tests for Model Derivative Commands (Priority: P2)

**Goal**: Add 4 new test functions for translate metadata/tree/properties/query-properties

**Independent Test**: Run `pytest --mock tests/test_05_model_derivative.py -k "metadata or tree or properties"` — all pass.

### Implementation for User Story 3

- [X] T020 [US3] Add `test_sr550_translate_metadata_lifecycle` test function to raps-examples/tests/test_05_model_derivative.py: lifecycle covering metadata, tree, properties, query-properties
- [X] T021 [P] [US3] (covered by SR-550 lifecycle) `raps translate tree <URN> <GUID> --output json`
- [X] T022 [P] [US3] (covered by SR-550 lifecycle) `raps translate properties <URN> <GUID> --output json`
- [X] T023 [US3] (covered by SR-550 lifecycle) `raps translate query-properties <URN> <GUID> --filter "1,2,3" --output json`

**Checkpoint**: All 4 translate metadata example tests pass against mock

---

## Phase 6: User Story 4 — Example Tests for OSS Batch Commands (Priority: P2)

**Goal**: Add 3 new test functions for object copy/batch-copy/batch-rename

**Independent Test**: Run `pytest --mock tests/test_03_storage.py -k "copy or batch"` — all pass.

### Implementation for User Story 4

- [X] T024 [US4] Already covered by existing SR-060 `test_sr060_object_copy` in raps-examples/tests/test_03_storage.py
- [X] T025 [P] [US4] Add `test_sr551_object_batch_copy` test function to raps-examples/tests/test_03_storage.py: `raps object batch-copy <src> <dest>`
- [X] T026 [P] [US4] Add `test_sr552_object_batch_rename` test function to raps-examples/tests/test_03_storage.py: `raps object batch-rename <bucket> --from ... --to ...`

**Checkpoint**: All 3 OSS batch example tests pass against mock

---

## Phase 7: User Story 5 — Mock DA Appbundle Upload (Priority: P3)

**Goal**: Add mock upload endpoint and upload parameters in bundle creation response

**Independent Test**: Create appbundle via mock, POST multipart form to mock upload URL — returns 200 OK.

### Implementation for User Story 5

- [X] T027 [US5] Add `upload_url` and `upload_form_data` fields to AppBundleInfo struct in raps-mock/src/state/da.rs
- [X] T028 [US5] Modify `create_app_bundle()` to populate upload parameters with mock endpoint URL and form data in raps-mock/src/state/da.rs
- [X] T029 [US5] Update `handle_create_app_bundle()` handler to include uploadParameters in response JSON in raps-mock/src/handlers/routes.rs
- [X] T030 [US5] Add `handle_mock_s3_upload(bundle_id, multipart)` handler that accepts multipart form POST and returns 200 OK in raps-mock/src/handlers/routes.rs
- [X] T031 [US5] Register `/mock-s3-upload/{bundle_id}` POST route in raps-mock/src/server/router.rs
- [X] T032 [US5] Add integration test for upload endpoint in raps-mock/tests/upload_test.rs: create bundle, POST multipart form, verify 200 response

**Checkpoint**: DA appbundle upload mock endpoint works end-to-end

---

## Phase 8: User Story 6 — Example Tests for DA Appbundle Upload (Priority: P3)

**Goal**: Add test function for da appbundle-upload command

**Independent Test**: Run `pytest --mock tests/test_06_design_automation.py -k "appbundle_upload"` — passes.

### Implementation for User Story 6

- [X] T033 [US6] Add `test_sr553_appbundle_upload_lifecycle` test function to raps-examples/tests/test_06_design_automation.py: `raps da appbundle-upload <ID> --file <test.zip> --engine Autodesk.Revit+2024`
- [X] T034 [US6] Create minimal test zip file fixture in raps-examples/test-data/sample-bundle.zip for upload testing

**Checkpoint**: DA appbundle upload example test passes against mock

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Final validation across all changes

- [X] T035 Run `cargo check` in raps-mock/ to verify full project compiles
- [X] T036 Run `cargo test` in raps-mock/ to verify zero test regressions (9 tests pass)
- [X] T037 Run `cargo clippy -- -D warnings` in raps-mock/ for lint compliance (fixed 5 warnings)
- [X] T038 Run `cargo fmt -- --check` in raps-mock/ for formatting compliance
- [ ] T039 Run `pytest --mock` in raps-examples/ to verify all example tests pass (old + new) — requires mock server running + raps binary built
- [X] T040 Validate quickstart.md scenarios match implemented mock endpoints and CLI commands (fixed appbundle-create prerequisite + command name)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — verify both repos immediately
- **Foundational (Phase 2)**: Depends on Setup — schema extension must come first
- **US1 (Phase 3)**: Depends on Foundational — metadata handlers need schema columns
- **US2 (Phase 4)**: Depends on Setup — copy handler is independent of metadata schema
- **US3 (Phase 5)**: Depends on US1 — example tests need mock metadata endpoints working
- **US4 (Phase 6)**: Depends on US2 — example tests need mock copy endpoint working
- **US5 (Phase 7)**: Depends on Setup — DA upload is independent of metadata/copy
- **US6 (Phase 8)**: Depends on US5 — example test needs mock upload endpoint working
- **Polish (Phase 9)**: Depends on ALL user stories being complete

### User Story Dependencies

- **US1 (P1)**: Depends on Foundational phase (schema) — independent of other stories
- **US2 (P1)**: Depends on Setup only — independent of US1 (different files)
- **US3 (P2)**: Depends on US1 completion — needs mock metadata endpoints
- **US4 (P2)**: Depends on US2 completion — needs mock copy endpoint
- **US5 (P3)**: Depends on Setup only — independent of US1/US2 (different files)
- **US6 (P3)**: Depends on US5 completion — needs mock upload endpoint

### Within Each User Story

- State methods before handler functions
- Handler functions before route registration
- Route registration before integration tests

### Parallel Opportunities

- US1 (Foundational + metadata) and US2 (copy) can run in parallel after Setup
- US5 (DA upload) can run in parallel with US1/US2 after Setup
- Within US1: T008 and T009 are parallel (independent state methods); T011 and T012 are parallel (independent handlers)
- Within US4: T025 and T026 are parallel (independent example tests)
- US3 can start as soon as US1 completes; US4 can start as soon as US2 completes

---

## Parallel Example: User Stories 1 + 2 + 5

```bash
# After Setup (Phase 1), launch three independent work streams:

# Stream A: Foundational + US1 (metadata mock)
Task: "Extend translations schema in db.rs"              # T004
Task: "Generate mock metadata on success"                 # T005-T006
Task: "Add metadata state methods + handlers + routes"    # T007-T014
Task: "Metadata integration test"                         # T015

# Stream B: US2 (copy mock) — no Foundational dependency
Task: "Add copy_object method to ObjectState"             # T016
Task: "Add copy handler + route registration"             # T017-T018
Task: "Copy integration test"                             # T019

# Stream C: US5 (DA upload mock) — no Foundational dependency
Task: "Add upload params to AppBundleInfo"                # T027-T028
Task: "Add upload handler + route"                        # T029-T031
Task: "Upload integration test"                           # T032
```

## Parallel Example: Example Tests (after mock handlers ready)

```bash
# After US1 + US2 complete, launch example test streams:

# Stream D: US3 (metadata examples) — depends on US1
Task: "Add 4 translate metadata test functions"           # T020-T023

# Stream E: US4 (batch examples) — depends on US2
Task: "Add 3 object batch test functions"                 # T024-T026

# Stream F: US6 (DA upload example) — depends on US5
Task: "Add appbundle upload test + fixture"               # T033-T034
```

---

## Implementation Strategy

### MVP First (US1 + US2: Mock Handlers)

1. Complete Phase 1: Setup (verify both repos)
2. Complete Phase 2: Foundational (schema extension)
3. Complete Phase 3: US1 — Metadata mock handlers
4. Complete Phase 4: US2 — Copy mock handler
5. **STOP and VALIDATE**: `cargo test` in raps-mock, all new handlers respond correctly
6. Mock server can now serve all v4.13.0 endpoints

### Incremental Delivery

1. Setup → Verify repos → Foundation ready
2. US1 + US2 → Mock handlers complete → Core mock capability (MVP)
3. US3 + US4 → Example tests for metadata + batch → CI coverage
4. US5 + US6 → DA upload mock + example → Specialized workflow complete
5. Polish → Full validation → Release ready

### Task Summary

| Phase | Story | Tasks | New Code | Verification Only |
|-------|-------|-------|----------|-------------------|
| 1 | Setup | 3 | 0 | 3 |
| 2 | Foundational | 3 | 3 | 0 |
| 3 | US1 | 9 | 9 | 0 |
| 4 | US2 | 4 | 4 | 0 |
| 5 | US3 | 4 | 4 | 0 |
| 6 | US4 | 3 | 3 | 0 |
| 7 | US5 | 6 | 6 | 0 |
| 8 | US6 | 2 | 2 | 0 |
| 9 | Polish | 6 | 0 | 6 |
| **Total** | | **40** | **31** | **9** |

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- raps-mock changes are in `C:\github\raps\raps-mock\`
- raps-examples changes are in `C:\github\raps\raps-examples\`
- Mock metadata is auto-generated (synthetic data) — no need for real APS responses
- Example tests use `raps.run()` pattern with SR-200+ IDs to avoid conflicts
- Commit after each completed user story phase
- Stop at any checkpoint to validate story independently

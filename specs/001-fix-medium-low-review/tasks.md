# Tasks: Fix MEDIUM and LOW Severity Review Findings

**Input**: Design documents from `/specs/001-fix-medium-low-review/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/
**Branch**: `001-fix-medium-low-review`
**Date**: 2026-02-24

**Tests**: Unit tests included per Constitution Principle V.

**Organization**: Tasks grouped by user story. US4, US6, US7 have prior work carried forward from previous session (uncommitted changes in working tree). US1 is partially complete (reality.rs timeout done, translate.rs timeout remaining).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Verify Prior Work)

**Purpose**: Verify the 6 prior fixes carried forward as uncommitted changes are valid and compilable

- [X] T001 Run `cargo check --workspace` to verify prior work compiles cleanly
- [X] T002 Run `cargo test -p raps-reality -p raps-webhooks -p raps-dm` to verify prior fix tests pass

**Checkpoint**: Prior work validated — new implementation can begin

---

## Phase 2: Foundational (Shared Types)

**Purpose**: No blocking shared infrastructure needed. All new types are scoped to individual crates.

**⚠️ NOTE**: Skipped — no cross-crate foundational work required. Each user story adds types to its own crate.

---

## Phase 3: User Story 1 — Safe Polling with Timeouts (Priority: P1)

**Goal**: Add 2-hour client-side timeout to the translate.rs polling loop (reality.rs already has 4-hour timeout from prior work)

**Independent Test**: Start a translation with `--wait` and verify the poll loop terminates after timeout with a clear message and URN for manual status check.

### Implementation for User Story 1

- [X] T003 [US1] Add 2-hour client-side timeout to translation polling loop in raps-cli/src/commands/translate.rs
- [X] T004 [US1] Add test verifying timeout constant is defined and matches 2-hour duration in raps-cli/src/commands/translate.rs

**Checkpoint**: All polling loops (translate, reality, DA) now have timeout protection

---

## Phase 4: User Story 2 — Model Derivative Metadata and Properties (Priority: P1) 🎯 MVP

**Goal**: Implement 4 client methods and 4 CLI subcommands for retrieving model metadata, object trees, and properties from translated models

**Independent Test**: After translating a model, run `raps translate metadata <URN>`, `raps translate tree <URN> <GUID>`, `raps translate properties <URN> <GUID>`, and `raps translate query-properties <URN> <GUID> --filter "1,2,3"` — all return structured output.

### Implementation for User Story 2

- [X] T005 [US2] Add response types (ModelViews, ModelView, ObjectTree, PropertiesResult) with Serialize/Deserialize derives in raps-derivative/src/lib.rs
- [X] T006 [US2] Add request types (PropertyQuery, PropertyQueryFilter, PropertyPagination) in raps-derivative/src/lib.rs
- [X] T007 [US2] Implement get_metadata(urn) client method with region support in raps-derivative/src/lib.rs
- [X] T008 [P] [US2] Implement get_object_tree(urn, model_guid) client method in raps-derivative/src/lib.rs
- [X] T009 [P] [US2] Implement get_properties(urn, model_guid) client method in raps-derivative/src/lib.rs
- [X] T010 [US2] Implement query_properties(urn, model_guid, query) client method with POST body in raps-derivative/src/lib.rs
- [X] T011 [US2] Add `metadata`, `tree`, `properties`, `query-properties` subcommands to TranslateCommands enum in raps-cli/src/commands/translate.rs
- [X] T012 [US2] Implement CLI handlers for metadata/tree/properties/query-properties with --output and --region support in raps-cli/src/commands/translate.rs
- [X] T013 [US2] Add unit tests for metadata response deserialization and region header in raps-derivative/src/lib.rs

**Checkpoint**: Users can retrieve model metadata, object trees, and properties through the CLI

---

## Phase 5: User Story 3 — OSS Batch Operations (Priority: P2)

**Goal**: Implement single-object copy base method plus batch copy and batch rename operations with concurrent execution and per-item result reporting

**Independent Test**: Run `raps oss batch-copy src-bucket dest-bucket` and `raps oss batch-rename bucket --from "old/" --to "new/"` — both report per-object results.

### Implementation for User Story 3

- [X] T014 [US3] Add BatchResult<T> and BatchItemResult<T> types in raps-oss/src/lib.rs
- [X] T015 [US3] Implement copy_object(src_bucket, object_key, dest_bucket, dest_key) single-object method in raps-oss/src/lib.rs
- [X] T016 [US3] Implement batch_copy_objects() with tokio::JoinSet + Semaphore(10) in raps-oss/src/lib.rs
- [X] T017 [US3] Implement batch_rename_object() with copy-then-delete pattern in raps-oss/src/lib.rs
- [X] T018 [US3] Add `copy`, `batch-copy`, `batch-rename` subcommands to OSS CLI in raps-cli/src/commands/object.rs
- [X] T019 [US3] Implement CLI handlers for copy/batch-copy/batch-rename with progress reporting in raps-cli/src/commands/object.rs
- [X] T020 [US3] Add unit tests for BatchResult types and batch operation result collection in raps-oss/src/lib.rs

**Checkpoint**: Users can copy and rename objects in bulk with concurrent execution

---

## Phase 6: User Story 4 — BIM360 Folder Support (Priority: P2)

**Goal**: Verify BIM360 folder extension auto-detection (carried forward from prior work)

**Independent Test**: Create a folder in a BIM360 project (project ID with `b.` prefix) and verify correct extension type is used.

### Verification for User Story 4

- [X] T021 [US4] Verify BIM360 folder extension detection logic is correct in raps-dm/src/lib.rs (prior work — no new code needed)

**Checkpoint**: Folder creation works for both BIM360 and ACC projects

---

## Phase 7: User Story 5 — Parallel User Imports (Priority: P2)

**Goal**: Refactor sequential user import to use concurrent requests bounded by a semaphore for >50% speedup on 20+ user batches

**Independent Test**: Import 20+ users and verify total time is significantly less than sequential, with individual success/failure reporting.

### Implementation for User Story 5

- [X] T022 [US5] Refactor import_users() to use tokio::JoinSet with Semaphore(10) for concurrent execution in raps-acc/src/users.rs
- [X] T023 [US5] Update import CLI handler to show concurrent progress indicator in raps-cli/src/commands/admin.rs
- [X] T024 [US5] Add unit test verifying import_users returns correct aggregate results in raps-acc/src/users.rs

**Checkpoint**: User imports run concurrently with rate-limit-safe concurrency

---

## Phase 8: User Story 6 — Webhook Validation and Clarity (Priority: P3)

**Goal**: Verify webhook event validation and auth documentation (carried forward from prior work)

**Independent Test**: Run `raps webhook create --event invalid.event --url ...` and verify immediate rejection with valid events list.

### Verification for User Story 6

- [X] T025 [US6] Verify is_valid_event() and CLI validation logic in raps-webhooks/src/lib.rs and raps-cli/src/commands/webhook.rs (prior work — no new code needed)

**Checkpoint**: Invalid webhook events are rejected before API submission

---

## Phase 9: User Story 7 — Reality Capture Data Quality (Priority: P3)

**Goal**: Verify human-readable file size display and filesize_bytes() helpers (carried forward from prior work)

**Independent Test**: View a photoscene result and verify file size displays as "52.43 MB" instead of raw "54935241".

### Verification for User Story 7

- [X] T026 [US7] Verify filesize_bytes() helpers and human-readable display in raps-reality/src/lib.rs and raps-cli/src/commands/reality.rs (prior work — no new code needed)

**Checkpoint**: File sizes display in human-readable format

---

## Phase 10: User Story 8 — DA App Bundle Upload (Priority: P3)

**Goal**: Implement app bundle archive upload using pre-signed S3 URL with multipart form data

**Independent Test**: Create an app bundle, then upload a .zip archive via `raps da upload-appbundle <ID> <FILE>` and verify successful upload.

### Implementation for User Story 8

- [X] T027 [US8] Implement upload_appbundle(upload_params, file_path) method with multipart form POST in raps-da/src/lib.rs
- [X] T028 [US8] Add `appbundle-upload` subcommand to DaCommands enum in raps-cli/src/commands/da.rs
- [X] T029 [US8] Implement CLI handler for appbundle-upload with file validation and progress output in raps-cli/src/commands/da.rs
- [X] T030 [US8] Add unit test for file path validation and upload parameter checks in raps-da/src/lib.rs

**Checkpoint**: App bundle archives can be uploaded through the CLI

---

## Phase 11: Polish & Cross-Cutting Concerns

**Purpose**: Final validation across all changes

- [X] T031 Run `cargo check --workspace` to verify full workspace compiles
- [X] T032 Run `cargo test --workspace` to verify zero test regressions
- [X] T033 Run `cargo clippy --workspace -- -D warnings` for lint compliance
- [X] T034 Run `cargo fmt -- --check` for formatting compliance
- [X] T035 Validate quickstart.md integration scenarios match implemented CLI commands

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — verify prior work immediately
- **Foundational (Phase 2)**: Skipped — no cross-crate prerequisites
- **US1 (Phase 3)**: Depends on Setup — can start immediately after
- **US2 (Phase 4)**: Depends on Setup — can run in parallel with US1 (different files)
- **US3 (Phase 5)**: Depends on Setup — can run in parallel with US1/US2 (different crate)
- **US4 (Phase 6)**: Verification only — can run anytime after Setup
- **US5 (Phase 7)**: Depends on Setup — can run in parallel with US1-US3 (different crate)
- **US6 (Phase 8)**: Verification only — can run anytime after Setup
- **US7 (Phase 9)**: Verification only — can run anytime after Setup
- **US8 (Phase 10)**: Depends on Setup — can run in parallel with US1-US5 (different crate)
- **Polish (Phase 11)**: Depends on ALL user stories being complete

### User Story Dependencies

- **US1 (P1)**: Independent — only touches translate.rs
- **US2 (P1)**: Independent — touches raps-derivative/src/lib.rs and translate.rs (shared file with US1, but different sections)
- **US3 (P2)**: Independent — touches raps-oss/src/lib.rs and object.rs
- **US4 (P2)**: Verification only — raps-dm/src/lib.rs
- **US5 (P2)**: Independent — touches raps-acc/src/users.rs and admin.rs
- **US6 (P3)**: Verification only — raps-webhooks/src/lib.rs and webhook.rs
- **US7 (P3)**: Verification only — raps-reality/src/lib.rs and reality.rs
- **US8 (P3)**: Independent — touches raps-da/src/lib.rs and da.rs

### Within Each User Story

- Types before client methods
- Client methods before CLI subcommands
- CLI subcommands before tests (tests verify end-to-end)

### Parallel Opportunities

- US1 and US2 can run in parallel (US1 touches translate.rs polling section; US2 adds new subcommands)
- US3, US5, US8 can all run in parallel (different crates entirely)
- US4, US6, US7 are verification-only and can run in parallel with anything
- Within US2: T008 and T009 are parallel (independent client methods)
- Within US3: T016 and T017 are parallel after T015 (both compose from copy_object)

---

## Parallel Example: User Story 2

```bash
# After types (T005, T006) are done, launch parallel client methods:
Task: "Implement get_object_tree() in raps-derivative/src/lib.rs"    # T008
Task: "Implement get_properties() in raps-derivative/src/lib.rs"      # T009

# After all client methods done, CLI and tests can proceed:
Task: "Add subcommands to TranslateCommands in translate.rs"          # T011
Task: "Add unit tests in raps-derivative/src/lib.rs"                  # T013
```

## Parallel Example: Independent Stories

```bash
# After Setup (Phase 1), launch all independent stories simultaneously:
Task: "US1 — translate.rs timeout"       # Phase 3 (raps-cli/commands/translate.rs)
Task: "US3 — OSS batch operations"       # Phase 5 (raps-oss + object.rs)
Task: "US5 — Parallel user imports"      # Phase 7 (raps-acc + admin.rs)
Task: "US8 — DA appbundle upload"        # Phase 10 (raps-da + da.rs)
```

---

## Implementation Strategy

### MVP First (P1 Stories: US1 + US2)

1. Complete Phase 1: Setup (verify prior work)
2. Complete Phase 3: US1 — Translate polling timeout
3. Complete Phase 4: US2 — Model Derivative metadata endpoints
4. **STOP and VALIDATE**: `cargo test --workspace`, `cargo clippy`
5. All polling operations have timeouts, all MD read endpoints available

### Incremental Delivery

1. Setup → Verify prior work → Foundation ready
2. US1 + US2 → P1 stories complete → Core functionality (MVP)
3. US3 → Batch OSS operations → Power user features
4. US5 → Parallel imports → Performance improvement
5. US8 → DA upload → Specialized workflow
6. US4 + US6 + US7 → Verify prior work → Carried-forward fixes confirmed
7. Polish → Full validation → Release ready

### Task Summary

| Phase | Story | Tasks | New Code | Verification Only |
|-------|-------|-------|----------|-------------------|
| 1 | Setup | 2 | 0 | 2 |
| 3 | US1 | 2 | 2 | 0 |
| 4 | US2 | 9 | 9 | 0 |
| 5 | US3 | 7 | 7 | 0 |
| 6 | US4 | 1 | 0 | 1 |
| 7 | US5 | 3 | 3 | 0 |
| 8 | US6 | 1 | 0 | 1 |
| 9 | US7 | 1 | 0 | 1 |
| 10 | US8 | 4 | 4 | 0 |
| 11 | Polish | 5 | 0 | 5 |
| **Total** | | **35** | **25** | **10** |

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Prior work (US4, US6, US7) needs only verification, not re-implementation
- US1 and US2 share translate.rs but modify different sections (polling vs new subcommands)
- All batch operations use Semaphore(10) for rate-limit safety
- Commit after each completed user story phase
- Stop at any checkpoint to validate story independently

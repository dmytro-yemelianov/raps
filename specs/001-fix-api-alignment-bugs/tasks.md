# Tasks: Fix API Alignment Bugs

**Input**: Design documents from `/specs/001-fix-api-alignment-bugs/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/api-changes.md

**Tests**: MANDATORY per Constitution Principle V. Tests are written first and must fail before implementation.

**Organization**: Tasks are grouped by user story. All 5 user stories are fully independent (different crates, no shared state) and can be implemented in any order or in parallel.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Rust workspace with per-crate `src/` directories:

```text
raps-dm/src/lib.rs              # US1: Pagination
raps-derivative/src/lib.rs      # US2: Region + Force
raps-cli/src/commands/translate.rs  # US2: CLI flags
raps-acc/src/{lib,admin,permissions,users}.rs  # US3: Project ID
raps-kernel/src/auth.rs         # US4: Token refresh
raps-reality/src/lib.rs         # US5: MIME type
```

---

## Phase 1: Setup

**Purpose**: Verify workspace baseline before making changes

- [X] T001 Verify workspace builds cleanly with `cargo check --workspace` and all existing tests pass with `cargo test --workspace`

**Checkpoint**: Workspace is green — safe to begin independent bug fixes

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: No shared foundational work needed — all 5 user stories modify independent crates with no cross-crate dependencies. Proceed directly to user story phases.

**Checkpoint**: N/A — user story implementation can begin immediately after Phase 1

---

## Phase 3: User Story 1 - Complete Data Retrieval for Large Projects (Priority: P1)

**Goal**: Follow all `links.next` pagination URLs in `list_projects()`, `list_folder_contents()`, and `get_item_versions()` so list operations return complete result sets instead of silently truncating at the first page.

**Independent Test**: Run `cargo test -p raps-dm` — pagination tests verify multi-page accumulation, empty-page continuation, max-page safety cap, and single-page no-op behavior.

### Tests for User Story 1

> **Write these tests FIRST, ensure they FAIL before implementation**

- [X] T002 [US1] Write unit test `test_pagination_follows_next_links` in raps-dm/src/lib.rs that verifies a paginated response with 3 pages accumulates all items (mock `JsonApiResponse` structs with `links.next` set)
- [X] T003 [P] [US1] Write unit test `test_pagination_stops_when_no_next` in raps-dm/src/lib.rs that verifies pagination stops when `links.next` is `None`
- [X] T004 [P] [US1] Write unit test `test_pagination_continues_on_empty_page` in raps-dm/src/lib.rs that verifies pagination continues when a page has zero items but `links.next` is present (FR-021)
- [X] T005 [P] [US1] Write unit test `test_pagination_max_page_cap` in raps-dm/src/lib.rs that verifies pagination stops after 100 pages even if `links.next` is still present (FR-005)

### Implementation for User Story 1

- [X] T006 [US1] Implement pagination loop in `list_projects()` in raps-dm/src/lib.rs: after initial request, follow `links.next.href` URLs accumulating `data` items into result `Vec`, with max 100 page cap. Use the existing `JsonApiResponse<T>` struct's `links` field (lines 137-158)
- [X] T007 [US1] Implement pagination loop in `list_folder_contents()` in raps-dm/src/lib.rs using the same pattern as `list_projects()` (follow `links.next`, accumulate, 100 page cap)
- [X] T008 [US1] Implement pagination loop in `get_item_versions()` in raps-dm/src/lib.rs using the same pattern as `list_projects()` (follow `links.next`, accumulate, 100 page cap)
- [X] T009 [US1] Verify all US1 tests pass with `cargo test -p raps-dm` and run `cargo clippy -p raps-dm -- -D warnings`

**Checkpoint**: All list operations follow pagination links. Tests confirm multi-page, empty-page, and safety-cap behaviors.

---

## Phase 4: User Story 2 - Region-Aware Model Translation (Priority: P1)

**Goal**: Add `--region` and `--force` flags to `raps translate start`. Route translation jobs to the configured region via `x-ads-region` header. Default `force` to `false` (previously hardcoded `true`) with deprecation notice.

**Independent Test**: Run `cargo test -p raps-derivative` and `cargo test -p raps-cli` — tests verify MdRegion parsing, correct header generation, force-flag behavior, and CLI argument handling.

### Tests for User Story 2

> **Write these tests FIRST, ensure they FAIL before implementation**

- [X] T010 [US2] Write unit test `test_md_region_display` in raps-derivative/src/lib.rs that verifies all 8 MdRegion variants produce correct display strings ("US", "EMEA", "AUS", "CAN", "DEU", "IND", "JPN", "GBR")
- [X] T011 [P] [US2] Write unit test `test_md_region_from_str` in raps-derivative/src/lib.rs that verifies case-insensitive parsing: "emea" → EMEA, "US" → US, "aus" → AUS, and invalid values return error with valid options listed
- [X] T012 [P] [US2] Write unit test `test_md_region_default_is_us` in raps-derivative/src/lib.rs that verifies `MdRegion::default()` returns `MdRegion::US`
- [X] T013 [P] [US2] Write unit test `test_translate_force_default_false` in raps-derivative/src/lib.rs that verifies calling `translate()` with `force=false` does NOT include `x-ads-force` header (or includes `x-ads-force: false`)

### Implementation for User Story 2

- [X] T014 [US2] Create `MdRegion` enum with 8 variants (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR) in raps-derivative/src/lib.rs with `Display`, `FromStr` (case-insensitive), `Clone`, `Copy`, `Debug`, `Default` (→ US) trait implementations. See data-model.md entity #2
- [X] T015 [US2] Update `translate()` signature in raps-derivative/src/lib.rs to add `region: MdRegion` and `force: bool` parameters. Replace hardcoded `region: "us"` (line ~293) with `region.to_string().to_lowercase()`. Replace hardcoded `.header("x-ads-force", "true")` (line ~315) with conditional: only add header when `force=true`. Add `.header("x-ads-region", region.to_string())` to the request. See contracts C2
- [X] T016 [US2] Add `--region` and `--force` flags to the `Start` subcommand in raps-cli/src/commands/translate.rs (lines ~22-38). `--region` takes a string value with default "US", parsed into `MdRegion`. `--force` is a boolean flag defaulting to false. Add deprecation notice in help text for `--force`: "Note: Prior versions always forced re-translation. The default is now to preserve existing manifests." See contracts C7
- [X] T017 [US2] Update the `translate()` call site in raps-cli/src/commands/translate.rs (line ~252-254) to pass the new `region` and `force` arguments from CLI flags
- [X] T018 [US2] Verify all US2 tests pass with `cargo test -p raps-derivative` and `cargo test -p raps-cli`, then run `cargo clippy -p raps-derivative -p raps-cli -- -D warnings`

**Checkpoint**: Translation jobs route to configured region. Force-translate defaults to off. CLI has `--region` and `--force` flags with deprecation notice.

---

## Phase 5: User Story 3 - Consistent Project ID Handling Across Modules (Priority: P2)

**Goal**: Replace three private `normalize_project_id()` functions (which do opposite things) with two clearly-named shared public functions: `strip_project_prefix()` and `ensure_project_prefix()`.

**Independent Test**: Run `cargo test -p raps-acc` — tests verify both functions handle BIM 360 "b." prefix, ACC "a." prefix, raw UUIDs, already-normalized IDs, and unknown formats.

### Tests for User Story 3

> **Write these tests FIRST, ensure they FAIL before implementation**

- [X] T019 [US3] Write unit tests for `strip_project_prefix()` in raps-acc/src/lib.rs: test stripping "b." prefix from "b.{uuid}", stripping "a." prefix from "a.{base64}", no-op on already-stripped UUID, pass-through on unknown format
- [X] T020 [P] [US3] Write unit tests for `ensure_project_prefix()` in raps-acc/src/lib.rs: test adding "b." prefix to raw UUID, no-op on already-prefixed "b.{uuid}", pass-through on "a."-prefixed IDs, pass-through on unknown format

### Implementation for User Story 3

- [X] T021 [US3] Create public functions `strip_project_prefix(id: &str) -> String` and `ensure_project_prefix(id: &str) -> String` in raps-acc/src/lib.rs. `strip_project_prefix` removes "b." or "a." prefix if present. `ensure_project_prefix` adds "b." prefix if no "b." or "a." prefix exists. Both are idempotent. See data-model.md entity #3
- [X] T022 [US3] Replace private `normalize_project_id()` in raps-acc/src/admin.rs (lines ~819-825) with call to `strip_project_prefix()`. Update all call sites in admin.rs to use the shared function
- [X] T023 [P] [US3] Replace private `normalize_project_id()` in raps-acc/src/permissions.rs (lines ~323-330) with call to `ensure_project_prefix()`. Update all call sites in permissions.rs to use the shared function
- [X] T024 [P] [US3] Replace private `normalize_project_id()` in raps-acc/src/users.rs (lines ~416-422) with call to `strip_project_prefix()`. Update all call sites in users.rs to use the shared function
- [X] T025 [US3] Verify all US3 tests pass with `cargo test -p raps-acc` and run `cargo clippy -p raps-acc -- -D warnings`

**Checkpoint**: All ACC modules use shared, correctly-named project ID functions. Same project ID produces correct API calls in admin, permissions, and users modules.

---

## Phase 6: User Story 4 - Reliable Concurrent Authentication (Priority: P2)

**Goal**: Replace `Arc<RwLock<Option<StoredToken>>>` with `Arc<tokio::sync::Mutex<TokenCache>>` to eliminate the TOCTOU race condition in 3-legged token refresh. Only one refresh occurs at a time; concurrent callers wait for the result.

**Independent Test**: Run `cargo test -p raps-kernel` — tests verify single-refresh-under-concurrency, waiter behavior, and failed-refresh-preserves-token semantics.

### Tests for User Story 4

> **Write these tests FIRST, ensure they FAIL before implementation**

- [X] T026 [US4] Write unit test `test_token_cache_single_refresh` in raps-kernel/src/auth.rs that verifies when multiple concurrent tasks detect an expired token, only one refresh HTTP call is made (use a counter or mock)
- [X] T027 [P] [US4] Write unit test `test_token_cache_waiters_get_new_token` in raps-kernel/src/auth.rs that verifies waiting tasks receive the newly refreshed token after the refresh completes
- [X] T028 [P] [US4] Write unit test `test_token_cache_failed_refresh_preserves_token` in raps-kernel/src/auth.rs that verifies a failed refresh does NOT clear a previously valid cached token (FR-016)

### Implementation for User Story 4

- [X] T029 [US4] Create `TokenCache` struct with `token: Option<StoredToken>` and `refreshing: bool` fields in raps-kernel/src/auth.rs. See data-model.md entity #4 for state transitions
- [X] T030 [US4] Replace `cached_3leg_token: Arc<RwLock<Option<StoredToken>>>` with `cached_3leg_token: Arc<tokio::sync::Mutex<TokenCache>>` in raps-kernel/src/auth.rs (line ~89-90). Update the struct initialization
- [X] T031 [US4] Rewrite `get_3leg_token()` in raps-kernel/src/auth.rs (lines ~179-201) to use Mutex-based coordination: acquire lock → check if refreshing → if yes, release lock and wait 100ms then re-check → if no, set refreshing=true, release lock, perform refresh, acquire lock, update token, set refreshing=false
- [X] T032 [US4] Update `refresh_token()` in raps-kernel/src/auth.rs (lines ~671-723) to NOT clear the cached token on failure. On failure: acquire lock, set refreshing=false, propagate error. On success: acquire lock, update token, set refreshing=false
- [X] T033 [US4] Verify all US4 tests pass with `cargo test -p raps-kernel` and run `cargo clippy -p raps-kernel -- -D warnings`

**Checkpoint**: Concurrent token refresh is safe. Only one refresh occurs at a time, waiters receive the new token, failed refreshes preserve valid cached tokens.

---

## Phase 7: User Story 5 - Upload Non-JPEG Photos for Reality Capture (Priority: P3)

**Goal**: Replace hardcoded `"image/jpeg"` MIME type with extension-based detection supporting JPEG, PNG, TIFF, BMP, WebP, GIF, and RAW formats with `application/octet-stream` fallback.

**Independent Test**: Run `cargo test -p raps-reality` — tests verify correct MIME type for each supported extension and fallback behavior.

### Tests for User Story 5

> **Write these tests FIRST, ensure they FAIL before implementation**

- [X] T034 [US5] Write unit tests for `mime_type_from_extension()` in raps-reality/src/lib.rs: verify "photo.jpg" → "image/jpeg", "photo.jpeg" → "image/jpeg", "photo.png" → "image/png", "photo.tiff" → "image/tiff", "photo.tif" → "image/tiff", "photo.bmp" → "image/bmp", "photo.webp" → "image/webp", "photo.gif" → "image/gif"
- [X] T035 [P] [US5] Write unit test `test_mime_fallback` in raps-reality/src/lib.rs that verifies "photo.raw" → "application/octet-stream", "photo.xyz" → "application/octet-stream", "photo.RAW" → "application/octet-stream" (case-insensitive)
- [X] T036 [P] [US5] Write unit test `test_mime_case_insensitive` in raps-reality/src/lib.rs that verifies "photo.PNG" → "image/png", "photo.JPEG" → "image/jpeg", "photo.Tiff" → "image/tiff"

### Implementation for User Story 5

- [X] T037 [US5] Create `fn mime_type_from_extension(filename: &str) -> &'static str` in raps-reality/src/lib.rs. Extract extension from filename, lowercase it, match against known extensions (jpg/jpeg → "image/jpeg", png → "image/png", tiff/tif → "image/tiff", bmp → "image/bmp", webp → "image/webp", gif → "image/gif"), fallback to "application/octet-stream". See data-model.md entity #5
- [X] T038 [US5] Replace hardcoded `.mime_str("image/jpeg")` (line ~301) in `upload_photos()` in raps-reality/src/lib.rs with `.mime_str(mime_type_from_extension(&filename))` using the filename already extracted at lines ~283-286
- [X] T039 [US5] Verify all US5 tests pass with `cargo test -p raps-reality` and run `cargo clippy -p raps-reality -- -D warnings`

**Checkpoint**: Reality Capture uploads use correct MIME types for all supported image formats with fallback for unrecognized extensions.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Full workspace validation and documentation

- [X] T040 Run full workspace test suite: `cargo test --workspace`
- [X] T041 Run full workspace lint: `cargo clippy --workspace -- -D warnings`
- [X] T042 Run formatting check: `cargo fmt -- --check`
- [X] T043 [P] Update CLI help text and any relevant documentation to reflect new `--region` and `--force` flags, deprecation notice for force-translate default change (FR-020)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Skipped — no shared prerequisites
- **User Stories (Phase 3-7)**: All depend on Phase 1 completion only. All 5 are fully independent
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

- **US1 (P1) — Pagination**: Independent. Only touches raps-dm/src/lib.rs
- **US2 (P1) — Region + Force**: Independent. Touches raps-derivative/src/lib.rs and raps-cli/src/commands/translate.rs
- **US3 (P2) — Project ID**: Independent. Only touches raps-acc/src/{lib,admin,permissions,users}.rs
- **US4 (P2) — Token Refresh**: Independent. Only touches raps-kernel/src/auth.rs
- **US5 (P3) — MIME Type**: Independent. Only touches raps-reality/src/lib.rs

**No cross-story dependencies.** All 5 stories can execute in parallel.

### Within Each User Story

1. Tests MUST be written and FAIL before implementation
2. Implementation tasks follow the order listed (some marked [P] for parallel within-story)
3. Verification task confirms all tests pass and clippy is clean

### Parallel Opportunities

All 5 user stories can run simultaneously — they modify completely different crates:

```
Phase 1 (Setup) ─────────────────────────────────────────────────
    │
    ├── Phase 3 (US1: raps-dm)          ─── T002-T009
    ├── Phase 4 (US2: raps-derivative)  ─── T010-T018
    ├── Phase 5 (US3: raps-acc)         ─── T019-T025
    ├── Phase 6 (US4: raps-kernel)      ─── T026-T033
    └── Phase 7 (US5: raps-reality)     ─── T034-T039
                                              │
Phase 8 (Polish) ────────────────────────────────────────────────
```

---

## Parallel Example: All User Stories

```bash
# After Phase 1, launch all 5 stories in parallel (different crates, zero conflicts):
Agent 1: US1 — raps-dm pagination (T002-T009)
Agent 2: US2 — raps-derivative region+force + raps-cli flags (T010-T018)
Agent 3: US3 — raps-acc project ID normalization (T019-T025)
Agent 4: US4 — raps-kernel token refresh (T026-T033)
Agent 5: US5 — raps-reality MIME detection (T034-T039)
```

## Parallel Example: Within User Story 1

```bash
# Tests can run in parallel (different test functions, no shared state):
Task: "T003 [P] test_pagination_stops_when_no_next in raps-dm/src/lib.rs"
Task: "T004 [P] test_pagination_continues_on_empty_page in raps-dm/src/lib.rs"
Task: "T005 [P] test_pagination_max_page_cap in raps-dm/src/lib.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 3: US1 — Pagination (highest user impact, BLOCKING severity)
3. **STOP and VALIDATE**: `cargo test -p raps-dm` — all list operations return complete data
4. This alone fixes the most severe class of bug (silent data truncation)

### Incremental Delivery

1. Setup → Baseline green
2. US1 (Pagination) → Complete data retrieval (MVP)
3. US2 (Region + Force) → Correct data center routing + manifest safety
4. US3 (Project ID) → Consistent cross-module behavior
5. US4 (Token Refresh) → Reliable concurrent auth
6. US5 (MIME Type) → Full image format support
7. Polish → Full validation, documentation

### Parallel Team Strategy

With multiple developers/agents:

1. All complete Phase 1 setup verification
2. Once Phase 1 is green, each developer takes one user story:
   - Developer A: US1 (raps-dm)
   - Developer B: US2 (raps-derivative + raps-cli)
   - Developer C: US3 (raps-acc)
   - Developer D: US4 (raps-kernel)
   - Developer E: US5 (raps-reality)
3. All stories complete independently, then Phase 8 validates everything together

---

## Notes

- [P] tasks = different files/functions, no dependencies on incomplete tasks
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable within its crate
- Tests MUST fail before implementation (TDD per Constitution Principle V)
- Commit after each completed user story (natural commit boundary = one crate)
- Stop at any checkpoint to validate story independently
- All 5 stories are in different crates — zero risk of merge conflicts when parallelized

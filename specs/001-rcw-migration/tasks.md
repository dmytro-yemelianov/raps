# Tasks: RCW Migration Automation

**Input**: Design documents from `/specs/001-rcw-migration/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Tests are not explicitly requested in the feature specification. Unit tests will be included for critical data types only.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

This project uses Rust workspace with multiple crates:
- **raps-da/src/**: Design Automation library (new rcw.rs module)
- **raps-dm/src/**: Data Management library (extend with relationships)
- **raps-cli/src/commands/**: CLI command implementations
- **raps-cli/tests/**: Integration tests

---

## Phase 1: Setup ✅

**Purpose**: Project structure and module scaffolding

- [x] T001 Create rcw.rs module file in raps-da/src/rcw.rs
- [x] T002 [P] Add `pub mod rcw;` to raps-da/src/lib.rs
- [x] T003 [P] Re-export RCW types in raps-da/src/lib.rs

---

## Phase 2: Foundational (Blocking Prerequisites) ✅

**Purpose**: Core types and DM extensions that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Define RevitEngine enum (Revit2025, Revit2026) in raps-da/src/rcw.rs
- [x] T005 [P] Define MigrationStatus enum in raps-da/src/rcw.rs
- [x] T006 [P] Define RcwModel struct in raps-da/src/rcw.rs
- [x] T007 [P] Define MigrationParams struct with PascalCase serde in raps-da/src/rcw.rs
- [x] T008 [P] Define MigrationJob struct in raps-da/src/rcw.rs
- [x] T009 [P] Define RcwActivityConfig struct in raps-da/src/rcw.rs
- [x] T010 Add VersionWithRelationships struct to raps-dm/src/lib.rs
- [x] T011 [P] Add VersionRelationships, StorageRelationship, StorageData structs to raps-dm/src/lib.rs
- [x] T012 Add get_version_with_storage method to DataManagementClient in raps-dm/src/lib.rs
- [x] T013 Create RcwCommands enum skeleton in raps-cli/src/commands/da.rs
- [x] T014 Wire RcwCommands subcommand into DaCommands enum in raps-cli/src/commands/da.rs

**Checkpoint**: Foundation ready - types defined, DM extended, CLI skeleton in place

---

## Phase 3: User Story 3 - Configure Migration Environment (Priority: P1) 🎯 MVP ✅

**Goal**: One-time setup of AppBundle and Activity for RCW migration

**Independent Test**: Run `raps da rcw configure --engine 2026` and verify AppBundle/Activity are created

### Implementation for User Story 3

- [x] T015 [US3] Implement create_rcw_appbundle method in raps-da/src/rcw.rs
- [x] T016 [US3] Implement create_rcw_activity method in raps-da/src/rcw.rs
- [x] T017 [US3] Implement configure_migration_environment orchestrator in raps-da/src/rcw.rs
- [x] T018 [US3] Implement RcwCommands::Configure handler in raps-cli/src/commands/da.rs
- [x] T019 [US3] Add --engine, --alias, --force CLI options for configure command
- [x] T020 [US3] Implement table and JSON output formats for configure result

**Checkpoint**: User Story 3 complete - `raps da rcw configure` works independently

---

## Phase 4: User Story 2 - List RCW Models (Priority: P1) ✅

**Goal**: Discover RCW models (C4RModel type) in BIM 360 folders

**Independent Test**: Run `raps da rcw list <folder-url>` and verify only C4RModel files are listed

### Implementation for User Story 2

- [x] T021 [US2] Implement is_rcw_model helper function in raps-da/src/rcw.rs
- [x] T022 [US2] Implement list_rcw_models method using DM client in raps-da/src/rcw.rs
- [x] T023 [US2] Implement list_rcw_models_recursive for nested folders in raps-da/src/rcw.rs
- [x] T024 [US2] Implement RcwCommands::List handler in raps-cli/src/commands/da.rs
- [x] T025 [US2] Add --recursive, --limit CLI options for list command
- [x] T026 [US2] Implement table output with Name, Size, Modified columns
- [x] T027 [US2] Implement JSON output for list command

**Checkpoint**: User Story 2 complete - `raps da rcw list` works independently

---

## Phase 5: User Story 1 - Migrate Single RCW Model (Priority: P1) ✅

**Goal**: Migrate one RCW model from BIM 360 to ACC Docs via Design Automation

**Independent Test**: Run `raps da rcw migrate <source> <dest> --wait` and verify model appears in ACC

### Implementation for User Story 1

- [x] T028 [US1] Implement parse_folder_url helper to extract project_id, folder_id in raps-da/src/rcw.rs
- [x] T029 [US1] Implement parse_item_url helper to extract project_id, item_id in raps-da/src/rcw.rs
- [x] T030 [US1] Implement get_rcw_model_details to fetch storage_id via DM in raps-da/src/rcw.rs
- [x] T031 [US1] Implement build_migration_params to construct target params in raps-da/src/rcw.rs
- [x] T032 [US1] Implement submit_migration_workitem using DA client in raps-da/src/rcw.rs
- [x] T033 [US1] Implement poll_migration_status with 5s interval in raps-da/src/rcw.rs
- [x] T034 [US1] Implement migrate_rcw_model orchestrator in raps-da/src/rcw.rs
- [x] T035 [US1] Implement RcwCommands::Migrate handler in raps-cli/src/commands/da.rs
- [x] T036 [US1] Add --engine, --name, --wait CLI options for migrate command
- [x] T037 [US1] Implement progress spinner for --wait mode
- [x] T038 [US1] Implement table and JSON output for migration result

**Checkpoint**: User Story 1 complete - core migration works end-to-end

---

## Phase 6: User Story 6 - Check Migration Status (Priority: P2) ✅

**Goal**: Query status of migration jobs by workitem ID

**Independent Test**: Run `raps da rcw status <workitem-id>` and see current state

### Implementation for User Story 6

- [x] T039 [US6] Implement get_migration_status method in raps-da/src/rcw.rs
- [x] T040 [US6] Implement RcwCommands::Status handler in raps-cli/src/commands/da.rs
- [x] T041 [US6] Add --wait, --batch CLI options for status command
- [x] T042 [US6] Implement table output with progress bar for in-progress jobs
- [x] T043 [US6] Implement JSON output for status command

**Checkpoint**: User Story 6 complete - status monitoring works

---

## Phase 7: User Story 4 - Batch Migration (Priority: P2) ✅

**Goal**: Migrate all RCW models from source folder to destination

**Independent Test**: Run `raps da rcw batch <source-folder> <dest-folder>` and verify all models migrate

### Implementation for User Story 4

- [x] T044 [P] [US4] Define BatchMigration and BatchSummary structs in raps-da/src/rcw.rs
- [x] T045 [US4] Implement batch_migrate_rcw_models orchestrator in raps-da/src/rcw.rs
- [x] T046 [US4] Implement sequential submission with rate limiting in batch migrate
- [x] T047 [US4] Implement parallel status polling for batch jobs in raps-da/src/rcw.rs
- [x] T048 [US4] Implement RcwCommands::Batch handler in raps-cli/src/commands/da.rs
- [x] T049 [US4] Add --engine, --limit, --wait, --dry-run CLI options for batch command
- [x] T050 [US4] Implement table output with job summary table
- [x] T051 [US4] Implement JSON output for batch result with summary stats

**Checkpoint**: User Story 4 complete - batch migration works ✅

---

## Phase 8: User Story 5 - Cancel Migration (Priority: P3) ✅

**Goal**: Cancel pending or in-progress migration jobs

**Independent Test**: Start a migration, then run `raps da rcw cancel <workitem-id>`

### Implementation for User Story 5

- [x] T052 [US5] Implement cancel_migration method using DA workitem cancel in raps-da/src/rcw.rs
- [x] T053 [US5] Implement RcwCommands::Cancel handler in raps-cli/src/commands/da.rs
- [x] T054 [US5] Add --batch CLI option for cancel command
- [x] T055 [US5] Implement error handling for already-completed jobs

**Checkpoint**: User Story 5 complete - cancel functionality works ✅

---

## Phase 9: Polish & Cross-Cutting Concerns ✅

**Purpose**: Error handling, logging, and documentation

- [x] T056 [P] Add actionable error messages with remediation hints in raps-da/src/rcw.rs
- [x] T057 [P] Add timestamp logging for migration operations
- [x] T058 [P] Add permission validation before migration (source read, dest write)
- [ ] T059 [P] Handle token expiry gracefully with re-auth prompt
- [x] T060 [P] Add unit tests for URL parsing helpers in raps-da/src/rcw.rs
- [x] T061 [P] Add unit tests for MigrationStatus state transitions
- [x] T062 Update quickstart.md with actual command examples
- [ ] T063 Run full workflow validation per quickstart.md

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 3 (Phase 3)**: Configure - can start after Foundational
- **User Story 2 (Phase 4)**: List - can start after Foundational
- **User Story 1 (Phase 5)**: Migrate - depends on US3 (needs configured environment)
- **User Story 6 (Phase 6)**: Status - can start after Foundational (but useful after US1)
- **User Story 4 (Phase 7)**: Batch - depends on US1 (reuses single migration)
- **User Story 5 (Phase 8)**: Cancel - depends on US1 (needs running migrations)
- **Polish (Phase 9)**: Depends on all user stories

### User Story Dependencies

```
Foundational (Phase 2)
        │
        ├───────────────┬───────────────┐
        ▼               ▼               ▼
    US3 Configure   US2 List      US6 Status
        │                               │
        ▼                               │
    US1 Migrate ◄───────────────────────┘
        │
        ├───────────────┐
        ▼               ▼
    US4 Batch      US5 Cancel
```

### Parallel Opportunities

**After Foundational completes:**
- US3 (Configure), US2 (List), US6 (Status) can start in parallel

**Within each user story:**
- Tasks marked [P] can run in parallel

---

## Parallel Example: Foundational Phase

```bash
# Launch all type definitions in parallel:
Task: "Define RevitEngine enum in raps-da/src/rcw.rs"
Task: "Define MigrationStatus enum in raps-da/src/rcw.rs"
Task: "Define RcwModel struct in raps-da/src/rcw.rs"
Task: "Define MigrationParams struct in raps-da/src/rcw.rs"
Task: "Define MigrationJob struct in raps-da/src/rcw.rs"
```

## Parallel Example: After Foundational

```bash
# Different developers can work on different user stories:
Developer A: US3 Configure (T015-T020)
Developer B: US2 List (T021-T027)
Developer C: US6 Status (T039-T043)
```

---

## Implementation Strategy

### MVP First (User Stories 3, 2, 1)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: US3 Configure
4. Complete Phase 4: US2 List
5. Complete Phase 5: US1 Migrate
6. **STOP and VALIDATE**: Test single migration end-to-end
7. Deploy/demo MVP

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add US3 Configure → Can set up environment
3. Add US2 List → Can discover RCW models
4. Add US1 Migrate → **MVP: Single file migration works!**
5. Add US6 Status → Better monitoring
6. Add US4 Batch → Production batch operations
7. Add US5 Cancel → Error recovery capability
8. Polish → Production ready

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story
- RevitEngine defaults to 2026 per research.md
- MigrationParams uses PascalCase per data-model.md
- All commands support --format json/yaml/table per contracts
- Error messages must include remediation per FR-010

---

## Phase 10: Private Plugin Extraction (Commercial Distribution)

**Purpose**: Extract RCW migration to a separate private repository for commercial licensing

### Completed Tasks

- [x] T064 Create private `raps-rcw` plugin repository structure
- [x] T065 Configure Cargo.toml with dependencies on published raps-* crates
- [x] T066 Create standalone CLI in src/main.rs using RAPS plugin system
- [x] T067 Adapt rcw.rs to use external crate imports (raps_da::, raps_dm::)
- [x] T068 Verify plugin builds and passes clippy
- [x] T069 Create README.md with usage documentation

### Plugin Location

The private plugin is at: `C:\github\raps\raps-rcw`

### Distribution Notes

- Plugin binary named `raps-rcw` is auto-discovered by RAPS CLI plugin system
- Uses path dependencies for development; switch to crates.io versions for release
- Binary can be distributed to paid customers independently

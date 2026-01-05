# Tasks: Monorepo Microkernel App

**Input**: Design documents from `/specs/002-monorepo-microkernel-app/`
**Prerequisites**: plan.md ✅, spec.md ✅

**Tests**: Tests are included as they are critical for verifying monorepo functionality and architectural boundaries.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

- **Monorepo workspace**: `raps/` (repository root)
- **Crates**: `raps-kernel/`, `raps-oss/`, `raps-derivative/`, `raps-dm/`, `raps-ssa/`, `raps-community/`, `raps-enterprise/`, `raps/`
- **Tests**: `tests/` directory in each crate

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create monorepo workspace structure and basic configuration

- [ ] T001 Create workspace root directory structure in `raps/`
- [ ] T002 [P] Create `raps/Cargo.toml` with workspace configuration (members, dependencies, profiles)
- [ ] T003 [P] Create `.cargo/config.toml` with linker configuration (lld-link for Windows, mold for Linux)
- [ ] T004 [P] Create directory structure for all 8 crates: `raps-kernel/`, `raps-oss/`, `raps-derivative/`, `raps-dm/`, `raps-ssa/`, `raps-community/`, `raps-enterprise/`, `raps/`
- [ ] T005 [P] Initialize git repository in `raps/` if not already initialized
- [ ] T006 [P] Create `.gitignore` with Rust patterns (target/, Cargo.lock, etc.)
- [ ] T007 [P] Create `README.md` documenting monorepo structure and workspace commands

**Checkpoint**: Workspace structure created - ready for foundational setup

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Workspace Configuration

- [ ] T008 Create workspace `Cargo.toml` with `[workspace]` section listing all 8 members
- [ ] T009 [P] Define `[workspace.package]` with version, edition, authors, license
- [ ] T010 [P] Define `[workspace.dependencies]` with shared dependencies (tokio, reqwest, serde, thiserror, anyhow, keyring, tracing)
- [ ] T011 [P] Configure `[profile.dev]` with `debug = 0`, `opt-level = 0`, `incremental = true`
- [ ] T012 [P] Configure `[profile.test]` with `debug = 0`, `opt-level = 0`
- [ ] T013 [P] Configure `[profile.release]` with `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`
- [ ] T014 [P] Configure `[workspace.lints.rust]` with `unsafe_code = "deny"`, `missing_docs = "warn"`

### Kernel Foundation (raps-kernel)

- [ ] T015 Create `raps-kernel/Cargo.toml` with package metadata referencing workspace
- [ ] T016 [P] Create `raps-kernel/src/lib.rs` with deny attributes: `#![deny(warnings)]`, `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`
- [ ] T017 [P] Create `raps-kernel/src/auth/mod.rs` module structure
- [ ] T018 [P] Create `raps-kernel/src/http/mod.rs` module structure
- [ ] T019 [P] Create `raps-kernel/src/config/mod.rs` module structure
- [ ] T020 [P] Create `raps-kernel/src/storage/mod.rs` module structure
- [ ] T021 [P] Create `raps-kernel/src/types/mod.rs` module structure
- [ ] T022 [P] Create `raps-kernel/src/error.rs` with error types and exit codes
- [ ] T023 [P] Create `raps-kernel/src/logging.rs` with tracing setup and secret redaction
- [ ] T024 Create `raps-kernel/tests/` directory for integration tests

### Service Crate Foundations

- [ ] T025 [P] Create `raps-oss/Cargo.toml` with dependency on `raps-kernel = { path = "../raps-kernel" }`
- [ ] T026 [P] Create `raps-derivative/Cargo.toml` with dependency on `raps-kernel`
- [ ] T027 [P] Create `raps-dm/Cargo.toml` with dependency on `raps-kernel`
- [ ] T028 [P] Create `raps-ssa/Cargo.toml` with dependency on `raps-kernel`
- [ ] T029 [P] Create `raps-oss/src/lib.rs` with basic module structure
- [ ] T030 [P] Create `raps-derivative/src/lib.rs` with basic module structure
- [ ] T031 [P] Create `raps-dm/src/lib.rs` with basic module structure
- [ ] T032 [P] Create `raps-ssa/src/lib.rs` with basic module structure

### Tier Crate Foundations

- [ ] T033 [P] Create `raps-community/Cargo.toml` with dependencies on kernel and service crates
- [ ] T034 [P] Create `raps-enterprise/Cargo.toml` with dependencies on kernel and service crates
- [ ] T035 [P] Create `raps-community/src/lib.rs` with basic module structure
- [ ] T036 [P] Create `raps-enterprise/src/lib.rs` with basic module structure

### CLI Binary Foundation

- [ ] T037 Create `raps/Cargo.toml` with dependencies on kernel, services, and tiers
- [ ] T038 Create `raps/src/main.rs` with basic CLI entry point
- [ ] T039 Create `raps/src/commands/mod.rs` module structure
- [ ] T040 Create `raps/src/output/mod.rs` module structure

### Build Tooling Setup

- [ ] T041 [P] Configure sccache in `.cargo/config.toml` with `rustc-wrapper = "sccache"`
- [ ] T042 [P] Verify sccache installation and configuration
- [ ] T043 [P] Configure lld-link for Windows in `.cargo/config.toml`
- [ ] T044 [P] Configure mold for Linux in `.cargo/config.toml` (CI only)

**Checkpoint**: Foundation ready - workspace configured, all crates created with basic structure, kernel foundation in place. User story implementation can now begin.

---

## Phase 3: User Story 1 - Atomic Cross-Module Changes (Priority: P1) 🎯 MVP

**Goal**: Enable developers to update kernel, service module, and CLI command in a single atomic commit

**Independent Test**: Make a change touching kernel types, service module, and CLI command; verify `cargo check` validates all changes together in a single commit.

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T045 [P] [US1] Integration test: Verify workspace builds after cross-module change in `raps/tests/integration/test_atomic_changes.rs`
- [ ] T046 [P] [US1] Test: Verify `cargo check --workspace` validates all crates together in `raps/tests/integration/test_workspace_check.rs`
- [ ] T047 [P] [US1] Test: Verify breaking kernel change fails all dependent crates in `raps/tests/integration/test_breaking_changes.rs`

### Implementation for User Story 1

- [ ] T048 [US1] Implement kernel type `Urn` in `raps-kernel/src/types/urn.rs` (newtype pattern)
- [ ] T049 [US1] Implement kernel type `BucketKey` in `raps-kernel/src/types/bucket_key.rs` (newtype pattern)
- [ ] T050 [US1] Implement OSS bucket creation in `raps-oss/src/bucket.rs` using kernel types
- [ ] T051 [US1] Implement CLI command `bucket create` in `raps/src/commands/bucket.rs` using OSS service
- [ ] T052 [US1] Add integration test demonstrating atomic change across kernel → service → CLI
- [ ] T053 [US1] Verify `cargo check --workspace` passes after atomic change
- [ ] T054 [US1] Document atomic change workflow in `raps/CONTRIBUTING.md`

**Checkpoint**: At this point, User Story 1 should be fully functional. Developers can make atomic changes across kernel, service, and CLI in a single commit, and `cargo check --workspace` validates all changes together.

---

## Phase 4: User Story 2 - Unified Development Workflow (Priority: P1)

**Goal**: Enable single command (`cargo check`) to validate entire workspace, catching integration issues immediately

**Independent Test**: Run `cargo check` in workspace root and verify it checks all crates, detecting breaking changes in dependent crates.

### Tests for User Story 2

- [ ] T055 [P] [US2] Test: Verify `cargo check` checks all workspace members in `raps/tests/integration/test_unified_check.rs`
- [ ] T056 [P] [US2] Test: Verify kernel change triggers dependent crate checks in `raps/tests/integration/test_dependency_propagation.rs`
- [ ] T057 [P] [US2] Test: Verify dependency version update propagates to all crates in `raps/tests/integration/test_version_propagation.rs`

### Implementation for User Story 2

- [ ] T058 [US2] Verify workspace `Cargo.toml` correctly lists all members
- [ ] T059 [US2] Test `cargo check --workspace` validates all 8 crates
- [ ] T060 [US2] Test `cargo clippy --workspace -- -D warnings` lints all crates
- [ ] T061 [US2] Test `cargo fmt --check --all` validates formatting across workspace
- [ ] T062 [US2] Create script `scripts/check-all.sh` (and `.ps1` for Windows) to run all checks
- [ ] T063 [US2] Document unified workflow commands in `raps/README.md`
- [ ] T064 [US2] Verify breaking change in kernel fails dependent crates immediately

**Checkpoint**: At this point, User Stories 1 AND 2 should both work. Developers can use `cargo check` to validate the entire workspace, catching integration issues immediately.

---

## Phase 5: User Story 3 - Independent Crate Testing (Priority: P2)

**Goal**: Enable developers to test individual crates in isolation without building entire workspace

**Independent Test**: Run `cargo test -p raps-oss` and verify only that crate's tests run, without building other crates.

### Tests for User Story 3

- [ ] T065 [P] [US3] Test: Verify `cargo test -p raps-kernel` runs only kernel tests in `raps/tests/integration/test_isolated_testing.rs`
- [ ] T066 [P] [US3] Test: Verify `cargo bench -p raps-kernel` runs only kernel benchmarks in `raps/tests/integration/test_isolated_benchmarking.rs`
- [ ] T067 [P] [US3] Test: Verify `cargo doc -p raps-derivative` generates only that crate's docs in `raps/tests/integration/test_isolated_docs.rs`

### Implementation for User Story 3

- [ ] T068 [US3] Add unit tests to `raps-kernel/tests/integration_test.rs` for kernel functionality
- [ ] T069 [US3] Add unit tests to `raps-oss/tests/integration_test.rs` for OSS functionality
- [ ] T070 [US3] Verify `cargo test -p raps-kernel` runs only kernel tests
- [ ] T071 [US3] Verify `cargo test -p raps-oss` runs only OSS tests
- [ ] T072 [US3] Create benchmark in `raps-kernel/benches/` and verify `cargo bench -p raps-kernel` works
- [ ] T073 [US3] Verify `cargo doc -p raps-derivative --open` generates only derivative docs
- [ ] T074 [US3] Document isolated testing commands in `raps/README.md`

**Checkpoint**: At this point, User Stories 1, 2, AND 3 should all work. Developers can test individual crates in isolation while still benefiting from workspace-level validation.

---

## Phase 6: User Story 4 - Consistent Dependency Versions (Priority: P2)

**Goal**: Ensure all crates use identical versions of shared dependencies, preventing version conflicts

**Independent Test**: Update `tokio` version in workspace `Cargo.toml`, run `cargo update`, and verify `Cargo.lock` shows single version for all crates.

### Tests for User Story 4

- [ ] T075 [P] [US4] Test: Verify workspace dependency update propagates to all crates in `raps/tests/integration/test_dependency_consistency.rs`
- [ ] T076 [P] [US4] Test: Verify `cargo tree` shows single version per dependency in `raps/tests/integration/test_dependency_tree.rs`
- [ ] T077 [P] [US4] Test: Verify dependency override in individual crate works with justification in `raps/tests/integration/test_dependency_override.rs`

### Implementation for User Story 4

- [ ] T078 [US4] Update all crate `Cargo.toml` files to use `{ workspace = true }` for shared dependencies
- [ ] T079 [US4] Verify `raps-kernel/Cargo.toml` references workspace dependencies correctly
- [ ] T080 [US4] Verify `raps-oss/Cargo.toml` references workspace dependencies correctly
- [ ] T081 [US4] Verify `raps-derivative/Cargo.toml` references workspace dependencies correctly
- [ ] T082 [US4] Verify `raps-dm/Cargo.toml` references workspace dependencies correctly
- [ ] T083 [US4] Verify `raps-ssa/Cargo.toml` references workspace dependencies correctly
- [ ] T084 [US4] Verify `raps-community/Cargo.toml` references workspace dependencies correctly
- [ ] T085 [US4] Verify `raps-enterprise/Cargo.toml` references workspace dependencies correctly
- [ ] T086 [US4] Verify `raps/Cargo.toml` references workspace dependencies correctly
- [ ] T087 [US4] Run `cargo update` and verify `Cargo.lock` shows single version per dependency
- [ ] T088 [US4] Run `cargo tree` and verify no duplicate dependencies in final binary
- [ ] T089 [US4] Document dependency management workflow in `raps/CONTRIBUTING.md`

**Checkpoint**: At this point, all user stories should be complete. Workspace uses consistent dependency versions, preventing conflicts and duplicate dependencies.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories and ensure production readiness

### CI/CD Integration

- [ ] T090 [P] Create `.github/workflows/ci.yml` with workspace validation
- [ ] T091 [P] Configure CI to run `cargo check --workspace --all-targets`
- [ ] T092 [P] Configure CI to run `cargo clippy --workspace -- -D warnings`
- [ ] T093 [P] Configure CI to run `cargo fmt --check --all`
- [ ] T094 [P] Configure CI to run `cargo nextest run --workspace`
- [ ] T095 [P] Configure CI to use sccache for compilation caching
- [ ] T096 [P] Configure CI to use mold linker on Linux
- [ ] T097 [P] Configure CI to build release artifacts with `cargo build --release --workspace`

### Performance Optimization

- [ ] T098 [P] Verify kernel incremental check completes in <5s (measure after single file change)
- [ ] T099 [P] Verify workspace incremental check completes in <30s (measure after single file change)
- [ ] T100 [P] Optimize kernel hot paths to minimize allocations
- [ ] T101 [P] Implement lazy initialization for optional features in kernel
- [ ] T102 [P] Verify CLI startup time <100ms for help/version commands

### Security Hardening

- [ ] T103 [P] Verify kernel compiles with `#![deny(unsafe_code)]` (no unsafe blocks)
- [ ] T104 [P] Verify kernel compiles with `#![deny(clippy::unwrap_used)]` (no unwrap/expect)
- [ ] T105 [P] Verify kernel compiles with `#![deny(warnings)]` (zero warnings)
- [ ] T106 [P] Implement secret redaction in `raps-kernel/src/logging.rs` for tokens, keys, credentials
- [ ] T107 [P] Verify HTTPS-only enforcement in HTTP client (reject non-HTTPS endpoints)
- [ ] T108 [P] Verify platform keyring integration works on Windows, macOS, Linux

### Documentation

- [ ] T109 [P] Update `raps/README.md` with monorepo structure and workspace commands
- [ ] T110 [P] Create `raps/CONTRIBUTING.md` with atomic change workflow
- [ ] T111 [P] Document kernel architecture in `raps-kernel/README.md`
- [ ] T112 [P] Generate unified documentation with `cargo doc --workspace --open`
- [ ] T113 [P] Verify all public APIs have rustdoc comments with examples

### Code Quality

- [ ] T114 [P] Run `cargo clippy --workspace -- -D warnings` and fix all warnings
- [ ] T115 [P] Run `cargo fmt --all` to format entire workspace
- [ ] T116 [P] Verify kernel LOC <3000 (excluding tests)
- [ ] T117 [P] Achieve >90% test coverage on kernel critical paths
- [ ] T118 [P] Verify zero circular dependencies with `cargo tree`

### Migration & Cleanup

- [ ] T119 [P] Archive separate kernel/module repositories (if they exist)
- [ ] T120 [P] Update distribution repos (`homebrew-tap`, `scoop-bucket`) to reference monorepo
- [ ] T121 [P] Update CI/CD workflows in distribution repos
- [ ] T122 [P] Update `raps-website` documentation to reflect monorepo structure

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - User Story 1 (P1) and User Story 2 (P1) can proceed in parallel after Foundational
  - User Story 3 (P2) and User Story 4 (P2) can proceed in parallel after US1/US2
- **Polish (Phase 7)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - Independent of US1
- **User Story 3 (P2)**: Can start after Foundational (Phase 2) - Independent of US1/US2
- **User Story 4 (P2)**: Can start after Foundational (Phase 2) - Independent of other stories

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Kernel types before service modules
- Service modules before CLI commands
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, User Stories 1 and 2 can start in parallel
- User Stories 3 and 4 can start in parallel after US1/US2
- All tests for a user story marked [P] can run in parallel
- Different user stories can be worked on in parallel by different team members
- All Polish tasks marked [P] can run in parallel

---

## Implementation Strategy

### MVP First (User Stories 1 & 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Atomic Changes)
4. Complete Phase 4: User Story 2 (Unified Workflow)
5. **STOP and VALIDATE**: Test that atomic changes work and unified workflow validates workspace
6. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo
4. Add User Story 3 → Test independently → Deploy/Demo
5. Add User Story 4 → Test independently → Deploy/Demo
6. Add Polish → Production ready
7. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Atomic Changes)
   - Developer B: User Story 2 (Unified Workflow)
3. Once US1/US2 complete:
   - Developer A: User Story 3 (Independent Testing)
   - Developer B: User Story 4 (Dependency Consistency)
4. Team completes Polish phase together
5. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
- Kernel must maintain <3000 LOC (excluding tests)
- Kernel must compile with `deny(unsafe_code)`, `deny(warnings)`, `deny(clippy::unwrap_used)`
- All workspace dependencies must use `{ workspace = true }` syntax
- Performance targets: kernel check <5s, workspace check <30s (incremental)


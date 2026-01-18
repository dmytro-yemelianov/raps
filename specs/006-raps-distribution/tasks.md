# Tasks: Multi-Channel RAPS Distribution

**Input**: Design documents from `/specs/006-raps-distribution/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Scope**: Phase 1 only (Install scripts + PyPI CLI + Release automation)

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure for distribution tooling

- [X] T001 Create `python/` directory structure for maturin packaging
- [X] T002 [P] Create `.github/workflows/release.yml` skeleton with trigger configuration
- [X] T003 [P] Create `.github/workflows/test-install.yml` skeleton for script testing

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**CRITICAL**: Binary build matrix must be working before install scripts or PyPI packaging can be tested

- [X] T004 Implement binary build job in `.github/workflows/release.yml` with 6-platform matrix (linux-x64, linux-arm64, darwin-x64, darwin-arm64, windows-x64, windows-arm64)
- [X] T005 Add cross-compilation setup for arm64 targets (linux: cross, windows: cargo-xwin)
- [X] T006 [P] Generate checksums.txt with SHA256 hashes in release workflow
- [X] T007 [P] Add artifact upload/download between jobs in release workflow

**Checkpoint**: Binary builds working for all 6 platforms - user story implementation can now begin

---

## Phase 3: User Story 1 - Quick Install via Shell Script (Priority: P1)

**Goal**: One-liner bash install for Linux/macOS users

**Independent Test**: `curl -fsSL .../install.sh | bash && raps --version` works on fresh system

### Implementation for User Story 1

- [X] T008 [US1] Create `install.sh` with ASCII banner and basic structure (FR-001 platform detection)
- [X] T009 [US1] Implement version resolution in `install.sh` - fetch latest from GitHub API or use RAPS_VERSION env var (FR-006)
- [X] T010 [US1] Implement binary download logic in `install.sh` with progress indicator (FR-002)
- [X] T011 [US1] Implement SHA256 checksum verification in `install.sh` using sha256sum/shasum (FR-002)
- [X] T012 [US1] Implement install directory creation and binary extraction in `install.sh` (FR-003)
- [X] T013 [US1] Implement shell detection (bash, zsh, fish) and PATH modification in `install.sh` (FR-004)
- [X] T014 [US1] Implement installation verification - run `raps --version` (FR-005)
- [X] T015 [US1] Implement `--uninstall` flag handling in `install.sh` (FR-007)
- [X] T016 [US1] Implement `--help` flag and error handling in `install.sh`
- [X] T017 [US1] Add bash install script test to `.github/workflows/test-install.yml` (ubuntu-latest, macos-latest)

**Checkpoint**: Bash install script fully functional on Linux and macOS

---

## Phase 4: User Story 2 - Quick Install via PowerShell Script (Priority: P1)

**Goal**: One-liner PowerShell install for Windows users

**Independent Test**: `irm .../install.ps1 | iex; raps --version` works on fresh Windows system

### Implementation for User Story 2

- [X] T018 [US2] Create `install.ps1` with ASCII banner and parameter definitions (FR-008, FR-012)
- [X] T019 [US2] Implement architecture detection (AMD64, ARM64) in `install.ps1` (FR-008)
- [X] T020 [US2] Implement version resolution in `install.ps1` - fetch latest from GitHub API or use $env:RAPS_VERSION
- [X] T021 [US2] Implement binary download with Invoke-WebRequest and progress in `install.ps1` (FR-009)
- [X] T022 [US2] Implement SHA256 checksum verification using Get-FileHash in `install.ps1`
- [X] T023 [US2] Implement install directory creation and zip extraction in `install.ps1` (FR-010)
- [X] T024 [US2] Implement User PATH modification via [Environment]::SetEnvironmentVariable (FR-011)
- [X] T025 [US2] Implement -NoPathUpdate switch handling in `install.ps1` (FR-012)
- [X] T026 [US2] Implement -Uninstall switch handling in `install.ps1` (FR-012)
- [X] T027 [US2] Implement installation verification and -Help switch in `install.ps1` (FR-013)
- [X] T028 [US2] Add PowerShell install script test to `.github/workflows/test-install.yml` (windows-latest)

**Checkpoint**: PowerShell install script fully functional on Windows

---

## Phase 5: User Story 3 - Install via pip (Priority: P2)

**Goal**: `pip install raps` installs the CLI and makes it available in PATH

**Independent Test**: `pip install raps && raps --version` in clean virtual environment

### Implementation for User Story 3

- [X] T029 [US3] Create `python/pyproject.toml` with maturin configuration (bindings = "bin") (FR-014, FR-016)
- [X] T030 [US3] Create `python/src/raps/__init__.py` with package namespace
- [X] T031 [US3] Create `python/src/raps/__main__.py` with binary wrapper entry point (FR-015)
- [X] T032 [US3] Create `python/README.md` for PyPI package page
- [X] T033 [US3] Add maturin wheel build job to `.github/workflows/release.yml` for all 6 platforms (FR-017)
- [X] T034 [US3] Configure maturin to extract version from Cargo.toml (FR-018)
- [X] T035 [US3] Add PyPI publish job using OIDC trusted publishing (FR-034)
- [ ] T036 [US3] Test local maturin build and install with `maturin develop`

**Checkpoint**: PyPI package builds and installs correctly on all platforms

---

## Phase 6: User Story 6 - Automated Release Distribution (Priority: P1)

**Goal**: Tag push triggers automated build and publish to all Phase 1 channels

**Independent Test**: Push test tag, verify GitHub release and PyPI package appear within 30 minutes

### Implementation for User Story 6

- [X] T037 [US6] Configure release workflow trigger for `v*.*.*` tags (FR-032)
- [X] T038 [US6] Add GitHub Release creation job using softprops/action-gh-release (FR-033)
- [X] T039 [US6] Upload all binary archives and checksums.txt to GitHub Release
- [X] T040 [US6] Add job dependencies: build-binaries → create-release → build-wheels → publish-pypi
- [X] T041 [US6] Add test-install-scripts job that depends on create-release (FR-036)
- [X] T042 [US6] Configure workflow permissions (contents: write, id-token: write) (FR-034)
- [X] T043 [US6] Add timeout configurations for each job (per contracts/github-actions-workflow.md)
- [X] T044 [US6] Add error handling and retry logic for PyPI publish

**Checkpoint**: Full release automation working - tag push publishes to GitHub and PyPI

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [X] T045 [P] Update website docs with new installation methods at `raps-website/src/content/docs/`
- [X] T046 [P] Update main README.md with new installation commands
- [X] T047 [P] Update CHANGELOG.md with distribution feature
- [ ] T048 Run quickstart.md validation - test all installation methods manually (requires post-merge testing)
- [X] T049 [P] Add troubleshooting section to website docs (permission denied, PATH issues, etc.)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - US1 (Bash) and US2 (PowerShell) can proceed in parallel
  - US3 (PyPI) can proceed in parallel with US1/US2
  - US6 (Release automation) depends on T033-T035 from US3
- **Polish (Phase 7)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 3 (P2)**: Can start after Foundational (Phase 2) - Independent of install scripts
- **User Story 6 (P1)**: Depends on T033 (maturin build job) to be complete for PyPI publishing

### Within Each User Story

- Core functionality before edge cases
- Implementation before verification/tests
- Local testing before CI integration
- Story complete before moving to next priority

### Parallel Opportunities

- T002, T003 can run in parallel (different workflow files)
- T006, T007 can run in parallel (independent workflow steps)
- US1, US2, US3 can all start in parallel after Phase 2
- T029-T032 can run in parallel (different files in python/ directory)
- T045, T046, T047, T049 can run in parallel (different documentation files)

---

## Implementation Strategy

### MVP First (Install Scripts Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (binary builds)
3. Complete Phase 3: User Story 1 (Bash script)
4. Complete Phase 4: User Story 2 (PowerShell script)
5. **STOP and VALIDATE**: Test both scripts on fresh systems
6. Tag and release manually to validate

### Full Phase 1 Delivery

1. Complete Setup + Foundational → Binary builds working
2. Add User Story 1 (Bash) → Test independently
3. Add User Story 2 (PowerShell) → Test independently
4. Add User Story 3 (PyPI) → Test with maturin develop
5. Add User Story 6 (Release automation) → Push test tag to validate
6. Polish and documentation updates

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Phase 2 user stories (US4, US5) are out of scope for this implementation
- Verify each install method works on fresh systems before marking complete
- Test with both latest version and specific version parameters
- Commit after each task or logical group

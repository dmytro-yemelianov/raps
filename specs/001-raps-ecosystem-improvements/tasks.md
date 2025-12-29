# Tasks: RAPS Ecosystem Improvements

**Input**: Design documents from `/specs/001-raps-ecosystem-improvements/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1-US9)
- Include exact file paths in descriptions

---

## Phase 0: Microkernel Foundation (Week 1-2) 🏗️

**Purpose**: Extract minimal trusted kernel from monolith - BLOCKS ALL OTHER PHASES

**⚠️ CRITICAL**: This is the architectural foundation. All subsequent phases depend on this.

### Workspace Setup

- [x] T001 [US8] Create Cargo workspace root with `resolver = "2"` ✅
- [x] T002 [P] [US8] Create crate directory structure: ✅
  - `raps-kernel/` - Minimal trusted core
  - `raps-oss/` - OSS service
  - `raps-derivative/` - Model Derivative service
  - `raps-dm/` - Data Management service
  - `raps-community/` - Community tier features
  - `raps-pro/` - Enterprise features (stub)
- [x] T003 [P] [US8] Configure workspace dependencies in root `Cargo.toml` ✅
- [x] T004 [P] [US8] Create/update `.cargo/config.toml`: ✅
  - Configure `lld-link` for Windows targets (`x86_64-pc-windows-msvc`)
  - Configure `mold` linker for Linux targets (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`)

### Build Performance Infrastructure (NEW)

- [x] T004a [P] [US8] Update workspace `Cargo.toml` with `debug = 0` for dev/test profiles ✅
- [x] T004b [P] [US8] Create GitHub Actions CI workflow with sccache: ✅
  - Install sccache via `mozilla-actions/sccache-action`
  - Set `RUSTC_WRAPPER=sccache` env var
  - Set `SCCACHE_GHA_ENABLED=true` for GitHub cache backend
- [x] T004c [P] [US8] Create GitHub Actions CI workflow with cargo-nextest: ✅
  - Install via `taiki-e/install-action@nextest`
  - Replace `cargo test` with `cargo nextest run`
- [x] T004d [P] [US8] Create GitHub Actions CI workflow with build timing: ✅
  - Run `cargo build --timings --release`
  - Upload `target/cargo-timings/cargo-timing-*.html` as artifact
- [x] T004e [P] [US8] Install mold linker in Linux CI jobs: ✅
  - `sudo apt-get install -y mold`
- [x] T004f [US8] Document local tooling setup in README.md: ✅ (DEVELOPMENT.md)
  - Windows: `winget install LLVM.LLVM`, `cargo install sccache`, `setx RUSTC_WRAPPER sccache`
  - Linux: `sudo apt install mold`, `cargo install sccache`, `export RUSTC_WRAPPER=sccache`
  - All platforms: `cargo install cargo-nextest`
- [ ] T004g [US8] Verify build performance targets:
  - `cargo check -p raps-kernel` < 5s (incremental)
  - `cargo check` (full workspace) < 30s (incremental)
  - sccache hit rate > 80% in CI

### Kernel Extraction

- [x] T005 [US8] Create `raps-kernel/src/lib.rs` with public API surface ✅
- [x] T006 [US8] Extract `error.rs` → `raps-kernel/src/error.rs`: ✅
  - Define `RapsError` enum with exit codes
  - Define `Result<T>` type alias
  - Implement `From` conversions
- [~] T007 [P] [US8] Extract HTTP client → `raps-kernel/src/http/`: ⚠️ PARTIAL
  - `client.rs` - Base HTTP client with config ✅
  - `retry.rs` - Exponential backoff with jitter ✅
  - `middleware.rs` - Request/response middleware ❌ MISSING
- [x] T008 [P] [US8] Extract config → `raps-kernel/src/config/`: ✅
  - `endpoints.rs` - APS API endpoint URLs
  - `profiles.rs` - Profile management
  - `mod.rs` - Config loading
- [x] T009 [P] [US8] Extract storage → `raps-kernel/src/storage/`: ✅
  - `keyring.rs` - OS keyring backend
  - `file.rs` - File-based fallback
  - `mod.rs` - Storage abstraction
- [~] T010 [P] [US8] Extract auth → `raps-kernel/src/auth/`: ⚠️ PARTIAL (flows in client.rs)
  - `token.rs` - AccessToken type (in storage/token.rs) ✅
  - `two_legged.rs` - Client credentials flow (in client.rs) ✅
  - `three_legged.rs` - Authorization code flow ❌ Not separate file
  - `device_code.rs` - Device code flow ❌ Not separate file
- [x] T011 [P] [US8] Extract types → `raps-kernel/src/types/`: ✅
  - `urn.rs` - URN newtype with validation
  - `bucket.rs` - BucketKey newtype
  - `object.rs` - ObjectKey newtype
- [x] T012 [US8] Extract logging → `raps-kernel/src/logging.rs`: ✅
  - Tracing integration
  - Secret redaction patterns

### Kernel Tests (100% coverage target)

- [x] T013 [P] [US8] Write tests for `raps-kernel/src/error.rs`: ✅ (4 tests)
  - `test_exit_codes_match_spec`
  - `test_error_display_format`
- [x] T014 [P] [US8] Write tests for `raps-kernel/src/http/`: ✅ (7 tests)
  - `test_retry_on_429_with_backoff`
  - `test_no_retry_on_4xx`
  - `test_timeout_respected`
  - `test_jitter_applied`
- [x] T015 [P] [US8] Write tests for `raps-kernel/src/auth/`: ✅ (6 tests)
  - `test_two_legged_returns_valid_token`
  - `test_token_refresh_before_expiry`
  - `test_auth_error_exit_code_3`
- [x] T016 [P] [US8] Write tests for `raps-kernel/src/config/`: ✅ (10 tests)
  - `test_endpoints_from_env`
  - `test_profile_loading`
- [x] T017 [US8] Verify kernel compiles with lint denies: ✅
  ```bash
  cargo clippy -p raps-kernel -- -D warnings -D clippy::unwrap_used
  ```
- [x] T018 [US8] Measure kernel LOC (target: <3000): ✅ **1,873 LOC**
  ```bash
  tokei raps-kernel/src --exclude tests
  ```

**Checkpoint**: ✅ Kernel crate compiles, 67 tests, LOC 1,873 (<3000), no unsafe code

---

## Phase 1: Service Extraction (Week 3-4)

**Purpose**: Extract API services from monolith into dedicated crates

### OSS Service (`raps-oss`)

- [x] T019 [US1] Create `raps-oss/Cargo.toml` depending on `raps-kernel` ✅
- [x] T020 [US1] Implement `OssClient` in `raps-oss/src/lib.rs` ✅
- [x] T021 [US1] Extract bucket operations → `raps-oss/src/bucket.rs`: ✅
  - `list_buckets`
  - `create_bucket`
  - `get_bucket`
  - `delete_bucket`
- [x] T022 [US1] Extract object operations → `raps-oss/src/object.rs`: ✅
  - `list_objects`
  - `get_object_details`
  - `delete_object`
- [x] T023 [US1] Implement parallel upload → `raps-oss/src/upload.rs`: ✅
  - `upload_simple` - Files <5MB
  - `upload_multipart` - Files ≥5MB, sequential
  - `upload_parallel` - Files ≥5MB, concurrent chunks
- [x] T024 [US1] Implement download → `raps-oss/src/download.rs` ✅
- [x] T025 [US1] Implement signed URLs → `raps-oss/src/signed_url.rs` ✅

### OSS Tests

- [ ] T026 [P] [US1] Write parallel upload tests:
  - `test_parallel_upload_respects_concurrency`
  - `test_parallel_upload_completes_all_chunks`
  - `test_parallel_upload_handles_partial_failure`
  - `test_upload_resume_after_interruption`
- [ ] T027 [P] [US1] Create upload benchmarks in `raps-oss/benches/`

### Model Derivative Service (`raps-derivative`)

- [x] T028 [P] [US2] Create `raps-derivative/Cargo.toml` ✅
- [x] T029 [P] [US2] Implement `DerivativeClient`: ✅
  - `translate` (translate.rs)
  - `status` (translate.rs)
  - `manifest` (manifest.rs)
  - `download_derivative` (download.rs)

### Data Management Service (`raps-dm`)

- [x] T030 [P] [US2] Create `raps-dm/Cargo.toml` ✅
- [x] T031 [P] [US2] Implement `DataManagementClient`: ✅
  - `list_hubs` (hub.rs)
  - `list_projects` (project.rs)
  - `list_folders` (folder.rs)
  - `list_items` (item.rs)
  - `get_item_versions` (item.rs)

**Checkpoint**: ✅ All core services extracted, workspace compiles, CLI uses service crates

---

## Phase 2: Community Tier (Week 5-6)

**Purpose**: Package community features into dedicated crate with feature flag

### Community Crate Setup

- [x] T032 [US9] Create `raps-community/Cargo.toml` with dependencies on core services ✅
- [x] T033 [US9] Create module structure in `raps-community/src/`: ✅
  - `acc/` - ACC modules
  - `da/` - Design Automation
  - `reality/` - Reality Capture
  - `webhooks/` - Webhook management
  - `pipeline/` - Pipeline automation
  - `plugin/` - Plugin system

### ACC Modules Extraction

- [x] T034 [P] [US4] Extract Issues → `raps-community/src/acc/issues.rs` ✅
- [x] T035 [P] [US4] Extract RFIs → `raps-community/src/acc/rfi.rs` ✅
- [x] T036 [P] [US4] Extract Assets → `raps-community/src/acc/assets.rs` ✅
- [x] T037 [P] [US4] Extract Submittals → `raps-community/src/acc/submittals.rs` ✅
- [x] T038 [P] [US4] Extract Checklists → `raps-community/src/acc/checklists.rs` ✅

### Other Community Features

- [x] T039 [P] [US4] Extract Design Automation → `raps-community/src/da/` ✅
- [x] T040 [P] [US4] Extract Reality Capture → `raps-community/src/reality/` ✅
- [x] T041 [P] [US4] Extract Webhooks → `raps-community/src/webhooks/` ✅
- [x] T042 [US4] Extract Pipelines → `raps-community/src/pipeline/` ✅
- [x] T043 [US4] Extract Plugins → `raps-community/src/plugin/` ✅

### MCP Server Integration

- [ ] T044 [US4] Update MCP server to use service crates
- [ ] T045 [US4] Add conditional tool registration based on tier:
  ```rust
  #[cfg(feature = "community")]
  tools.register(IssueTools::new());
  ```
- [ ] T046 [US4] Write MCP parity tests

**Checkpoint**: ✅ Community tier structure complete; MCP integration pending (T044-T046)

---

## Phase 3: CLI Refactoring (Week 7-8)

**Purpose**: Update CLI to thin shell dispatching to service crates

### CLI Updates

- [x] T047 [US9] Update `raps/Cargo.toml` with feature flags: ✅
  ```toml
  [features]
  default = ["community"]
  core = ["raps-kernel", "raps-oss", "raps-derivative", "raps-dm"]
  community = ["core", "raps-community"]
  pro = ["community", "raps-pro"]
  ```
- [ ] T048 [US9] Refactor command handlers to dispatch to services
- [ ] T049 [US9] Implement tier-gated command handling:
  ```rust
  #[cfg(not(feature = "community"))]
  pub fn handle_issue_list() -> Result<()> {
      Err(RapsError::TierRequired { 
          feature: "ACC Issues",
          required_tier: "Community" 
      })
  }
  ```
- [ ] T050 [US9] Update `--version` to show tier name
- [ ] T051 [US3] Audit all commands for non-interactive mode
- [ ] T052 [US3] Wrap all `dialoguer` calls with `spawn_blocking`
- [ ] T053 [US3] Wrap OAuth callback with `spawn_blocking`

### Output Schema Formalization

- [ ] T054 [P] [US2] Define output types with `JsonSchema` derive
- [ ] T055 [P] [US2] Implement `raps schema <command>` subcommand
- [ ] T056 [US2] Generate JSON Schema documentation
- [ ] T057 [US2] Write schema validation tests

**Checkpoint**: CLI builds at all tiers, non-interactive mode works

---

## Phase 4: Cross-Interface Consistency (Week 9-10)

**Purpose**: Ensure CLI, MCP, GitHub Action, Docker provide identical experiences

### MCP Expansion

- [ ] T058 [US4] Add missing MCP tools to match CLI:
  - [ ] T058a `object_upload`
  - [ ] T058b `translate_download`
  - [ ] T058c `folder_list`
  - [ ] T058d `issue_list` (community tier)
  - [ ] T058e `webhook_list` (community tier)
- [ ] T059 [US4] Add `tool_call_id` to all responses
- [ ] T060 [US4] Implement request queuing with backpressure
- [ ] T061 [US4] Write MCP stress tests:
  - `test_mcp_handles_100_requests_per_minute`
  - `test_mcp_memory_stable_under_load`

### GitHub Action Updates

- [ ] T062 [P] [US6] Add Windows PowerShell install script
- [ ] T063 [US6] Update `action.yml` with Windows step
- [ ] T064 [US6] Add binary caching with `actions/cache@v4`
- [ ] T065 [US6] Expose structured JSON outputs
- [ ] T066 [US6] Add version validation

### Docker Updates

- [ ] T067 [P] [US5] Update `raps-docker/Dockerfile` to v3.2.0
- [ ] T068 [P] [US5] Add `HEALTHCHECK` instruction
- [ ] T069 [US5] Verify multi-arch build (amd64/arm64)
- [ ] T070 [US5] Add `RAPS_NO_KEYCHAIN=1` default

**Checkpoint**: All interfaces consistent, GitHub Action works on Windows

---

## Phase 5: TUI & Shared Experience (Week 11-12)

**Purpose**: Complete TUI using extracted service crates

### TUI Updates

- [ ] T071 [US7] Update `aps-tui/Cargo.toml` to use service crates:
  ```toml
  [dependencies]
  raps-kernel = { path = "../raps-kernel" }
  raps-oss = { path = "../raps-oss" }
  raps-derivative = { path = "../raps-derivative" }
  ```
- [ ] T072 [US7] Remove duplicated API code from `aps-tui/src/api/`
- [ ] T073 [US7] Implement Buckets panel in `aps-tui/src/ui/buckets.rs`
- [ ] T074 [US7] Implement Objects panel in `aps-tui/src/ui/objects.rs`
- [ ] T075 [US7] Implement Translations panel
- [ ] T076 [US7] Implement Auth status header
- [ ] T077 [US7] Add vim-style keybindings (j/k, /)
- [ ] T078 [US7] Implement upload with progress bar

**Checkpoint**: TUI launches, shows auth, can browse buckets/objects

---

## Phase 6: Pro Tier Foundation (Week 13-14)

**Purpose**: Establish Pro tier infrastructure (proprietary)

### Pro Crate Setup

- [x] T079 [US9] Create `raps-pro/Cargo.toml` (consider separate private repo) ✅
- [x] T080 [US9] Define Pro module structure: ✅
  - `analytics/` - Usage analytics
  - `audit/` - Audit logging
  - `compliance/` - SOC2, GDPR reporting
  - `multitenant/` - Tenant management
  - `sso/` - SSO/SAML integration
  - `license/` - License validation (pending)

### Pro Feature Stubs

- [ ] T081 [P] [US9] Implement `analytics` stub with telemetry hooks
- [ ] T082 [P] [US9] Implement `audit` stub with log appender
- [ ] T083 [US9] Implement `license` validation:
  - License key format
  - Server validation (stub)
  - Feature gating

**Checkpoint**: Pro tier builds, license validation works (stub)

---

## Phase 7: Documentation & Polish (Week 15-16)

**Purpose**: Update documentation and distribution

### Documentation Updates

- [ ] T084 [P] Update `raps/README.md` with architecture overview
- [ ] T085 [P] Update `raps/CHANGELOG.md` with v3.2.0 changes
- [ ] T086 [P] Create architecture diagram in `raps-website/`
- [ ] T087 [P] Document tier feature matrix
- [ ] T088 [P] Generate API reference for each crate

### Distribution Updates

- [ ] T089 [P] Update `homebrew-tap/Formula/raps.rb`
- [ ] T090 [P] Update `scoop-bucket/bucket/raps.json`
- [ ] T091 [P] Create release workflow with tier variants
- [ ] T092 Run full benchmark suite

### Final Validation

- [ ] T093 Run `cargo clippy --workspace` and fix warnings
- [ ] T094 Run `cargo fmt --check --all`
- [ ] T095 Validate quickstart.md with fresh install
- [ ] T096 Security audit: verify secret redaction

**Checkpoint**: All documentation updated, distribution ready

---

## Dependencies & Execution Order

### Phase Dependencies

```
Phase 0 (Kernel) ──────────────────────────────────────────┐
    │ BLOCKS ALL                                            │
    ▼                                                       │
Phase 1 (Services) ─────────────────────────────────────────┤
    │                                                       │
    ▼                                                       │
Phase 2 (Community) ────────────────────────────────────────┤
    │                                                       │
    ├──────────────────┐                                    │
    ▼                  ▼                                    │
Phase 3 (CLI)     Phase 5 (TUI)                            │
    │                  │                                    │
    ▼                  │                                    │
Phase 4 (Interfaces)   │                                    │
    │                  │                                    │
    ├──────────────────┘                                    │
    ▼                                                       │
Phase 6 (Pro) ──────────────────────────────────────────────┤
    │                                                       │
    ▼                                                       │
Phase 7 (Polish) ───────────────────────────────────────────┘
```

### Parallel Opportunities

**Phase 0**: T002, T003, T004 parallel; T007-T012 parallel after T005-T006
**Phase 1**: T028-T031 parallel (independent services)
**Phase 2**: T034-T041 parallel (independent modules)
**Phase 4**: T062, T067, T068 parallel (different repos)
**Phase 5**: Can start after Phase 2 (parallel to Phase 3-4)
**Phase 7**: T084-T091 all parallel

---

## Estimated Effort

| Phase | Tasks | Completed | Status | Critical Path |
|-------|-------|-----------|--------|---------------|
| Phase 0: Kernel | 25 | 20 | ✅ 80% | ✅ DONE |
| Phase 1: Services | 13 | 11 | ✅ 85% | ✅ DONE |
| Phase 2: Community | 15 | 12 | ✅ 80% | ⚠️ MCP pending |
| Phase 3: CLI | 11 | 1 | ⏳ 9% | Not started |
| Phase 4: Interfaces | 13 | 0 | ⏳ 0% | Not started |
| Phase 5: TUI | 8 | 0 | ⏳ 0% | Not started |
| Phase 6: Pro | 5 | 2 | ⏳ 40% | Stubs only |
| Phase 7: Polish | 13 | 0 | ⏳ 0% | Not started |
| **Total** | **103** | **46** | **~45%** | |

---

## Success Metrics

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Kernel LOC | <3000 | 1,873 | ✅ Met |
| Kernel tests | 67+ | 67 | ✅ Met |
| Core build time | <30s | TBD | ⏳ Verify |
| Upload speed | 6x faster | TBD | ⏳ Verify |
| Startup time | <100ms | ~80ms | ✅ Met |
| MCP stability | <100MB | TBD | ⏳ Verify |
| Tier separation | 3 builds | 3 | ✅ Core/Community/Pro compile |
| Workspace compiles | No errors | ✅ | ✅ Met (warnings only) |

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps to user story for traceability
- Phase 0 is critical path - invest in solid foundation
- Pro tier may move to separate private repository
- Version after refactor: 3.2.0 (breaking internal changes)

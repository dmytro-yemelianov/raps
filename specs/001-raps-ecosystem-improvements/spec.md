# Feature Specification: RAPS Ecosystem Improvements

**Feature Branch**: `001-raps-ecosystem-improvements`  
**Created**: 2025-12-29  
**Updated**: 2025-12-29  
**Status**: In Progress (~45% Complete)  
**Input**: User description: "Build and improve the solution that is implementing command-line interface for the Autodesk Platform Services (APS). CLI, MCP, GitHub Actions, Docker container, etc. modes. Refactor to microkernel architecture with Core → Community → Pro tiers."

## Executive Summary

This specification covers comprehensive improvements to the RAPS (Rust Autodesk Platform Services) ecosystem—a multi-repository project providing CLI, MCP server, GitHub Actions, Docker container, and TUI interfaces for Autodesk Platform Services automation.

### Vision: Microkernel Architecture with Tiered Products

RAPS will evolve into a **Unix-like microkernel architecture** with three product tiers:

| Tier | Description | License |
|------|-------------|---------|
| **Core (Kernel)** | Minimal trusted foundation: Auth, OSS, Derivative, Data Management | Apache 2.0 |
| **Community** | Extended features: ACC, DA, Reality, Webhooks, Pipelines, Plugins, MCP, TUI | Apache 2.0 |
| **Pro (Enterprise)** | Advanced features: Analytics, Audit, Compliance, Multi-tenant, SSO | Commercial |

### Repository Taxonomy

| Category | Repositories |
|----------|--------------|
| **Primary App** | `raps` (Rust CLI + microkernel crates) |
| **Documentation** | `raps-website` (main website at rapscli.xyz) |
| **Distribution Satellites** | `homebrew-tap`, `scoop-bucket`, `raps-action`, `raps-docker` |
| **Ecosystem** | `aps-tui`, `aps-wasm-demo`, `aps-sdk-openapi` |

**Current State (v3.2.0)**:
- ✅ Mature CLI with 50+ commands covering Authentication, OSS, Model Derivative, Data Management, Webhooks, Design Automation, ACC Issues/RFIs/Assets/Submittals/Checklists, Reality Capture, Pipelines, Plugins
- ✅ MCP server with 14 tools for AI assistant integration
- ✅ GitHub Action for CI/CD workflows
- ✅ Docker container for containerized deployments
- ✅ **Microkernel architecture implemented** - 6 service crates extracted
- ✅ **Tiered product strategy** - Core/Community/Pro feature flags working
- ✅ **Build performance infrastructure** - sccache, nextest, mold/lld-link in CI
- ⚠️ TUI (Terminal UI) in early development
- ⚠️ Some kernel modules need refinement (middleware.rs, auth flow separation)

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - High-Performance File Operations (Priority: P1)

As a DevOps engineer automating CAD file workflows, I need to upload large files (100MB+) quickly so that my CI/CD pipelines complete within acceptable time limits.

**Why this priority**: File operations are the most common use case and directly impact user productivity. Performance issues here affect every workflow.

**Independent Test**: Upload a 100MB test file with parallel chunks and measure throughput improvement vs baseline.

**Acceptance Scenarios**:

1. **Given** a 100MB file to upload, **When** I run `raps object upload --parallel bucket file.dwg`, **Then** upload completes 3-5x faster than sequential upload using parallel chunk uploads.
2. **Given** a multipart upload in progress, **When** the network fails mid-transfer, **Then** I can resume with `--resume` and only incomplete chunks are re-uploaded without restarting.
3. **Given** the `--concurrency 10` flag, **When** performing batch operations, **Then** exactly 10 operations run in parallel with proper semaphore control.

---

### User Story 2 - Consistent Cross-Interface Experience (Priority: P1)

As a developer using RAPS across CLI, MCP, and GitHub Actions, I need identical behavior and output formats so that my automation scripts work regardless of interface.

**Why this priority**: Inconsistency between interfaces creates confusion and doubles maintenance burden for automation scripts.

**Independent Test**: Execute the same logical operation (e.g., list buckets) via CLI, MCP tool, and GitHub Action; verify identical JSON output schema.

**Acceptance Scenarios**:

1. **Given** `raps bucket list --output json` via CLI, **When** I call `bucket_list` via MCP, **Then** the JSON output schema is identical.
2. **Given** error code 3 (auth failure) from CLI, **When** the same error occurs in GitHub Action, **Then** `exit-code` output is 3 with matching error message format.
3. **Given** a destructive operation in MCP, **When** `--yes` equivalent is not provided, **Then** operation fails with confirmation requirement (matching CLI behavior).

---

### User Story 3 - Robust CI/CD Integration (Priority: P2)

As a CI/CD pipeline author, I need strict non-interactive mode and clear exit codes so that my workflows never hang waiting for input and failures are detectable.

**Why this priority**: CI/CD reliability is critical for enterprise adoption; hanging pipelines waste compute resources.

**Independent Test**: Run all commands with `--non-interactive` flag in GitHub Actions; verify none prompt for input.

**Acceptance Scenarios**:

1. **Given** `--non-interactive` mode, **When** a command requires missing input, **Then** it fails immediately with exit code 2 and clear error message.
2. **Given** a bucket creation in non-interactive mode, **When** `--key` is not provided, **Then** error says "Required argument --key missing in non-interactive mode".
3. **Given** rate limiting from APS API, **When** retry exhausts, **Then** exit code 5 with JSON output containing retry count and last error.

---

### User Story 4 - MCP Server Reliability (Priority: P2)

As an AI assistant (Claude/Cursor) user, I need the MCP server to handle rapid sequential requests without resource exhaustion so that long AI-assisted sessions remain responsive.

**Why this priority**: MCP is a differentiating feature; poor performance here undermines the AI integration value proposition.

**Independent Test**: Send 100 sequential MCP tool calls in 60 seconds; verify no memory growth or connection failures.

**Acceptance Scenarios**:

1. **Given** MCP server running for 1 hour with continuous requests, **When** I check memory usage, **Then** it remains under 100MB with no growth trend.
2. **Given** an MCP tool call with invalid parameters, **When** processed, **Then** error response follows JSON-RPC spec with actionable message.
3. **Given** concurrent MCP requests, **When** rate limits are hit, **Then** server queues requests with exponential backoff.

---

### User Story 5 - Docker Container for Air-Gapped Environments (Priority: P2)

As a security-conscious organization, I need a self-contained Docker image that works in restricted networks with all dependencies bundled.

**Why this priority**: Enterprise adoption often requires containerized deployments with security scanning.

**Independent Test**: Run Docker container in network-isolated environment; verify `--help` works without external downloads.

**Acceptance Scenarios**:

1. **Given** the Docker image, **When** I run `docker run dmytroyemelianov/raps bucket list`, **Then** it executes without downloading additional dependencies.
2. **Given** environment variables for credentials, **When** container starts, **Then** credentials are NOT written to any files inside the container.
3. **Given** multi-arch build (amd64/arm64), **When** running on Apple Silicon or AWS Graviton, **Then** native binary is used without emulation.

---

### User Story 6 - GitHub Action for Workflow Automation (Priority: P3)

As a GitHub workflow author, I need a reliable action that installs RAPS quickly and passes outputs correctly for downstream steps.

**Why this priority**: GitHub Actions integration enables the largest CI/CD platform ecosystem.

**Independent Test**: Create workflow that installs RAPS, runs command, and uses output in subsequent step.

**Acceptance Scenarios**:

1. **Given** `version: latest` input, **When** action runs, **Then** it installs the most recent release within 30 seconds.
2. **Given** a failed command, **When** workflow continues, **Then** `exit-code` output is available for conditional logic.
3. **Given** Windows runner, **When** action runs, **Then** Windows binary is downloaded and executed correctly.

---

### User Story 7 - TUI for Interactive Exploration (Priority: P3)

As a developer exploring APS resources, I need a visual terminal interface to browse hubs, projects, and folders without memorizing command syntax.

**Why this priority**: TUI lowers the barrier to entry for new users discovering APS capabilities.

**Independent Test**: Launch TUI, navigate to a bucket, upload a file, and view translation status—all with keyboard.

**Acceptance Scenarios**:

1. **Given** TUI launched with valid credentials, **When** I press Tab, **Then** I can cycle through panels (Buckets, Objects, Translations).
2. **Given** a long list of objects, **When** I type characters, **Then** list filters incrementally (fuzzy search).
3. **Given** an upload operation in TUI, **When** in progress, **Then** visual progress bar shows completion percentage.

---

### User Story 8 - Microkernel Architecture (Priority: P0) 🏗️ ARCHITECTURE

As a maintainer, I need the codebase refactored into a microkernel architecture so that the core is minimal, auditable, and highly testable while features can evolve independently.

**Why this priority**: Architecture foundation must be established before new features; enables security auditing and isolated testing.

**Independent Test**: Build with `--features core` produces minimal binary; kernel crate has 100% test coverage.

**Acceptance Scenarios**:

1. **Given** `cargo build --features core`, **When** compiled, **Then** binary includes only Auth, OSS, Derivative, and Data Management (no ACC, DA, etc.).
2. **Given** `raps-kernel` crate, **When** I run `cargo test -p raps-kernel`, **Then** all tests pass with >90% coverage.
3. **Given** a bug in `raps-community` ACC module, **When** it panics, **Then** kernel functionality remains unaffected.
4. **Given** `raps-kernel` source, **When** audited, **Then** LOC < 3000 with zero `unsafe` blocks.

---

### User Story 9 - Tiered Product Builds (Priority: P1)

As a product manager, I need to build different product tiers (Core, Community, Pro) from the same codebase so that we can offer free and commercial versions.

**Why this priority**: Enables sustainable business model while keeping core functionality open source.

**Independent Test**: Build all three tiers; verify feature availability matches specification.

**Acceptance Scenarios**:

1. **Given** `cargo build --features core`, **When** I run `raps acc issue list`, **Then** command fails with "Feature requires Community tier".
2. **Given** `cargo build --features community`, **When** I run `raps analytics dashboard`, **Then** command fails with "Feature requires Pro tier".
3. **Given** `cargo build --features pro`, **When** I run any command, **Then** all features are available.
4. **Given** Community build, **When** I check `raps --version`, **Then** output shows "RAPS Community v3.2.0".

---

### Edge Cases

- What happens when APS API is down? → Graceful degradation with cached data display and clear offline indicator.
- How does system handle token expiry mid-operation? → Auto-refresh if possible; prompt re-auth if refresh fails.
- What if user provides conflicting flags (`--json` and `--table`)? → Last flag wins with warning; document precedence.
- How does parallel upload handle partial failure? → Retry failed chunks; report partial success with list of failed chunks.
- What if Pro license expires? → Gracefully degrade to Community tier with warning; no data loss.
- How do tier-gated commands behave? → Clear error message indicating required tier; suggest upgrade path.

---

## Requirements *(mandatory)*

### Functional Requirements

#### Microkernel Architecture (raps-kernel)

- **FR-001**: System MUST be split into microkernel (`raps-kernel`) + service crates (`raps-oss`, `raps-derivative`, `raps-dm`)
- **FR-002**: Kernel MUST contain only: Auth, HTTP client, Config, Storage, Types, Error, Logging
- **FR-003**: Kernel MUST compile with `#![deny(warnings)]`, `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`
- **FR-004**: Kernel LOC MUST be <3000 lines (excluding tests)
- **FR-005**: Kernel test coverage MUST be >90% on critical paths
- **FR-006**: All service crates MUST depend only on `raps-kernel`, not on each other

#### Tiered Product Strategy

- **FR-007**: System MUST support feature flags: `core`, `community` (default), `pro`
- **FR-008**: Core tier MUST include: Auth, OSS, Derivative, Data Management, Config, Completions
- **FR-009**: Community tier MUST include Core + ACC, DA, Reality, Webhooks, Pipelines, Plugins, MCP, TUI
- **FR-010**: Pro tier MUST include Community + Analytics, Audit, Compliance, Multi-tenant, SSO
- **FR-011**: Tier-gated commands MUST fail gracefully with clear upgrade guidance
- **FR-012**: Version output MUST include tier name (e.g., "RAPS Community v3.2.0")

#### Build Performance & Tooling

- **FR-013**: Workspace MUST include `.cargo/config.toml` with `lld-link` linker for Windows targets
- **FR-014**: Workspace MUST include `.cargo/config.toml` with `mold` linker for Linux targets in CI
- **FR-015**: CI pipelines MUST use `sccache` for compilation caching to accelerate builds
- **FR-016**: Workspace Cargo.toml MUST set `debug = 0` for dev and test profiles to reduce PDB overhead
- **FR-017**: README MUST document local setup for lld-link/mold, sccache, and cargo-nextest (optional but recommended)
- **FR-017a**: CI pipelines MUST use `cargo-nextest` for parallel test execution instead of `cargo test`
- **FR-017b**: CI pipelines MUST run `cargo build --timings` and upload HTML report as artifact for build diagnostics

#### CLI Performance & Architecture

- **FR-018**: System MUST support parallel chunk uploads for files >5MB with configurable concurrency via `--concurrency` flag
- **FR-019**: System MUST implement buffer reuse for chunk uploads to reduce memory allocations
- **FR-020**: System MUST wrap all blocking calls (dialoguer, tiny_http) with `tokio::task::spawn_blocking`
- **FR-021**: System MUST use config-based URLs for all API endpoints (eliminate hardcoded URLs)
- **FR-022**: System MUST apply retry logic consistently across ALL API operations with exponential backoff
- **FR-023**: System MUST formalize JSON output schemas and maintain backward compatibility
- **FR-024**: System MUST support streaming pagination for large result sets with `--limit` flag
- **FR-025**: System MUST pass all commands through non-interactive mode audit

#### MCP Server (raps serve)

- **FR-009**: MCP server MUST cache APS client instances to reduce lock contention
- **FR-010**: MCP server MUST implement request queuing with backpressure for rate-limited scenarios
- **FR-011**: MCP server MUST expose all bucket/object operations currently available in CLI
- **FR-012**: MCP server MUST include `tool_call_id` in all responses for request tracing

#### GitHub Action (raps-action)

- **FR-013**: Action MUST support Windows runners (currently Linux/macOS only)
- **FR-014**: Action MUST cache RAPS binary across workflow runs using GitHub cache
- **FR-015**: Action MUST expose structured JSON output as output variable
- **FR-016**: Action MUST validate version input and fail fast on invalid version

#### Docker Container (raps-docker)

- **FR-017**: Container MUST be multi-arch (linux/amd64, linux/arm64)
- **FR-018**: Container MUST NOT persist credentials to filesystem
- **FR-019**: Container MUST include health check endpoint via `raps auth test`
- **FR-020**: Container MUST pin to specific RAPS version (currently hardcoded v2.0.0)

#### TUI (aps-tui)

- **FR-021**: TUI MUST share API client code with raps crate (extract to shared library)
- **FR-022**: TUI MUST support keyboard navigation for all operations
- **FR-023**: TUI MUST display authentication status in header bar
- **FR-024**: TUI MUST support vim-style keybindings (j/k navigation, / search)

### Key Entities

- **ApsClient**: Unified HTTP client with retry, timeout, and credential management
- **OutputSchema**: Formal JSON schema definitions for all command outputs
- **McpToolRegistry**: Dynamic tool registration for MCP server extensibility
- **UploadSession**: Resumable upload state with parallel chunk tracking
- **TuiState**: Centralized TUI application state with navigation stack

---

## Clarifications

### Session 2025-12-29

- Q: Should the spec include explicit build-time performance targets for the raps-kernel crate? → A: Yes, add `cargo check -p raps-kernel` < 5s, full workspace check < 30s
- Q: Should the spec require specific build tooling in `.cargo/config.toml` and CI? → A: Require lld-link + sccache in CI, document for local (balanced approach)
- Q: Should the spec include advanced cross-platform build optimizations beyond Windows lld-link? → A: Add mold linker for Linux CI builds (proven, significant speedup)
- Q: Should the spec require cargo-nextest for test execution in CI? → A: Require cargo-nextest in CI for parallel test execution
- Q: Should CI capture and report build timing metrics? → A: Capture `--timings` HTML report as CI artifact (visibility)

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Large file upload (100MB) completes in <30s on 100Mbps connection (currently ~2min)
- **SC-002**: Zero blocking async calls detected by `cargo clippy` with async lints enabled
- **SC-003**: 100% of commands pass non-interactive mode audit (no hanging in CI)
- **SC-004**: MCP server memory usage remains <100MB under sustained load (100 req/min for 1 hour)
- **SC-005**: GitHub Action installation time <30s (including download and extraction)
- **SC-006**: Docker image size <50MB (slim variant)
- **SC-007**: JSON output schemas documented and validated with JSON Schema Draft 7
- **SC-008**: TUI launches and displays auth status in <500ms
- **SC-009**: `cargo check -p raps-kernel` completes in <5s (incremental build)
- **SC-010**: `cargo check` for full workspace completes in <30s (incremental build)

### Performance Benchmarks (from Constitution)

| Metric | Target | Current | Gap |
|--------|--------|---------|-----|
| CLI startup (--help) | <100ms | ~80ms | ✅ Met |
| Auth test | <500ms | ~300ms | ✅ Met |
| 100MB upload (parallel) | <30s | ~120s | ❌ 4x gap |
| Bucket list (100 items) | <2s | ~1.5s | ✅ Met |
| MCP tool call | <200ms | ~150ms | ✅ Met |

---

## Technical Approach

### Phase 1: Performance & Architecture (Priority Issues)

1. **Parallel Multipart Upload** (Issue #70)
   - Implement `FuturesUnordered` for concurrent chunk uploads
   - Respect `--concurrency` flag for limit
   - Add progress tracking per chunk

2. **Blocking Async Fix** (Issue #73)
   - Audit all `dialoguer` and `tiny_http` usage
   - Wrap with `spawn_blocking`
   - Add clippy lint for blocking detection

3. **Unified Retry Logic** (Issue #77)
   - Extract retry logic to `http.rs` module
   - Apply to all API clients consistently
   - Configurable retry count and backoff

4. **Config-Based URLs** (Issue #76)
   - Define all APS endpoints in config
   - Support environment-based overrides (staging vs production)
   - Eliminate hardcoded URLs

### Phase 2: Cross-Interface Consistency

1. **Output Schema Formalization**
   - Define JSON Schema for each command output
   - Generate schema documentation
   - Add backward-compatibility tests

2. **MCP-CLI Parity**
   - Audit MCP tools against CLI commands
   - Add missing operations
   - Ensure error codes match

3. **GitHub Action Enhancements**
   - Add Windows runner support
   - Implement binary caching
   - Add structured JSON output

### Phase 3: TUI Development

1. **Shared API Library**
   - Extract `raps-core` crate for shared code
   - Move API clients to shared crate
   - Update both `raps` and `aps-tui` to use shared crate

2. **TUI Feature Completion**
   - Bucket/Object browsing panels
   - Upload/Download with progress
   - Translation status viewer

### Phase 4: Container & Distribution

1. **Docker Improvements**
   - Update to current RAPS version (3.1.0)
   - Add health check
   - Multi-arch manifest

2. **Distribution Channels**
   - Homebrew tap maintenance
   - Scoop bucket maintenance
   - Cargo installation verification

---

## Constitution Compliance

This specification aligns with the RAPS Ecosystem Constitution v1.0.0:

| Principle | Compliance Status |
|-----------|-------------------|
| I. Rust Idiomatic Code Quality | ✅ All changes follow clippy/rustfmt |
| II. Cross-Repository Consistency | ✅ Shared crate extraction planned |
| III. Test-First Development | ✅ Tests required before implementation |
| IV. User Experience Consistency | ✅ Output schema formalization |
| V. Performance & Resource Efficiency | ✅ Primary focus of Phase 1 |
| VI. Security & Secrets Handling | ✅ Container credential handling |

---

## Dependencies & Risks

### Dependencies

- `rmcp` crate stability for MCP server
- APS API availability for integration testing
- GitHub Actions runner availability for CI

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking changes to MCP protocol | Low | High | Pin rmcp version, test against spec |
| APS API deprecation | Low | High | Abstract API layer, version pin |
| Rust edition issues | Medium | Medium | MSRV policy, CI matrix testing |

---

## Out of Scope

- GUI desktop application (use TUI instead)
- Mobile applications
- Language bindings (Python, Node.js)—may be future consideration
- APS API proxy/gateway functionality

---

## References

- [RAPS README](../raps/README.md)
- [RAPS CHANGELOG](../raps/CHANGELOG.md)
- [Issues README](../raps/issues/README.md) - 15 tracked improvement issues
- [Constitution](../.specify/memory/constitution.md)
- [APS OpenAPI Specs](../aps-sdk-openapi/)

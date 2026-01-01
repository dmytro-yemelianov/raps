<!--
Sync Impact Report
==================
Version Change: 1.1.0 → 2.0.0 (MAJOR - Monorepo Consolidation)
Added Sections:
  - Monorepo Architecture (new section clarifying consolidated structure)
Modified Principles:
  - II. Cross-Repository Consistency: Updated repository taxonomy to reflect monorepo structure
    - raps now contains: kernel, all modules (oss, derivative, dm, ssa), community features, pro features, CLI
    - Separate repos: distribution, website, SMM, examples
  - VII. Microkernel Architecture: Updated to reflect kernel and modules in same repo (monorepo)
Removed Sections: None
Templates Requiring Updates:
  - .specify/templates/plan-template.md: ⚠ Pending (may need updates for monorepo structure)
  - .specify/templates/spec-template.md: ✅ Already aligned (testable requirements format)
  - .specify/templates/tasks-template.md: ✅ Already aligned (test-first approach documented)
Follow-up TODOs:
  - Update repository documentation to reflect monorepo structure
  - Update CI/CD workflows for consolidated repository
  - Migrate separate kernel/module repos into raps monorepo
-->

# RAPS Ecosystem Constitution

## Core Principles

### I. Rust Idiomatic Code Quality

All code across the RAPS ecosystem MUST adhere to Rust idioms and best practices:

- **Zero Warnings Policy**: Code MUST compile with `#![deny(warnings)]` and pass `cargo clippy` without warnings or suppressions (unless explicitly justified in code comments)
- **Error Handling**: Use `thiserror` for library errors, `anyhow` for application errors. NEVER use `.unwrap()` or `.expect()` in library code; application code MAY use `.expect()` only with descriptive messages
- **Type Safety**: Prefer newtype patterns over primitive types for domain concepts (e.g., `Urn(String)` not raw `String`)
- **Ownership Clarity**: Prefer borrowing over cloning. Document lifetime requirements when non-trivial
- **Documentation**: All public APIs MUST have rustdoc comments with examples. Modules MUST have `//!` documentation explaining purpose

**Rationale**: Rust's compiler catches bugs at compile time; leveraging this requires discipline in code style.

### II. Cross-Repository Consistency

The multi-repo ecosystem MUST maintain unified standards across all repositories:

**Repository Taxonomy:**
- **Primary Monorepo**: `raps` (contains: Rust CLI, microkernel, all service modules, community features, pro features)
  - Kernel: `raps-kernel` crate
  - Service Modules: `raps-oss`, `raps-derivative`, `raps-dm`, `raps-ssa` crates
  - Tier Features: `raps-community`, `raps-pro` crates
  - CLI Application: `raps` binary crate
- **Documentation**: `raps-website` (main website at rapscli.xyz)
- **Distribution**: `homebrew-tap`, `scoop-bucket`, `raps-action`, `raps-docker` (package managers and containers)
- **Social Media Marketing**: `raps-smm` (content library for promotion)
- **Examples**: `aps-demo-scripts`, `aps-tui`, `aps-wasm-demo` (example projects using RAPS kernel and modules)

**Consistency Requirements:**
- **Shared Versioning**: Core `raps` workspace version dictates compatibility; dependent projects MUST pin to compatible versions
- **Common Dependencies**: Workspace-level dependency management in `raps` monorepo; shared crates MUST use identical versions
- **Unified Error Codes**: Exit codes, error types, and user-facing messages MUST follow the canonical definitions in `raps/docs/cli/exit-codes.md`
- **API Contracts**: Changes to public APIs MUST be documented in CHANGELOG.md with migration guidance before release
- **Code Style**: All Rust code MUST use `rustfmt` with default settings; deviations require team consensus
- **Documentation Alignment**: `raps-website` MUST reflect current tier feature matrix and architecture decisions

**Rationale**: Users interact with the ecosystem as a unified product; inconsistency erodes trust and increases support burden. Monorepo structure enables atomic changes across kernel, modules, and features while maintaining clear separation of concerns.

### III. Test-First Development (NON-NEGOTIABLE)

All feature development MUST follow test-driven practices:

- **Red-Green-Refactor**: Tests MUST be written and fail before implementation begins
- **Test Categories**:
  - Unit tests: Colocated in `src/` modules via `#[cfg(test)]`
  - Integration tests: `tests/` directory for cross-module behavior
  - Contract tests: Validate external API interactions (APS endpoints)
- **Coverage Requirements**: New features MUST include tests for happy path + at least 2 error scenarios
- **CI Gate**: PRs MUST pass `cargo nextest run` (or `cargo test`) before merge
- **Test Naming**: Use descriptive names following pattern `test_<function>_<scenario>_<expected_outcome>`

**Rationale**: CLI tools require high reliability; untested code is unshippable code.

### IV. User Experience Consistency

All user-facing interfaces (CLI, TUI, MCP, web) MUST provide predictable, polished experiences:

- **Output Formats**: Support `--output {table,json,yaml,csv,plain}` for all data-returning commands
- **Progress Feedback**: Long-running operations (>1s) MUST show progress indicators
- **Error Messages**: Include actionable remediation steps; format: `Error: <what failed>\n  Cause: <why>\n  Fix: <how to resolve>`
- **Color & Accessibility**: Respect `--no-color` and `NO_COLOR` env var; never rely solely on color for meaning
- **Confirmation Prompts**: Destructive operations MUST prompt unless `--yes` flag provided
- **Exit Codes**: MUST use standardized codes (0=success, 2=args, 3=auth, 4=not found, 5=API error, 6=internal)

**Rationale**: Professionals rely on RAPS for automation; predictable behavior enables trust and scripting.

### V. Performance & Resource Efficiency

CLI tools MUST be fast and resource-conscious:

- **Startup Time**: Cold start MUST complete in <100ms for help/version commands
- **Memory Bounds**: Peak memory usage MUST not exceed 2x the size of data being processed
- **Network Efficiency**: Batch API calls where possible; respect rate limits with exponential backoff
- **Parallel Operations**: Use `--parallel` flag pattern for bulk operations; default to 5 concurrent operations
- **Build Speed**: Development feedback loop MUST meet these targets:
  - `cargo check -p raps-kernel` MUST complete in <5s for incremental changes
  - `cargo check` (full workspace) MUST complete in <30s for incremental changes
  - Use lld-link on Windows, mold on Linux CI; sccache for compilation caching
- **Binary Size**: Release builds SHOULD stay under 15MB; audit dependencies if exceeded
- **Upload Performance**: Large file uploads (100MB) MUST complete in <30s on 100Mbps connection using parallel chunk uploads

**Rationale**: Developer tools compete on speed; slow tools interrupt flow and reduce adoption. Fast build times enable rapid iteration.

### VI. Security & Secrets Handling

All credentials and sensitive data MUST be protected:

- **Secret Storage**: Use platform keyring (via `keyring` crate) for tokens; NEVER persist secrets in plain files
- **Secret Redaction**: All logging and debug output MUST redact tokens, keys, and credentials using regex patterns
- **Environment Variables**: Support credential injection via env vars for CI/CD; document required vars
- **HTTPS Only**: All APS API communication MUST use TLS; reject non-HTTPS endpoints
- **Minimal Permissions**: Request only required OAuth scopes; document scope requirements per command

**Rationale**: RAPS handles authentication to production APS resources; security failures have real-world impact.

### VII. Microkernel Architecture (NON-NEGOTIABLE)

RAPS MUST follow a Unix-like microkernel architecture with strict separation of concerns:

- **Monorepo Structure**: Kernel, all service modules, and tier features MUST reside in the `raps` monorepo as separate crates within a single workspace
- **Kernel Isolation**: `raps-kernel` crate MUST contain only: Auth, HTTP client, Config, Storage, Types, Error, Logging
- **Kernel Constraints**: Kernel MUST compile with `#![deny(warnings)]`, `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`
- **Kernel Size**: Kernel LOC MUST be <3000 lines (excluding tests); target ~2000 LOC
- **Service Independence**: Service crates (`raps-oss`, `raps-derivative`, `raps-dm`, `raps-ssa`) MUST depend only on `raps-kernel`, not on each other
- **Tier Separation**: `raps-community` and `raps-pro` crates MUST depend only on kernel and service crates, not on each other
- **Failure Isolation**: Bugs in service crates or tier features MUST NOT crash kernel functionality
- **Test Coverage**: Kernel test coverage MUST be >90% on critical paths
- **No Blocking**: All I/O MUST be async; blocking operations MUST use `tokio::task::spawn_blocking`
- **Workspace Organization**: All crates MUST be organized in `raps/` workspace root with clear crate boundaries

**Rationale**: Microkernel architecture enables security auditing, isolated testing, and independent feature evolution. The kernel is the trusted foundation that all features build upon. Monorepo structure enables atomic changes and shared tooling while maintaining architectural boundaries.

## Monorepo Architecture

### Repository Structure

RAPS follows a **monorepo architecture** where the primary `raps` repository contains all core components:

```
raps/ (monorepo)
├── raps-kernel/          # Microkernel foundation crate
├── raps-oss/              # Object Storage Service crate
├── raps-derivative/       # Model Derivative Service crate
├── raps-dm/               # Data Management Service crate
├── raps-ssa/              # Secure Service Accounts crate
├── raps-community/        # Community tier features crate
├── raps-pro/              # Pro tier features crate
└── raps/                  # CLI binary crate (depends on all above)
```

**Separate Repositories:**
- **Distribution**: `homebrew-tap`, `scoop-bucket`, `raps-action`, `raps-docker` (package distribution)
- **Documentation**: `raps-website` (website and docs)
- **Social Media Marketing**: `raps-smm` (content library)
- **Examples**: `aps-demo-scripts`, `aps-tui`, `aps-wasm-demo` (example projects using RAPS)

**Rationale**: Monorepo structure enables atomic changes across kernel, modules, and features while maintaining clear architectural boundaries. Separate repos for distribution, documentation, marketing, and examples allow independent evolution and deployment cycles.

## Tiered Product Strategy

### Architecture Overview

RAPS follows a **three-tier product strategy** with microkernel foundation:

```
┌─────────────────────────────────────────────────────────────┐
│                    RAPS Pro (Enterprise)                    │
│  Analytics, Audit, Compliance, Multi-tenant, SSO          │
│  License: Commercial                                        │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                  RAPS Community (Free)                       │
│  Account Admin, ACC, DA, Reality, Webhooks, MCP, TUI      │
│  License: Apache 2.0                                        │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                    RAPS Core (Kernel)                       │
│  Auth, SSA, OSS, Derivative, Data Management               │
│  License: Apache 2.0                                        │
└─────────────────────────────────────────────────────────────┘
```

### Tier Feature Matrix

| Feature | Core | Community | Pro |
|---------|:----:|:---------:|:---:|
| **Authentication** | ✅ | ✅ | ✅ |
| 2-Legged OAuth | ✅ | ✅ | ✅ |
| 3-Legged OAuth | ✅ | ✅ | ✅ |
| Device Code Flow | ✅ | ✅ | ✅ |
| SSA (Secure Service Accounts) | ✅ | ✅ | ✅ |
| SSO/SAML | ❌ | ❌ | ✅ |
| **Object Storage (OSS)** | ✅ | ✅ | ✅ |
| Bucket CRUD | ✅ | ✅ | ✅ |
| Object CRUD | ✅ | ✅ | ✅ |
| Parallel Uploads | ✅ | ✅ | ✅ |
| Signed URLs (S3) | ✅ | ✅ | ✅ |
| **Model Derivative** | ✅ | ✅ | ✅ |
| Translation Jobs | ✅ | ✅ | ✅ |
| Status & Manifest | ✅ | ✅ | ✅ |
| Derivative Download | ✅ | ✅ | ✅ |
| **Data Management** | ✅ | ✅ | ✅ |
| Hubs & Projects | ✅ | ✅ | ✅ |
| Folders & Items | ✅ | ✅ | ✅ |
| Tip Version & Derivatives | ✅ | ✅ | ✅ |
| **Account Admin** | ❌ | ✅ | ✅ |
| Projects & Users | ❌ | ✅ | ✅ |
| Companies & Business Units | ❌ | ✅ | ✅ |
| **ACC Modules** | ❌ | ✅ | ✅ |
| Issues | ❌ | ✅ | ✅ |
| RFIs | ❌ | ✅ | ✅ |
| Assets | ❌ | ✅ | ✅ |
| Submittals | ❌ | ✅ | ✅ |
| Checklists | ❌ | ✅ | ✅ |
| **Design Automation** | ❌ | ✅ | ✅ |
| **Reality Capture** | ❌ | ✅ | ✅ |
| **Webhooks** | ❌ | ✅ | ✅ |
| **Pipelines** | ❌ | ✅ | ✅ |
| **Plugins** | ❌ | ✅ | ✅ |
| **MCP Server** | ❌ | ✅ | ✅ |
| MCP: OSS/Derivative | ❌ | ✅ | ✅ |
| MCP: SSA Tools | ❌ | ✅ | ✅ |
| MCP: Data Management | ❌ | ✅ | ✅ |
| MCP: Account Admin | ❌ | ❌ | ❌ |
| **TUI** | ❌ | ✅ | ✅ |
| **Enterprise Features** | ❌ | ❌ | ✅ |
| Usage Analytics | ❌ | ❌ | ✅ |
| Audit Logs | ❌ | ❌ | ✅ |
| Compliance Policies | ❌ | ❌ | ✅ |
| Multi-tenant Mgmt | ❌ | ❌ | ✅ |
| Enterprise SSO | ❌ | ❌ | ✅ |

### Tier Build Commands

```bash
# Core only (minimal trusted foundation)
cargo build --no-default-features --features core

# Community (default, open source)
cargo build

# Pro (enterprise - requires license)
cargo build --features pro
```

### Tier Enforcement

- **Feature Flags**: Tiers MUST be enforced via Cargo feature flags (`core`, `community`, `pro`)
- **Tier-Gated Commands**: Commands requiring higher tiers MUST fail gracefully with clear upgrade guidance
- **Version Output**: `raps --version` MUST include tier name (e.g., "RAPS Community v3.2.0")
- **License Validation**: Pro tier features MUST validate license before execution

**Rationale**: Tiered strategy enables sustainable business model while keeping core functionality open source. Clear tier boundaries prevent feature leakage and enable independent evolution.

## Performance Standards

### Build Time Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| `cargo check -p raps-kernel` (incremental) | <5s | After single file change in kernel |
| `cargo check` (full workspace, incremental) | <30s | After single file change |
| `cargo build --release` (full) | <3min | Clean build |
| `cargo nextest run` (unit) | <30s | All unit tests |
| `cargo nextest run` (integration) | <2min | Full test suite |
| sccache hit rate (CI) | >80% | Compilation cache effectiveness |

### Runtime Performance Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| CLI startup (--help) | <100ms | Cold start |
| Auth test | <500ms | Network RTT dependent |
| Bucket list (100 items) | <2s | Includes parsing |
| File upload (100MB, parallel) | <30s | Network dependent (100Mbps) |
| Translation status poll | <1s per check | Polling interval |
| MCP tool call | <200ms | AI assistant integration |
| MCP server memory (sustained) | <100MB | Under 100 req/min for 1 hour |
| TUI launch | <500ms | Display auth status |

### Resource Limits

- **Memory**: Peak usage ≤ 256MB for typical operations
- **Disk I/O**: Minimize writes; use streaming for large files
- **Threads**: Limit to `num_cpus` for parallel operations

## Development Workflow

### Branch Strategy

- **main**: Protected; requires PR approval and CI pass
- **feature/[name]**: Short-lived feature branches
- **release/v[X.Y.Z]**: Release preparation branches

### Commit Standards

- Follow Conventional Commits: `type(scope): description`
- Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`
- Breaking changes: Include `BREAKING CHANGE:` footer

### Code Review Requirements

- All PRs MUST have at least 1 approving review
- CI MUST pass: `cargo check`, `cargo clippy`, `cargo test`, `cargo fmt --check`
- New public APIs require documentation review
- Performance-sensitive changes require benchmark comparison

### Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md` with changes since last release
3. Create release tag `vX.Y.Z`
4. CI builds and publishes to crates.io and GitHub Releases
5. Update dependent repos within 1 week

## Governance

### Constitutional Authority

This constitution supersedes all other development practices and guidelines within the RAPS ecosystem. Conflicts MUST be resolved in favor of this document.

### Amendment Process

1. **Proposal**: Submit PR to `.specify/memory/constitution.md` with rationale
2. **Discussion**: Minimum 3-day review period for non-trivial changes
3. **Approval**: Requires maintainer approval
4. **Migration**: Include migration plan for existing code if principles change
5. **Version Bump**: Follow semantic versioning for constitution changes

### Versioning Policy

- **MAJOR**: Backward-incompatible principle changes or removals
- **MINOR**: New principles or substantial guidance additions
- **PATCH**: Clarifications, typo fixes, non-semantic refinements

### Compliance Verification

- All PRs MUST include Constitution Check section in plan documents
- CI pipelines SHOULD enforce measurable requirements (clippy, tests, fmt)
- Quarterly review of constitution relevance recommended

### Guidance References

- Primary development guidance: `raps/CONTRIBUTING.md`
- Exit code reference: `raps/docs/cli/exit-codes.md`
- API coverage: `aps-sdk-openapi/` specifications
- Tier feature matrix: `specs/001-raps-ecosystem-improvements/spec.md` (Tiered Product Strategy section)
- Architecture details: `specs/001-raps-ecosystem-improvements/plan.md` (Microkernel Architecture section)
- Website documentation: `raps-website/README.md` (must reflect tier strategy)

**Version**: 2.0.0 | **Ratified**: 2025-12-29 | **Last Amended**: 2025-12-30

# Feature Specification: Monorepo Microkernel App

**Feature Branch**: `002-monorepo-microkernel-app`  
**Created**: 2026-01-01  
**Status**: Draft  
**Input**: User description: "monorepo microkernel app that will be used to interact with Autodesk Platform Services"

## Executive Summary

This specification defines the monorepo microkernel architecture for RAPS (Rust Autodesk Platform Services CLI) - a unified repository structure that consolidates the kernel, all service modules, community features, and pro features into a single workspace while maintaining strict architectural boundaries.

### Vision: Unified Monorepo with Microkernel Architecture

RAPS will be organized as a **monorepo** containing all core components in a single Rust workspace, enabling atomic changes across kernel, modules, and features while maintaining clear separation of concerns through microkernel architecture principles.

**Key Benefits:**
- **Atomic Changes**: Update kernel and dependent modules in a single commit
- **Shared Tooling**: Unified CI/CD, linting, formatting, and testing infrastructure
- **Dependency Management**: Workspace-level dependency resolution prevents version conflicts
- **Developer Experience**: Single `cargo check` validates entire system
- **Architectural Boundaries**: Microkernel principles enforced through crate dependencies

### Repository Structure

```
raps/ (monorepo workspace)
├── raps-kernel/          # Microkernel foundation crate (<3000 LOC)
├── raps-oss/             # Object Storage Service crate
├── raps-derivative/      # Model Derivative Service crate
├── raps-dm/              # Data Management Service crate
├── raps-ssa/             # Secure Service Accounts crate
├── raps-community/       # Extended Features features crate
├── raps-enterprise/             # Enterprise Features features crate
└── raps/                 # CLI binary crate (depends on all above)
```

**Separate Repositories:**
- **Distribution**: `homebrew-tap`, `scoop-bucket`, `raps-action`, `raps-docker`
- **Documentation**: `raps-website`
- **Social Media Marketing**: `raps-smm`
- **Examples**: `aps-demo-scripts`, `aps-tui`, `aps-wasm-demo`

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Atomic Cross-Module Changes (Priority: P1)

As a developer implementing a new APS API feature, I need to update the kernel, service module, and CLI command in a single atomic commit so that changes are always consistent and can be reviewed together.

**Why this priority**: This is the core value proposition of monorepo - enabling atomic changes across architectural layers. Without this, developers must coordinate changes across multiple repositories, leading to inconsistency and merge conflicts.

**Independent Test**: Can be fully tested by making a change that touches kernel, service module, and CLI, then verifying `cargo check` passes for the entire workspace. Delivers immediate value by ensuring consistency.

**Acceptance Scenarios**:

1. **Given** a new APS API endpoint, **When** I update `raps-kernel` types, `raps-oss` service, and `raps` CLI command in a single commit, **Then** `cargo check` validates all changes together and the commit is atomic.
2. **Given** a breaking change in `raps-kernel`, **When** I update all dependent crates in the same PR, **Then** CI validates that all crates compile together before merge.
3. **Given** a refactoring that affects multiple modules, **When** I use workspace-wide refactoring tools, **Then** changes propagate correctly across all affected crates.

---

### User Story 2 - Unified Development Workflow (Priority: P1)

As a developer working on RAPS, I need a single command to validate the entire system so that I can quickly verify my changes don't break anything across modules.

**Why this priority**: Developer productivity depends on fast feedback loops. Monorepo enables `cargo check` to validate the entire workspace, catching integration issues immediately.

**Independent Test**: Can be fully tested by running `cargo check` in the monorepo root and verifying it checks all crates. Delivers immediate value by providing comprehensive validation.

**Acceptance Scenarios**:

1. **Given** I make a change to `raps-kernel`, **When** I run `cargo check` in the workspace root, **Then** all dependent crates are checked and any breaking changes are immediately visible.
2. **Given** I update a dependency version in workspace `Cargo.toml`, **When** I run `cargo check`, **Then** all crates use the same version and conflicts are detected.
3. **Given** I run `cargo clippy` in the workspace root, **When** linting completes, **Then** all crates are linted with consistent rules.

---

### User Story 3 - Independent Crate Testing (Priority: P2)

As a developer working on a specific service module, I need to test that module in isolation so that I can develop and verify features without building the entire workspace.

**Why this priority**: While monorepo enables atomic changes, developers still need to work on individual modules efficiently. Isolation testing enables parallel development.

**Independent Test**: Can be fully tested by running `cargo test -p raps-oss` and verifying only that crate's tests run. Delivers value by enabling focused development.

**Acceptance Scenarios**:

1. **Given** I'm working on `raps-oss` module, **When** I run `cargo test -p raps-oss`, **Then** only tests for that crate run, without building the entire workspace.
2. **Given** I want to benchmark a specific module, **When** I run `cargo bench -p raps-kernel`, **Then** only that crate's benchmarks execute.
3. **Given** I need to check a single crate's documentation, **When** I run `cargo doc -p raps-derivative --open`, **Then** only that crate's docs are generated and opened.

---

### User Story 4 - Consistent Dependency Versions (Priority: P2)

As a developer, I need all crates in the workspace to use identical dependency versions so that I don't encounter version conflicts or duplicate dependencies in the final binary.

**Why this priority**: Dependency conflicts are a common source of bugs in multi-repo setups. Monorepo with workspace dependencies ensures consistency.

**Independent Test**: Can be fully tested by checking `Cargo.lock` and verifying all crates use the same version of shared dependencies. Delivers value by preventing version conflicts.

**Acceptance Scenarios**:

1. **Given** I update `tokio` version in workspace `Cargo.toml`, **When** I run `cargo update`, **Then** all crates use the same `tokio` version.
2. **Given** multiple crates depend on `serde`, **When** workspace `Cargo.toml` specifies `serde = "1.0"`, **Then** `Cargo.lock` shows a single `serde` version for all crates.
3. **Given** I add a new dependency to workspace, **When** multiple crates use it, **Then** they all reference the workspace dependency, not individual versions.

---

### Edge Cases

- What happens when a crate needs a different version of a dependency than the workspace default?
  - **Resolution**: Use `[dependencies]` override in that crate's `Cargo.toml` with justification comment
- How does the system handle circular dependencies between crates?
  - **Resolution**: Architecture MUST prevent circular dependencies; kernel has no dependencies on service crates
- What happens when CI needs to test only changed crates?
  - **Resolution**: Use `cargo-workspace` or similar tools to detect changed crates and run targeted tests
- How are version bumps handled for individual crates vs workspace?
  - **Resolution**: All crates in workspace share the same version; version bump applies to entire workspace

---

## Requirements *(mandatory)*

### Functional Requirements

#### Monorepo Structure

- **FR-001**: System MUST organize all core components (kernel, modules, tiers, CLI) in a single `raps` repository workspace
- **FR-002**: System MUST use Rust workspace (`Cargo.toml` with `[workspace]` section) to manage all crates
- **FR-003**: System MUST define shared dependencies in workspace `Cargo.toml` `[workspace.dependencies]` section
- **FR-004**: System MUST enforce crate boundaries through `Cargo.toml` dependencies (no circular dependencies)
- **FR-005**: System MUST support building individual crates with `cargo build -p <crate-name>`
- **FR-006**: System MUST support testing individual crates with `cargo test -p <crate-name>`
- **FR-007**: System MUST support building entire workspace with `cargo build --workspace`
- **FR-008**: System MUST generate unified documentation with `cargo doc --workspace`

#### Microkernel Architecture in Monorepo

- **FR-009**: `raps-kernel` crate MUST contain only: Auth, HTTP client, Config, Storage, Types, Error, Logging
- **FR-010**: `raps-kernel` MUST have zero dependencies on service crates (`raps-oss`, `raps-derivative`, `raps-dm`, `raps-ssa`)
- **FR-011**: Service crates (`raps-oss`, `raps-derivative`, `raps-dm`, `raps-ssa`) MUST depend only on `raps-kernel`, not on each other
- **FR-012**: `raps-community` and `raps-enterprise` crates MUST depend only on kernel and service crates, not on each other
- **FR-013**: `raps` CLI binary crate MUST depend on kernel, service crates, and tier crates as needed
- **FR-014**: Kernel MUST compile with `#![deny(warnings)]`, `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`
- **FR-015**: Kernel LOC MUST be <3000 lines (excluding tests); target ~2000 LOC

#### Workspace Dependency Management

- **FR-016**: Workspace MUST define common dependencies in `[workspace.dependencies]` section
- **FR-017**: Crates MUST reference workspace dependencies using `{ workspace = true }` syntax
- **FR-018**: System MUST prevent duplicate dependencies in final binary through workspace resolution
- **FR-019**: System MUST support dependency version overrides in individual crates when justified
- **FR-020**: Workspace `Cargo.lock` MUST be committed to version control

#### Build Performance

- **FR-021**: `cargo check -p raps-kernel` (incremental) MUST complete in <5s
- **FR-022**: `cargo check` (full workspace, incremental) MUST complete in <30s
- **FR-023**: System MUST use sccache for compilation caching
- **FR-024**: System MUST use lld-link on Windows, mold on Linux for faster linking
- **FR-025**: System MUST configure `[profile.dev]` and `[profile.test]` with `debug = 0` to reduce PDB overhead

#### CI/CD Integration

- **FR-026**: CI MUST validate entire workspace with `cargo check --workspace`
- **FR-027**: CI MUST run tests for entire workspace with `cargo nextest run --workspace`
- **FR-028**: CI MUST enforce linting with `cargo clippy --workspace -- -D warnings`
- **FR-029**: CI MUST enforce formatting with `cargo fmt --check --all`
- **FR-030**: CI SHOULD support testing only changed crates for faster feedback (optional optimization)

#### Version Management

- **FR-031**: All crates in workspace MUST share the same version number
- **FR-032**: Version MUST be defined in workspace `Cargo.toml` `[workspace.package]` section
- **FR-033**: Individual crate `Cargo.toml` files MUST reference workspace version using `version.workspace = true`
- **FR-034**: Version bump MUST update workspace version, affecting all crates atomically

### Key Entities

- **Workspace**: The root `raps` repository containing all crates in a single Rust workspace
- **Kernel Crate** (`raps-kernel`): Minimal trusted foundation providing core services (auth, HTTP, config, storage, types, error, logging)
- **Service Crate**: Independent module implementing a specific APS API (OSS, Derivative, DM, SSA)
- **Tier Crate**: Feature collection implementing tier-specific functionality (`raps-community`, `raps-enterprise`)
- **CLI Binary Crate** (`raps`): Main application binary that depends on kernel, services, and tiers
- **Workspace Dependencies**: Shared dependencies defined in workspace `Cargo.toml` and referenced by all crates

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Developers can make atomic changes across kernel, service module, and CLI in a single commit (100% of cross-module changes are atomic)
- **SC-002**: `cargo check` validates entire workspace in <30s for incremental changes (measured after single file change)
- **SC-003**: All crates use identical versions of shared dependencies (verified by `cargo tree` showing single version per dependency)
- **SC-004**: Individual crate builds complete in <5s for kernel, <10s for service modules (incremental builds)
- **SC-005**: CI pipeline validates entire workspace in <5 minutes (including tests, linting, formatting)
- **SC-006**: Zero circular dependencies between crates (verified by `cargo tree` and architecture review)
- **SC-007**: Kernel maintains <3000 LOC (excluding tests) with >90% test coverage
- **SC-008**: All architectural boundaries enforced through `Cargo.toml` dependencies (no runtime violations)

### Architectural Integrity

- **SC-009**: Kernel has zero dependencies on service crates (verified by `cargo tree -p raps-kernel`)
- **SC-010**: Service crates depend only on kernel (verified by `cargo tree -p raps-oss -p raps-derivative -p raps-dm`)
- **SC-011**: Tier crates depend only on kernel and services, not on each other (verified by dependency graph)
- **SC-012**: CLI binary successfully builds with all features enabled (`cargo build --features pro`)

### Developer Experience

- **SC-013**: New developers can clone repository and run `cargo check` successfully within 5 minutes
- **SC-014**: Documentation generation (`cargo doc --workspace --open`) completes in <2 minutes
- **SC-015**: Workspace structure is self-documenting through `Cargo.toml` organization

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         raps/ (CLI Binary)                      │
│              Depends on: kernel + services + tiers              │
└─────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
┌───────▼────────┐   ┌────────▼────────┐   ┌────────▼────────┐
│ raps-community │   │   raps-enterprise      │   │  (other tiers)  │
│ (Community)    │   │   (Enterprise)   │   │                 │
└───────┬────────┘   └────────┬────────┘   └────────┬────────┘
        │                     │                     │
        └─────────────────────┼─────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
┌───────▼────────┐   ┌────────▼────────┐   ┌────────▼────────┐
│   raps-oss      │   │ raps-derivative │   │    raps-dm     │
│   (Service)     │   │    (Service)    │   │   (Service)    │
└───────┬────────┘   └────────┬────────┘   └────────┬────────┘
        │                     │                     │
        └─────────────────────┼─────────────────────┘
                              │
                    ┌─────────▼─────────┐
                    │   raps-kernel      │
                    │   (Foundation)     │
                    │   No dependencies  │
                    │   on services      │
                    └────────────────────┘
```

**Dependency Rules:**
- Kernel: No dependencies on services or tiers
- Services: Depend only on kernel
- Tiers: Depend only on kernel and services
- CLI: Depends on kernel, services, and tiers

---

## Implementation Notes

### Workspace Configuration

The workspace `Cargo.toml` should be structured as:

```toml
[workspace]
members = [
    "raps-kernel",
    "raps-oss",
    "raps-derivative",
    "raps-dm",
    "raps-ssa",
    "raps-community",
    "raps-enterprise",
    "raps",
]

[workspace.package]
version = "3.3.0"
edition = "2021"
authors = ["RAPS Contributors"]
license = "Apache-2.0"

[workspace.dependencies]
# Common dependencies shared across all crates
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
anyhow = "1.0"
# ... other shared dependencies
```

### Crate Configuration Example

Individual crate `Cargo.toml` should reference workspace:

```toml
[package]
name = "raps-oss"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
raps-kernel = { path = "../raps-kernel" }
tokio.workspace = true
serde.workspace = true
```

### Migration Strategy

1. **Phase 1**: Create workspace structure in `raps` repository
2. **Phase 2**: Move kernel and modules into workspace as separate crates
3. **Phase 3**: Update all crate dependencies to use workspace references
4. **Phase 4**: Consolidate CI/CD to validate entire workspace
5. **Phase 5**: Archive separate kernel/module repositories (if they exist)

---

## References

- Constitution: `.specify/memory/constitution.md` (Principle II, VII, Monorepo Architecture section)
- Rust Workspace Documentation: https://doc.rust-lang.org/cargo/reference/workspaces.html
- Microkernel Architecture: `specs/001-raps-ecosystem-improvements/spec.md`



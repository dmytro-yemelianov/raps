# Implementation Plan: RAPS Ecosystem Improvements

**Branch**: `001-raps-ecosystem-improvements` | **Date**: 2025-12-29 | **Updated**: 2025-12-30 | **Spec**: [spec.md](spec.md)  
**Status**: Planning Complete (Phase 0 & Phase 1 artifacts ready)  
**Input**: Feature specification from `/specs/001-raps-ecosystem-improvements/spec.md`

## Summary

Comprehensive improvements to the RAPS (Rust Autodesk Platform Services) ecosystem addressing:
- **Microkernel Architecture**: Refactor to Unix-like kernel design for performance, security, and testability
- **Tiered Product Strategy**: Core → Community → Pro evolution path
- **APS API Alignment**: SSA, Account Admin, Data Management navigation, Pro tier features
- Performance bottlenecks (parallel uploads, async blocking, pagination)
- Architecture consistency (unified retry, config-based URLs, output schemas)
- Cross-interface parity (CLI ↔ MCP ↔ GitHub Action ↔ Docker)
- TUI development and shared library extraction

## Technical Context

**Language/Version**: Rust 1.88+ (Edition 2024)  
**Primary Dependencies**: clap 4.4, reqwest 0.11, tokio 1.35, rmcp 0.12, ratatui (TUI)  
**Storage**: File-based config + platform keyring for secrets  
**Testing**: cargo nextest, assert_cmd, predicates  
**Target Platform**: Windows, macOS, Linux (x64, arm64)  
**Project Type**: Multi-crate workspace (microkernel architecture)  
**Performance Goals**: 100MB upload <30s, CLI startup <100ms, MCP memory <100MB, kernel check <5s, workspace check <30s  
**Build Tooling**: lld-link (Windows), mold (Linux CI), sccache, cargo-nextest  
**Constraints**: MSRV 1.88, Apache 2.0 license (Core/Community), proprietary (Pro)  
**Scale/Scope**: 9 repositories, 50+ CLI commands, 14 MCP tools, 3 product tiers

---

## Repository Taxonomy

### Core Application
| Repository | Role | Description |
|------------|------|-------------|
| `raps` | **Primary Application** | Main Rust CLI application - the kernel and all tiers |

### Documentation & Website
| Repository | Role | Description |
|------------|------|-------------|
| `raps-website` | **Documentation Hub** | Main website (rapscli.xyz), docs, blog, API reference |

### Distribution Satellites
| Repository | Role | Description |
|------------|------|-------------|
| `homebrew-tap` | Package Manager | macOS/Linux Homebrew formula |
| `scoop-bucket` | Package Manager | Windows Scoop manifest |
| `raps-action` | CI/CD Integration | GitHub Actions composite action |
| `raps-docker` | Containerization | Docker image and compose files |

### Ecosystem Extensions
| Repository | Role | Description |
|------------|------|-------------|
| `aps-tui` | Terminal UI | Interactive TUI application |
| `aps-wasm-demo` | WebAssembly | Browser-based demo |
| `aps-sdk-openapi` | API Specs | OpenAPI specifications for APS |

---

## Microkernel Architecture

### Design Philosophy

Inspired by Unix OS design principles, RAPS will adopt a **microkernel architecture** where:

1. **Minimal Kernel**: Core functionality is small, audited, and battle-tested
2. **Message Passing**: Components communicate through well-defined interfaces
3. **Isolation**: Failures in extensions don't crash the kernel
4. **Extensibility**: Features added via plugins, not kernel modifications
5. **Security**: Attack surface minimized in the trusted kernel

### Architecture Layers

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              USER INTERFACES                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐      │
│  │   CLI    │  │   TUI    │  │   MCP    │  │  Action  │  │  Docker  │      │
│  │  (clap)  │  │(ratatui) │  │  (rmcp)  │  │  (yaml)  │  │(container│      │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘      │
│       │             │             │             │             │             │
│       └─────────────┴─────────────┴─────────────┴─────────────┘             │
│                                   │                                          │
├───────────────────────────────────┼──────────────────────────────────────────┤
│                           PRODUCT TIERS                                      │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                         raps-pro (Enterprise)                        │    │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │    │
│  │  │ Analytics   │ │ Compliance  │ │ Multi-Tenant│ │ Priority    │   │    │
│  │  │ Dashboard   │ │ Reporting   │ │ Management  │ │ Support API │   │    │
│  │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘   │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      raps-community (Free)                           │    │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐   │    │
│  │  │ ACC Issues  │ │ ACC RFIs    │ │ ACC Assets  │ │ Webhooks    │   │    │
│  │  │ Reality Cap │ │ Design Auto │ │ Pipelines   │ │ Plugins     │   │    │
│  │  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘   │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────────────────────┤
│                         MICROKERNEL (raps-kernel)                            │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                        Trusted Core (~2000 LOC)                      │    │
│  │  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐           │    │
│  │  │   Auth    │ │   HTTP    │ │  Config   │ │   Error   │           │    │
│  │  │ (OAuth2)  │ │ (reqwest) │ │ (serde)   │ │ (thiserror│           │    │
│  │  └───────────┘ └───────────┘ └───────────┘ └───────────┘           │    │
│  │  ┌───────────┐ ┌───────────┐ ┌───────────┐ ┌───────────┐           │    │
│  │  │  Storage  │ │   Types   │ │  Logging  │ │  Secrets  │           │    │
│  │  │ (keyring) │ │ (domain)  │ │ (tracing) │ │ (redact)  │           │    │
│  │  └───────────┘ └───────────┘ └───────────┘ └───────────┘           │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      Essential Services (OSS, Derivative)            │    │
│  │  ┌───────────────────────────────┐ ┌───────────────────────────────┐│    │
│  │  │      OSS Client               │ │    Model Derivative Client    ││    │
│  │  │  - Bucket CRUD                │ │  - Translation Jobs           ││    │
│  │  │  - Object CRUD                │ │  - Manifest Queries           ││    │
│  │  │  - Parallel Uploads           │ │  - Derivative Downloads       ││    │
│  │  │  - Signed URLs                │ │                               ││    │
│  │  └───────────────────────────────┘ └───────────────────────────────┘│    │
│  └─────────────────────────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────────────────────────┤
│                              PLATFORM LAYER                                  │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            │
│  │  Tokio RT   │ │  Reqwest    │ │  Keyring    │ │  Crossterm  │            │
│  │  (async)    │ │  (HTTP/TLS) │ │  (secrets)  │ │  (terminal) │            │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘            │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Kernel Guarantees

The microkernel (`raps-kernel`) provides these guarantees:

| Guarantee | Implementation |
|-----------|----------------|
| **No Panics** | All fallible operations return `Result<T, RapsError>` |
| **No Blocking** | All I/O wrapped in async; blocking ops use `spawn_blocking` |
| **Bounded Memory** | Streaming APIs for large data; no unbounded buffers |
| **Constant Startup** | <100ms startup; lazy initialization for optional features |
| **Auditable** | ~2000 LOC kernel; 100% test coverage on core paths |
| **Secure by Default** | Secret redaction, HTTPS-only, minimal scopes |

---

## Tiered Product Strategy

### Product Tiers

```
┌──────────────────────────────────────────────────────────────────────┐
│                                                                       │
│   ┌─────────────────────────────────────────────────────────────┐    │
│   │                     RAPS Pro (Enterprise)                    │    │
│   │                                                              │    │
│   │  Everything in Community, plus:                             │    │
│   │  • Usage Analytics & Audit Logs                             │    │
│   │  • Compliance Reporting (SOC2, GDPR)                        │    │
│   │  • Multi-tenant Project Management                          │    │
│   │  • Priority API Rate Limits                                 │    │
│   │  • SSO/SAML Integration                                     │    │
│   │  • Dedicated Support SLA                                    │    │
│   │  • On-premise Deployment Option                             │    │
│   │                                                              │    │
│   │  License: Commercial (Subscription)                         │    │
│   └─────────────────────────────────────────────────────────────┘    │
│                              ▲                                        │
│                              │                                        │
│   ┌─────────────────────────────────────────────────────────────┐    │
│   │                   RAPS Community (Free)                      │    │
│   │                                                              │    │
│   │  Everything in Core, plus:                                  │    │
│   │  • ACC Issues, RFIs, Assets, Submittals, Checklists         │    │
│   │  • Design Automation (Engines, Activities, Work Items)      │    │
│   │  • Reality Capture (Photogrammetry)                         │    │
│   │  • Webhooks Management                                      │    │
│   │  • Pipeline Automation (YAML/JSON workflows)                │    │
│   │  • Plugin System (External, Hooks, Aliases)                 │    │
│   │  • MCP Server (AI Assistant Integration)                    │    │
│   │  • TUI (Terminal User Interface)                            │    │
│   │                                                              │    │
│   │  License: Apache 2.0 (Open Source)                          │    │
│   └─────────────────────────────────────────────────────────────┘    │
│                              ▲                                        │
│                              │                                        │
│   ┌─────────────────────────────────────────────────────────────┐    │
│   │                    RAPS Core (Kernel)                        │    │
│   │                                                              │    │
│   │  Essential Foundation:                                      │    │
│   │  • Authentication (2-legged, 3-legged, Device Code)         │    │
│   │  • OSS (Buckets, Objects, Uploads, Downloads)               │    │
│   │  • Model Derivative (Translate, Status, Manifest)           │    │
│   │  • Data Management (Hubs, Projects, Folders, Items)         │    │
│   │  • Configuration & Profiles                                 │    │
│   │  • Output Formats (JSON, YAML, CSV, Table)                  │    │
│   │  • Shell Completions                                        │    │
│   │                                                              │    │
│   │  License: Apache 2.0 (Open Source)                          │    │
│   └─────────────────────────────────────────────────────────────┘    │
│                                                                       │
└──────────────────────────────────────────────────────────────────────┘
```

### Feature Matrix

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
| Signed URLs | ✅ | ✅ | ✅ |
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
| Priority Rate Limits | ❌ | ❌ | ✅ |
| Dedicated Support | ❌ | ❌ | ✅ |

---

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Status |
|-----------|-------|--------|
| I. Rust Idiomatic Code Quality | All changes pass clippy, rustfmt, no unwrap in lib | ✅ Required |
| II. Cross-Repository Consistency | Shared crate extraction, version alignment, raps-website docs | ✅ Addressed |
| III. Test-First Development | Tests before implementation for each phase | ✅ Required |
| IV. User Experience Consistency | Output schema formalization, error message consistency | ✅ Addressed |
| V. Performance & Resource Efficiency | Parallel uploads, async fixes, memory bounds, kernel check <5s, workspace <30s | ✅ Primary focus |
| VI. Security & Secrets Handling | Container credential handling, secret redaction | ✅ Addressed |
| VII. Microkernel Architecture | Kernel isolation, service independence, kernel size <3000 LOC, >90% test coverage | ✅ Core requirement |

---

## Project Structure

### Documentation (this feature)

```text
specs/001-raps-ecosystem-improvements/
├── plan.md              # This file
├── research.md          # Current state analysis (complete)
├── spec.md              # Feature specification (complete)
├── data-model.md        # Domain types (complete)
├── quickstart.md        # Usage examples (complete)
├── contracts/           # API schemas (complete)
│   ├── ssa-api.yaml
│   ├── account-admin-api.yaml
│   ├── data-management-api.yaml
│   ├── mcp-server-api.json
│   └── pro-tier-api.yaml
└── tasks.md             # Task breakdown (complete)
```

### Source Code (repository root) - Microkernel Layout

```text
# NEW: Microkernel Architecture

raps-kernel/                         # ★ MICROKERNEL: Minimal trusted core
├── Cargo.toml
├── src/
│   ├── lib.rs                       # Public API surface
│   ├── auth/                        # Authentication (OAuth2 flows)
│   │   ├── mod.rs
│   │   ├── two_legged.rs
│   │   ├── three_legged.rs
│   │   ├── device_code.rs
│   │   └── token.rs
│   ├── http/                        # HTTP client with retry/backoff
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── retry.rs
│   │   └── middleware.rs
│   ├── config/                      # Configuration management
│   │   ├── mod.rs
│   │   ├── endpoints.rs
│   │   └── profiles.rs
│   ├── storage/                     # Secure credential storage
│   │   ├── mod.rs
│   │   ├── keyring.rs
│   │   └── file.rs
│   ├── types/                       # Domain primitives
│   │   ├── mod.rs
│   │   ├── urn.rs
│   │   ├── bucket.rs
│   │   └── object.rs
│   ├── error.rs                     # Error types with exit codes
│   └── logging.rs                   # Tracing & secret redaction
└── tests/
    ├── auth_test.rs
    ├── http_test.rs
    └── integration/

raps-ssa/                            # ★ SSA Service (Core tier - CI/CD critical)
├── Cargo.toml                       # Depends on raps-kernel
├── src/
│   ├── lib.rs
│   ├── service_account.rs           # Account CRUD
│   ├── key.rs                       # Key generation/management
│   ├── jwt.rs                       # JWT assertion generation (RS256)
│   └── token_exchange.rs            # JWT → 3LO token exchange
└── tests/

raps-oss/                            # ★ OSS Service (kernel extension)
├── Cargo.toml                       # Depends on raps-kernel
├── src/
│   ├── lib.rs
│   ├── bucket.rs
│   ├── object.rs
│   ├── upload.rs                    # Parallel multipart uploads
│   ├── download.rs
│   └── signed_url.rs
└── tests/

raps-derivative/                     # ★ Model Derivative Service
├── Cargo.toml                       # Depends on raps-kernel
├── src/
│   ├── lib.rs
│   ├── translate.rs
│   ├── manifest.rs
│   └── download.rs
└── tests/

raps-dm/                             # ★ Data Management Service (enhanced)
├── Cargo.toml                       # Depends on raps-kernel
├── src/
│   ├── lib.rs
│   ├── hub.rs                       # Hub navigation
│   ├── project.rs                   # Project navigation
│   ├── folder.rs                    # Folder contents, top folders
│   ├── item.rs                      # Item CRUD
│   ├── version.rs                   # Versions, tip version
│   └── storage.rs                   # Storage creation for uploads
└── tests/

raps-community/                      # ★ Community tier features
├── Cargo.toml                       # Depends on raps-kernel, raps-oss, etc.
├── src/
│   ├── lib.rs
│   ├── acc/                         # ACC modules
│   │   ├── issues.rs
│   │   ├── rfi.rs
│   │   ├── assets.rs
│   │   ├── submittals.rs
│   │   └── checklists.rs
│   ├── da/                          # Design Automation
│   ├── reality/                     # Reality Capture
│   ├── webhooks/
│   ├── pipeline/
│   └── plugin/
└── tests/

raps-admin/                          # ★ Account Admin Service (Community tier)
├── Cargo.toml                       # Depends on raps-kernel
├── src/
│   ├── lib.rs
│   ├── project.rs                   # Project CRUD
│   ├── user.rs                      # User assignment/management
│   ├── company.rs                   # Company directory
│   └── business_unit.rs             # Business units structure
└── tests/

raps-pro/                            # ★ Enterprise tier (proprietary)
├── Cargo.toml                       # Depends on raps-community
├── src/
│   ├── lib.rs
│   ├── analytics/                   # Usage analytics
│   ├── audit/                       # Audit logging
│   ├── compliance/                  # SOC2, GDPR reporting
│   ├── multitenant/                 # Tenant management
│   └── sso/                         # SSO/SAML integration
└── tests/

raps/                                # ★ CLI Application (thin shell)
├── Cargo.toml                       # Feature flags select tier
├── src/
│   ├── main.rs                      # Entry point
│   ├── cli.rs                       # Clap definitions
│   ├── commands/                    # Command handlers (dispatch only)
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── bucket.rs
│   │   └── ...
│   ├── mcp/                         # MCP server
│   └── output.rs                    # Output formatting
└── tests/

aps-tui/                             # TUI Application
├── Cargo.toml                       # Depends on raps-kernel, raps-oss, etc.
└── src/

# Distribution Satellites (unchanged)
raps-action/                         # GitHub Action
raps-docker/                         # Docker container
homebrew-tap/                        # Homebrew formula
scoop-bucket/                        # Scoop manifest

# Documentation
raps-website/                        # Main website & docs (Astro)
├── src/
│   ├── content/
│   │   ├── docs/                    # Documentation
│   │   └── blog/                    # Blog posts
│   └── pages/
└── astro.config.mjs
```

### Cargo Workspace Configuration

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "raps-kernel",
    "raps-ssa",           # NEW: SSA service (Core tier)
    "raps-oss",
    "raps-derivative",
    "raps-dm",
    "raps-admin",         # NEW: Account Admin (Community tier)
    "raps-community",
    "raps-pro",
    "raps",
    "aps-tui",
]

[workspace.package]
edition = "2024"
rust-version = "1.88"
license = "Apache-2.0"
repository = "https://github.com/dmytro-yemelianov/raps"

[workspace.dependencies]
# Shared dependency versions
tokio = { version = "1.35", features = ["full"] }
reqwest = { version = "0.11", features = ["json", "stream", "rustls-tls"], default-features = false }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
# SSA dependencies
jsonwebtoken = "9.3"          # For JWT assertion generation (RS256)
sha2 = "0.10"                 # For audit log integrity hashing
chrono = { version = "0.4", features = ["serde"] }
# SSO dependencies (Pro tier)
openidconnect = "5.0"          # For OIDC SSO support
```

### Feature Flags for Tier Selection

```toml
# raps/Cargo.toml
[package]
name = "raps"

[features]
default = ["community"]
core = ["raps-kernel", "raps-ssa", "raps-oss", "raps-derivative", "raps-dm"]
community = ["core", "raps-admin", "raps-community"]
pro = ["community", "raps-pro"]

[dependencies]
raps-kernel = { path = "../raps-kernel" }
raps-ssa = { path = "../raps-ssa" }              # NEW: SSA (Core)
raps-oss = { path = "../raps-oss" }
raps-derivative = { path = "../raps-derivative" }
raps-dm = { path = "../raps-dm" }
raps-admin = { path = "../raps-admin", optional = true }  # NEW: Account Admin (Community)
raps-community = { path = "../raps-community", optional = true }
raps-pro = { path = "../raps-pro", optional = true }
```

**Build commands by tier:**
```bash
# Core only (minimal)
cargo build --no-default-features --features core

# Community (default, open source)
cargo build

# Pro (enterprise)
cargo build --features pro
```

---

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Microkernel split (7 crates) | Isolation, testability, security auditing | Monolith untestable; changes risk regressions |
| Tiered feature flags | Business model; license separation | Single tier limits monetization |
| Workspace dependencies | Version consistency across crates | Per-crate deps cause version drift |
| Parallel upload complexity | 6x performance for large files | Sequential unacceptable for 100MB+ |
| Output schemas | Contract testing, documentation | Ad-hoc JSON breaks scripts |

---

## Phase 0: Microkernel Foundation (Week 1-2)

### Objective
Extract the minimal trusted kernel from the existing monolith.

### Kernel Extraction Criteria

Code belongs in `raps-kernel` if it:
1. Is required by ALL higher layers
2. Handles security-critical operations (auth, secrets)
3. Is called on EVERY command invocation
4. Has zero dependencies on specific APS APIs

### Tasks

#### 0.1 Create Workspace Structure

```bash
# Create crate directories
mkdir -p raps-kernel/src/{auth,http,config,storage,types}
mkdir -p raps-oss/src
mkdir -p raps-derivative/src
mkdir -p raps-dm/src
mkdir -p raps-community/src/{acc,da,reality,webhooks,pipeline,plugin}
mkdir -p raps-pro/src/{analytics,audit,compliance,multitenant,sso}
```

#### 0.2 Extract Kernel Components

**From `raps/src/` to `raps-kernel/src/`:**

| Source | Destination | Notes |
|--------|-------------|-------|
| `error.rs` | `error.rs` | Add exit code mapping |
| `http.rs` | `http/client.rs`, `http/retry.rs` | Split concerns |
| `config.rs` | `config/mod.rs` | Extract endpoints |
| `storage.rs` | `storage/mod.rs` | Keyring + file backends |
| `logging.rs` | `logging.rs` | Tracing + redaction |
| Types from `api/*.rs` | `types/*.rs` | Domain primitives only |
| Auth logic from `api/auth.rs` | `auth/*.rs` | OAuth flows |

#### 0.3 Define Kernel Public API

```rust
// raps-kernel/src/lib.rs
#![deny(warnings)]
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]

//! RAPS Kernel - Minimal trusted core for APS CLI operations.
//!
//! This crate provides the foundational infrastructure:
//! - Authentication (OAuth2 flows)
//! - HTTP client with retry/backoff
//! - Configuration management
//! - Secure credential storage
//! - Domain types and error handling

pub mod auth;
pub mod config;
pub mod error;
pub mod http;
pub mod logging;
pub mod storage;
pub mod types;

// Re-exports for convenience
pub use auth::{AccessToken, AuthClient, AuthConfig};
pub use config::{ApsEndpoints, Config, Profile};
pub use error::{RapsError, Result};
pub use http::{HttpClient, HttpClientConfig, RetryConfig};
pub use types::{BucketKey, ObjectKey, Urn};
```

#### 0.4 Kernel Test Suite

**Target: 100% coverage on kernel paths**

```rust
// raps-kernel/tests/auth_test.rs
#[tokio::test]
async fn test_two_legged_auth_returns_valid_token() { ... }

#[tokio::test]
async fn test_token_refresh_before_expiry() { ... }

#[tokio::test]
async fn test_auth_error_returns_exit_code_3() { ... }

// raps-kernel/tests/http_test.rs
#[tokio::test]
async fn test_retry_on_429_with_backoff() { ... }

#[tokio::test]
async fn test_no_retry_on_4xx_client_errors() { ... }

#[tokio::test]
async fn test_timeout_respected() { ... }
```

#### 0.5 Build Performance Infrastructure

**Goal**: Achieve <5s kernel check, <30s full workspace check.

##### 0.5.1 Configure Fast Linkers

Create `.cargo/config.toml`:

```toml
# .cargo/config.toml
# Fast linker configuration for development

[target.x86_64-pc-windows-msvc]
linker = "lld-link"

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.aarch64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

##### 0.5.2 Reduce Debug Symbol Overhead

Update workspace `Cargo.toml`:

```toml
# Cargo.toml (workspace root)

[profile.dev]
debug = 0  # Eliminate PDB generation overhead

[profile.test]
debug = 0  # Fast test builds

[profile.dev.package."*"]
opt-level = 0
```

##### 0.5.3 Configure CI Build Caching

**GitHub Actions workflow snippet**:

```yaml
# .github/workflows/ci.yml

jobs:
  build:
    steps:
      # Install sccache
      - name: Install sccache
        uses: mozilla-actions/sccache-action@v0.0.6
        
      - name: Configure sccache
        run: |
          echo "RUSTC_WRAPPER=sccache" >> $GITHUB_ENV
          echo "SCCACHE_GHA_ENABLED=true" >> $GITHUB_ENV
      
      # Install cargo-nextest for parallel tests
      - name: Install cargo-nextest
        uses: taiki-e/install-action@nextest
      
      # Install mold linker (Linux)
      - name: Install mold
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y mold
      
      # Build with timing report
      - name: Build with timing
        run: cargo build --timings --release
      
      # Upload timing report
      - name: Upload build timing
        uses: actions/upload-artifact@v4
        with:
          name: cargo-timing-${{ runner.os }}
          path: target/cargo-timings/cargo-timing-*.html
      
      # Run tests with nextest
      - name: Run tests
        run: cargo nextest run --workspace
```

##### 0.5.4 Document Local Setup

Add to `README.md`:

```markdown
## Fast Development Setup (Optional)

For faster build times, install these optional tools:

### Windows
```powershell
# Install LLVM for lld-link
winget install LLVM.LLVM

# Install sccache
cargo install sccache
setx RUSTC_WRAPPER sccache

# Install cargo-nextest
cargo install cargo-nextest
```

### Linux
```bash
# Install mold linker
sudo apt install mold  # or: brew install mold

# Install sccache
cargo install sccache
export RUSTC_WRAPPER=sccache

# Install cargo-nextest
cargo install cargo-nextest
```

### Verify Setup
```bash
# Check sccache is working
sccache --show-stats

# Verify fast builds
cargo check -p raps-kernel  # Should be <5s incremental
cargo check                 # Should be <30s incremental
```
```

---

## Phase 1: Service Extraction (Week 3-4)

### Objective
Extract API services from monolith into dedicated crates.

### Tasks

#### 1.1 Extract OSS Service (`raps-oss`)

```rust
// raps-oss/src/lib.rs
use raps_kernel::{HttpClient, Result};

pub struct OssClient {
    http: HttpClient,
}

impl OssClient {
    pub async fn list_buckets(&self, region: Option<&str>) -> Result<Vec<Bucket>> { ... }
    pub async fn create_bucket(&self, key: &BucketKey, policy: Policy) -> Result<Bucket> { ... }
    pub async fn upload_object(&self, bucket: &BucketKey, key: &ObjectKey, data: impl AsyncRead) -> Result<Object> { ... }
    pub async fn upload_parallel(&self, bucket: &BucketKey, key: &ObjectKey, file: &Path, concurrency: usize) -> Result<Object> { ... }
}
```

#### 1.2 Extract Derivative Service (`raps-derivative`)

```rust
// raps-derivative/src/lib.rs
use raps_kernel::{HttpClient, Result, Urn};

pub struct DerivativeClient {
    http: HttpClient,
}

impl DerivativeClient {
    pub async fn translate(&self, urn: &Urn, format: OutputFormat) -> Result<TranslationJob> { ... }
    pub async fn status(&self, urn: &Urn) -> Result<TranslationStatus> { ... }
    pub async fn manifest(&self, urn: &Urn) -> Result<Manifest> { ... }
}
```

#### 1.3 Extract Data Management Service (`raps-dm`)

```rust
// raps-dm/src/lib.rs
use raps_kernel::{HttpClient, Result};

pub struct DataManagementClient {
    http: HttpClient,
}

impl DataManagementClient {
    pub async fn list_hubs(&self) -> Result<Vec<Hub>> { ... }
    pub async fn get_hub(&self, hub_id: &str) -> Result<Hub> { ... }
    pub async fn list_projects(&self, hub_id: &str) -> Result<Vec<Project>> { ... }
    pub async fn get_project(&self, hub_id: &str, project_id: &str) -> Result<Project> { ... }
    pub async fn list_top_folders(&self, hub_id: &str, project_id: &str) -> Result<Vec<Folder>> { ... }
    pub async fn list_folders(&self, project_id: &str, folder_id: &str) -> Result<Vec<Folder>> { ... }
    pub async fn list_folder_contents(&self, project_id: &str, folder_id: &str) -> Result<FolderContents> { ... }
    pub async fn get_item(&self, project_id: &str, item_id: &str) -> Result<Item> { ... }
    pub async fn list_item_versions(&self, project_id: &str, item_id: &str) -> Result<Vec<Version>> { ... }
    pub async fn get_tip_version(&self, project_id: &str, item_id: &str) -> Result<Version> { ... }
    pub async fn create_storage(&self, project_id: &str, storage: StorageRequest) -> Result<StorageLocation> { ... }
}
```

#### 1.5 Update CLI to Use Services

```rust
// raps/src/commands/bucket.rs
use raps_oss::OssClient;

pub async fn handle_bucket_list(client: &OssClient, region: Option<String>) -> Result<()> {
    let buckets = client.list_buckets(region.as_deref()).await?;
    output::render(buckets)?;
    Ok(())
}

// raps/src/commands/ssa.rs (NEW)
use raps_ssa::SsaClient;

pub async fn handle_ssa_create(client: &SsaClient, name: String) -> Result<()> {
    let account = client.create_service_account(&name).await?;
    output::render(account)?;
    Ok(())
}

pub async fn handle_ssa_key_create(client: &SsaClient, account_id: String) -> Result<()> {
    let key = client.create_key(&account_id).await?;
    // Display private key once (never logged)
    println!("{}", key.private_key);
    Ok(())
}

pub async fn handle_auth_ssa_token(client: &SsaClient, config: SsaConfig, scopes: Vec<String>) -> Result<()> {
    let assertion = client.generate_jwt_assertion(&config)?;
    let token = client.exchange_token(&assertion, &scopes).await?;
    output::render(token)?;
    Ok(())
}

// raps/src/commands/dm.rs (NEW - enhanced)
use raps_dm::DataManagementClient;

pub async fn handle_dm_hubs_list(client: &DataManagementClient) -> Result<()> {
    let hubs = client.list_hubs().await?;
    output::render(hubs)?;
    Ok(())
}

pub async fn handle_dm_tip_version(client: &DataManagementClient, project_id: String, item_id: String) -> Result<()> {
    let version = client.get_tip_version(&project_id, &item_id).await?;
    // Extract derivatives URN for Viewer SDK
    if let Some(derivatives_urn) = version.derivatives_urn {
        output::render(json!({ "urn": derivatives_urn }))?;
    }
    Ok(())
}
```

---

## Phase 2: Community Tier (Week 5-6)

### Objective
Package community features into dedicated crate.

### Tasks

#### 2.1 Extract ACC Modules

```rust
// raps-community/src/acc/issues.rs
use raps_kernel::{HttpClient, Result};

pub struct IssuesClient {
    http: HttpClient,
}

impl IssuesClient {
    pub async fn list(&self, project_id: &str) -> Result<Vec<Issue>> { ... }
    pub async fn create(&self, project_id: &str, issue: CreateIssue) -> Result<Issue> { ... }
    pub async fn update(&self, project_id: &str, issue_id: &str, update: UpdateIssue) -> Result<Issue> { ... }
}
```

#### 2.2 Extract Design Automation

```rust
// raps-community/src/da/mod.rs
pub struct DesignAutomationClient { ... }

impl DesignAutomationClient {
    pub async fn list_engines(&self) -> Result<Vec<Engine>> { ... }
    pub async fn create_activity(&self, activity: CreateActivity) -> Result<Activity> { ... }
    pub async fn submit_workitem(&self, workitem: WorkItem) -> Result<WorkItemStatus> { ... }
}
```

#### 2.3 Extract Plugin System

```rust
// raps-community/src/plugin/mod.rs
pub struct PluginManager {
    plugins: Vec<Plugin>,
    aliases: HashMap<String, String>,
}

impl PluginManager {
    pub fn discover(&mut self) -> Result<()> { ... }
    pub fn execute_hook(&self, hook: Hook, context: &Context) -> Result<()> { ... }
}
```

#### 2.6 CLI Commands for Admin/SSA/DM

```rust
// raps/src/commands/admin.rs (NEW)
use raps_admin::AccountAdminClient;

pub async fn handle_admin_projects_list(client: &AccountAdminClient, account_id: String) -> Result<()> {
    let projects = client.list_projects(&account_id).await?;
    output::render(projects)?;
    Ok(())
}

pub async fn handle_admin_projects_create(client: &AccountAdminClient, account_id: String, name: String) -> Result<()> {
    let project = client.create_project(&account_id, CreateProject { name }).await?;
    output::render(project)?;
    Ok(())
}

pub async fn handle_admin_users_assign(client: &AccountAdminClient, project_id: String, email: String) -> Result<()> {
    let user = client.assign_user(&project_id, AssignUser { email }).await?;
    output::render(user)?;
    Ok(())
}

pub async fn handle_admin_companies_list(client: &AccountAdminClient, account_id: String) -> Result<()> {
    let companies = client.list_companies(&account_id).await?;
    output::render(companies)?;
    Ok(())
}
```

#### 2.4 Extract Account Admin Service (`raps-admin`)

```rust
// raps-admin/src/lib.rs (Community tier)
use raps_kernel::{HttpClient, Result};

pub struct AccountAdminClient {
    http: HttpClient,
}

impl AccountAdminClient {
    pub async fn list_projects(&self, account_id: &str) -> Result<Vec<Project>> { ... }
    pub async fn create_project(&self, account_id: &str, project: CreateProject) -> Result<Project> { ... }
    pub async fn get_project(&self, project_id: &str) -> Result<Project> { ... }
    pub async fn list_project_users(&self, project_id: &str) -> Result<Vec<ProjectUser>> { ... }
    pub async fn assign_user(&self, project_id: &str, user: AssignUser) -> Result<ProjectUser> { ... }
    pub async fn list_companies(&self, account_id: &str) -> Result<Vec<Company>> { ... }
    pub async fn create_company(&self, account_id: &str, company: CreateCompany) -> Result<Company> { ... }
    pub async fn list_business_units(&self, account_id: &str) -> Result<BusinessUnitStructure> { ... }
}
```

#### 2.5 Extract SSA Service (`raps-ssa`) - Core Tier

```rust
// raps-ssa/src/lib.rs (Core tier - critical for CI/CD)
use raps_kernel::{HttpClient, Result};
use jsonwebtoken::{encode, Header, EncodingKey};
use serde::{Deserialize, Serialize};

pub struct SsaClient {
    http: HttpClient,
}

#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,  // Client ID
    sub: String,  // Service Account ID
    aud: String,  // Token endpoint
    exp: i64,     // Expiration (max 5 min)
    scope: Vec<String>,
}

impl SsaClient {
    pub async fn create_service_account(&self, name: &str) -> Result<ServiceAccount> { ... }
    pub async fn list_service_accounts(&self) -> Result<Vec<ServiceAccount>> { ... }
    pub async fn get_service_account(&self, account_id: &str) -> Result<ServiceAccount> { ... }
    pub async fn create_key(&self, account_id: &str) -> Result<ServiceAccountKey> { ... }
    pub async fn list_keys(&self, account_id: &str) -> Result<Vec<KeyMetadata>> { ... }
    pub async fn disable_key(&self, account_id: &str, key_id: &str) -> Result<()> { ... }
    pub async fn enable_key(&self, account_id: &str, key_id: &str) -> Result<()> { ... }
    pub async fn delete_key(&self, account_id: &str, key_id: &str) -> Result<()> { ... }
    pub fn generate_jwt_assertion(&self, config: &SsaConfig) -> Result<String> {
        // RS256 signing with private key
        let claims = JwtClaims { ... };
        encode(&Header::new(jsonwebtoken::Algorithm::RS256), &claims, &EncodingKey::from_rsa_pem(&config.private_key)?)
    }
    pub async fn exchange_token(&self, assertion: &str, scopes: &[String]) -> Result<AccessToken> { ... }
}
```

#### 2.6 MCP Server Integration with SSA and Data Management

```rust
// raps/src/mcp/server.rs
use raps_kernel::*;
use raps_oss::OssClient;
use raps_derivative::DerivativeClient;
use raps_dm::DataManagementClient;
use raps_ssa::SsaClient;
#[cfg(feature = "community")]
use raps_community::*;
#[cfg(feature = "community")]
use raps_admin::AccountAdminClient;

pub struct McpServer {
    kernel: Arc<Kernel>,
    oss: OssClient,
    derivative: DerivativeClient,
    dm: DataManagementClient,
    ssa: SsaClient,
    #[cfg(feature = "community")]
    issues: IssuesClient,
    #[cfg(feature = "community")]
    admin: AccountAdminClient,
}

impl McpServer {
    // SSA tools (FR-MCP-005)
    pub async fn ssa_create(&self, name: String) -> Result<ServiceAccount> { ... }
    pub async fn ssa_list(&self) -> Result<Vec<ServiceAccount>> { ... }
    pub async fn ssa_key_create(&self, account_id: String) -> Result<ServiceAccountKey> { ... }
    pub async fn ssa_token(&self, assertion: String, scopes: Vec<String>) -> Result<AccessToken> { ... }
    
    // Data Management tools (FR-MCP-006)
    pub async fn dm_hubs_list(&self) -> Result<Vec<Hub>> { ... }
    pub async fn dm_projects_list(&self, hub_id: String) -> Result<Vec<Project>> { ... }
    pub async fn dm_folders_list(&self, project_id: String, folder_id: String) -> Result<Vec<Folder>> { ... }
    pub async fn dm_versions_list(&self, project_id: String, item_id: String) -> Result<Vec<Version>> { ... }
    pub async fn dm_tip_version(&self, project_id: String, item_id: String) -> Result<Version> { ... }
    
    // Note: Account Admin tools NOT exposed via MCP (per Q2 clarification)
}
```

---

## Phase 3: Performance & Polish (Week 7-8)

### Objective
Implement remaining performance improvements and cross-interface consistency.

### Tasks

#### 3.1 Parallel Multipart Upload (in `raps-oss`)

```rust
// raps-oss/src/upload.rs
pub async fn upload_parallel(
    &self,
    bucket: &BucketKey,
    key: &ObjectKey,
    file: &Path,
    config: UploadConfig,
) -> Result<UploadResult> {
    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    let mut futures = FuturesUnordered::new();
    
    for (i, chunk) in chunks.into_iter().enumerate() {
        let permit = semaphore.clone().acquire_owned().await?;
        futures.push(async move {
            let result = self.upload_chunk(chunk, i).await;
            drop(permit);
            (i, result)
        });
    }
    
    // ... collect results
}
```

#### 3.2 Output Schema Formalization

```rust
// raps-kernel/src/types/output.rs
use schemars::JsonSchema;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ApiResponse<T> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ResponseMeta {
    pub total: Option<usize>,
    pub next_marker: Option<String>,
}
```

#### 3.3 Distribution Satellite Updates

- Update `raps-docker/Dockerfile` to v3.2.0
- Add Windows support to `raps-action`
- Update `homebrew-tap` and `scoop-bucket` formulas

---

## Phase 4: Pro Tier Foundation (Week 9-12)

### Objective
Establish Pro tier infrastructure with complete feature implementation (Analytics, Audit, Compliance, Multi-tenant, SSO).

### Tasks

#### 4.1 Create Pro Crate Structure

```rust
// raps-pro/src/lib.rs
//! RAPS Pro - Enterprise features for production deployments.
//! 
//! License: Commercial (requires subscription)

pub mod analytics;
pub mod audit;
pub mod compliance;
pub mod multitenant;
pub mod sso;
pub mod license;
```

#### 4.2 Usage Analytics (FR-PRO-ANA-*)

```rust
// raps-pro/src/analytics/mod.rs
use std::collections::HashMap;
use chrono::{DateTime, Utc};

pub struct AnalyticsClient {
    storage: AnalyticsStorage,
}

#[derive(Debug, Clone)]
pub struct UsageMetrics {
    pub endpoint: String,
    pub latency_ms: u64,
    pub status_code: u16,
    pub user_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl AnalyticsClient {
    pub fn record_api_call(&self, metrics: UsageMetrics) -> Result<()> { ... }
    pub fn generate_dashboard(&self) -> Result<Dashboard> {
        // Aggregate metrics by endpoint, user, time period
    }
    pub fn generate_report(&self, period: DateRange, format: ReportFormat) -> Result<Vec<u8>> { ... }
    pub fn watch(&self) -> impl Stream<Item = UsageMetrics> {
        // Real-time metrics streaming
    }
    pub fn check_threshold(&self, threshold: UsageThreshold) -> Result<Option<Alert>> { ... }
}
```

#### 4.3 Audit Logging (FR-PRO-AUD-*)

```rust
// raps-pro/src/audit/mod.rs
use sha2::{Sha256, Digest};

pub struct AuditLog {
    storage: AppendOnlyStorage,
    integrity: IntegrityChecker,
}

#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub command: String,
    pub parameters: HashMap<String, String>,  // Secrets redacted
    pub exit_code: i32,
    pub hash: String,  // Integrity hash
}

impl AuditLog {
    pub fn log_action(&self, entry: AuditEntry) -> Result<()> {
        // Append-only write with integrity hash
        let hash = self.compute_hash(&entry)?;
        let entry = AuditEntry { hash, ..entry };
        self.storage.append(&entry)?;
        Ok(())
    }
    
    pub fn search(&self, query: AuditQuery) -> Result<Vec<AuditEntry>> { ... }
    pub fn export(&self, format: ExportFormat) -> Result<Vec<u8>> {
        match format {
            ExportFormat::SiemCef => self.export_cef(),
            ExportFormat::SiemLeef => self.export_leef(),
            ExportFormat::Json => self.export_json(),
        }
    }
    
    pub fn verify_integrity(&self) -> Result<IntegrityResult> {
        // Check all entries have valid hashes
    }
    
    fn compute_hash(&self, entry: &AuditEntry) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_string(entry)?);
        Ok(format!("{:x}", hasher.finalize()))
    }
}
```

#### 4.4 Compliance Policies (FR-PRO-CMP-*)

```rust
// raps-pro/src/compliance/mod.rs
use std::collections::HashMap;

pub struct ComplianceManager {
    policies: Vec<Policy>,
    roles: HashMap<String, Role>,
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub name: String,
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone)]
pub enum PolicyRule {
    AllowCommand { command: String, roles: Vec<String> },
    DenyCommand { command: String },
    RequireClassification { command: String, min_level: Classification },
    RateLimit { command: String, max_per_hour: u32 },
}

impl ComplianceManager {
    pub fn load_policies(&mut self, path: &Path) -> Result<()> { ... }
    pub fn reload(&mut self) -> Result<()> {
        // Hot-reload policies without restart
    }
    
    pub fn check_permission(&self, user: &User, command: &str, params: &HashMap<String, String>) -> Result<()> {
        for policy in &self.policies {
            for rule in &policy.rules {
                match rule {
                    PolicyRule::DenyCommand { command: cmd } if cmd == command => {
                        return Err(RapsError::PolicyViolation(policy.name.clone()));
                    }
                    PolicyRule::AllowCommand { command: cmd, roles } if cmd == command => {
                        if !roles.contains(&user.role) {
                            return Err(RapsError::InsufficientPermissions);
                        }
                    }
                    PolicyRule::RequireClassification { command: cmd, min_level } if cmd == command => {
                        // Check data classification
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
```

#### 4.5 Multi-Tenant Management (FR-PRO-MTN-*)

```rust
// raps-pro/src/multitenant/mod.rs
use std::collections::HashMap;

pub struct TenantManager {
    tenants: HashMap<String, Tenant>,
    current: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub config: TenantConfig,
    pub credentials: Credentials,
    pub last_used: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct TenantConfig {
    pub region: Option<String>,
    pub endpoints: Option<HashMap<String, String>>,
    pub defaults: HashMap<String, String>,
}

impl TenantManager {
    pub fn list(&self) -> Vec<&Tenant> {
        self.tenants.values().collect()
    }
    
    pub fn switch(&mut self, tenant_id: &str) -> Result<()> {
        if !self.tenants.contains_key(tenant_id) {
            return Err(RapsError::TenantNotFound(tenant_id.to_string()));
        }
        self.current = Some(tenant_id.to_string());
        Ok(())
    }
    
    pub fn get_current(&self) -> Option<&Tenant> {
        self.current.as_ref().and_then(|id| self.tenants.get(id))
    }
    
    pub fn get_config(&self, tenant_id: &str) -> Result<&TenantConfig> {
        self.tenants.get(tenant_id)
            .map(|t| &t.config)
            .ok_or_else(|| RapsError::TenantNotFound(tenant_id.to_string()))
    }
    
    pub fn set_config(&mut self, tenant_id: &str, key: String, value: String) -> Result<()> {
        let tenant = self.tenants.get_mut(tenant_id)
            .ok_or_else(|| RapsError::TenantNotFound(tenant_id.to_string()))?;
        tenant.config.defaults.insert(key, value);
        Ok(())
    }
}
```

#### 4.6 Enterprise SSO Integration (FR-PRO-SSO-*)

```rust
// raps-pro/src/sso/mod.rs
use openidconnect::{ClientId, ClientSecret, IssuerUrl, RedirectUrl};
use openidconnect::core::{CoreClient, CoreProviderMetadata, CoreResponseType};
use openidconnect::{AuthorizationCode, CsrfToken, PkceCodeChallenge, Scope};

pub struct SsoClient {
    oidc_client: Option<CoreClient>,
    saml_client: Option<SamlClient>,
    session: Option<SsoSession>,
}

#[derive(Debug, Clone)]
pub struct SsoSession {
    pub identity_token: String,
    pub aps_token: Option<AccessToken>,
    pub expires_at: DateTime<Utc>,
}

impl SsoClient {
    pub async fn configure_oidc(&mut self, config: OidcConfig) -> Result<()> {
        let provider_metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new(config.issuer_url)?,
            async_http_client,
        ).await?;
        
        let client = CoreClient::from_provider_metadata(provider_metadata, 
            ClientId::new(config.client_id),
            Some(ClientSecret::new(config.client_secret)),
        );
        
        self.oidc_client = Some(client);
        Ok(())
    }
    
    pub async fn login(&mut self) -> Result<String> {
        // Open browser for SSO login
        let (auth_url, csrf_token, pkce_challenge) = self.start_authorization()?;
        open::that(&auth_url.to_string())?;
        
        // Wait for callback with authorization code
        let code = self.wait_for_callback(csrf_token).await?;
        
        // Exchange code for identity token
        let token_response = self.exchange_code(code, pkce_challenge).await?;
        
        // Exchange identity token for APS access token
        let aps_token = self.exchange_for_aps_token(token_response.id_token()?).await?;
        
        self.session = Some(SsoSession {
            identity_token: token_response.id_token()?.to_string(),
            aps_token: Some(aps_token),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        });
        
        Ok("SSO login successful".to_string())
    }
    
    pub async fn refresh_session(&mut self) -> Result<()> {
        // Refresh SSO session without user interaction
        if let Some(ref mut session) = self.session {
            if session.expires_at > Utc::now() {
                // Refresh APS token using identity token
                session.aps_token = Some(self.exchange_for_aps_token(&session.identity_token).await?);
            }
        }
        Ok(())
    }
}
```

#### 4.7 License Enforcement

```rust
// raps-pro/src/license.rs
pub struct LicenseManager {
    license: Option<License>,
    server: LicenseServer,
}

#[derive(Debug, Clone)]
pub struct License {
    pub key: String,
    pub features: Vec<ProFeature>,
    pub expires_at: DateTime<Utc>,
    pub seat_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProFeature {
    Analytics,
    Audit,
    Compliance,
    MultiTenant,
    Sso,
}

impl LicenseManager {
    pub async fn validate(&mut self, key: &str) -> Result<License> {
        // Validate against license server
        let license = self.server.validate(key).await?;
        self.license = Some(license.clone());
        Ok(license)
    }
    
    pub fn check_feature(&self, feature: ProFeature) -> Result<()> {
        let license = self.license.as_ref()
            .ok_or_else(|| RapsError::LicenseRequired)?;
        
        if license.expires_at < Utc::now() {
            return Err(RapsError::LicenseExpired);
        }
        
        if !license.features.contains(&feature) {
            return Err(RapsError::FeatureNotLicensed(feature));
        }
        
        Ok(())
    }
}
```

---

## Dependencies & Execution Order

### Phase Dependencies

```
Phase 0 (Kernel) ──────────────────────────────────────────┐
    │                                                       │
    ├── 0.1 Create workspace                               │
    ├── 0.2 Extract kernel components                      │
    ├── 0.3 Define public API                              │
    └── 0.4 Kernel test suite (100% coverage)              │
                                                           │
Phase 1 (Services) ────────────────────────────────────────┤
    │                                                       │
    ├── 1.1 Extract raps-oss (depends on 0.*)              │
    ├── 1.2 Extract raps-derivative                        │
    ├── 1.3 Extract raps-dm (enhanced navigation)          │
    ├── 1.4 Extract raps-ssa (Core tier)                  │
    └── 1.5 Update CLI to use services                     │
                                                           │
Phase 2 (Community) ───────────────────────────────────────┤
    │                                                       │
    ├── 2.1 Extract ACC modules                            │
    ├── 2.2 Extract Design Automation                      │
    ├── 2.3 Extract Plugin system                          │
    ├── 2.4 Extract Account Admin (raps-admin)            │
    ├── 2.5 MCP Server integration (SSA + DM tools)       │
    └── 2.6 CLI commands for Admin/SSA/DM                   │
                                                           │
Phase 3 (Polish) ──────────────────────────────────────────┤
    │                                                       │
    ├── 3.1 Parallel uploads                               │
    ├── 3.2 Output schemas                                 │
    └── 3.3 Distribution updates                           │
                                                           │
Phase 4 (Pro) ─────────────────────────────────────────────┘
    │
    ├── 4.1 Pro crate structure
    ├── 4.2 Usage analytics (FR-PRO-ANA-*)
    ├── 4.3 Audit logging (FR-PRO-AUD-*)
    ├── 4.4 Compliance policies (FR-PRO-CMP-*)
    ├── 4.5 Multi-tenant management (FR-PRO-MTN-*)
    ├── 4.6 Enterprise SSO integration (FR-PRO-SSO-*)
    └── 4.7 License enforcement
```

### Parallel Opportunities

**Within Phase 0**:
- 0.1 and 0.2 must be sequential
- 0.3 and 0.4 can start as soon as structure exists

**Within Phase 1**:
- 1.1, 1.2, 1.3, 1.4 can run in parallel (independent services)
- 1.5 depends on all services being extracted

**Within Phase 2**:
- All ACC modules (2.1) can be extracted in parallel
- 2.2, 2.3, 2.4 can run parallel to 2.1
- 2.5 depends on SSA and DM services (1.4, 1.3)
- 2.6 depends on 2.4 and 1.4

**Within Phase 4**:
- 4.2, 4.3, 4.4 can run in parallel (independent features)
- 4.5, 4.6 can run in parallel
- 4.7 depends on all Pro features for license gating

---

## Verification Checkpoints

### After Phase 0 (Kernel) ✅ COMPLETE
- [x] `raps-kernel` compiles with `#![deny(warnings)]` ✅
- [x] `cargo test -p raps-kernel` passes ✅ (67 tests)
- [x] Kernel LOC < 3000 (target: ~2000) ✅ **1,873 LOC**
- [x] No `unsafe` code in kernel ✅
- [x] No `.unwrap()` or `.expect()` in kernel ✅
- [x] `.cargo/config.toml` configured with lld-link (Windows) and mold (Linux) ✅
- [ ] `cargo check -p raps-kernel` completes in <5s (incremental) ⏳ Needs benchmark
- [ ] `cargo check` full workspace completes in <30s (incremental) ⏳ Needs benchmark
- [x] sccache configured in CI pipelines ✅
- [x] cargo-nextest configured in CI pipelines ✅
- [x] `cargo build --timings` artifact uploaded in CI ✅

### After Phase 1 (Services) ✅ COMPLETE
- [x] All services compile independently ✅
- [x] `cargo test --workspace` passes ✅
- [x] CLI functionality unchanged (regression test) ✅
- [x] Startup time still <100ms ✅

### After Phase 2 (Community) ⚠️ PARTIAL
- [x] Feature flag `--features community` builds ✅
- [ ] MCP server works with tier separation ⏳ Pending
- [ ] Plugin system loads external plugins ⏳ Pending

### After Phase 3 (Polish) ⏳ NOT STARTED
- [ ] 100MB upload <30s with parallel
- [ ] JSON schemas generated and documented
- [ ] All distribution satellites updated

### After Phase 4 (Pro) ⏳ IN PROGRESS
- [x] Feature flag `--features pro` builds ✅
- [ ] License validation works ⏳ Stub only
- [ ] Analytics dashboard generates reports (FR-PRO-ANA-*) ⏳ Pending
- [ ] Audit logs are immutable and searchable (FR-PRO-AUD-*) ⏳ Pending
- [ ] Compliance policies enforce rules (FR-PRO-CMP-*) ⏳ Pending
- [ ] Multi-tenant switching works (FR-PRO-MTN-*) ⏳ Pending
- [ ] SSO login flow completes (FR-PRO-SSO-*) ⏳ Pending

### After Phase 1.4 (SSA) ⏳ NEW
- [ ] SSA service account creation works (FR-SSA-001) ⏳ Pending
- [ ] SSA key generation returns PEM (FR-SSA-002) ⏳ Pending
- [ ] JWT assertion generation (RS256) works (FR-SSA-003) ⏳ Pending
- [ ] Token exchange returns 3LO access token (FR-SSA-004) ⏳ Pending
- [ ] Private keys never logged (FR-SSA-005) ⏳ Pending

### After Phase 2.4 (Account Admin) ⏳ NEW
- [ ] Project listing works (FR-ADMIN-001) ⏳ Pending
- [ ] Project creation works (FR-ADMIN-002) ⏳ Pending
- [ ] User assignment works (FR-ADMIN-003) ⏳ Pending
- [ ] Company management works (FR-ADMIN-004) ⏳ Pending
- [ ] Business units API works (FR-ADMIN-005) ⏳ Pending

### After Phase 1.3 (Data Management Enhanced) ⏳ NEW
- [ ] Hub navigation works (FR-DM-001) ⏳ Pending
- [ ] Project navigation works (FR-DM-002) ⏳ Pending
- [ ] Folder contents listing works (FR-DM-003) ⏳ Pending
- [ ] Item versions listing works (FR-DM-004) ⏳ Pending
- [ ] Tip version retrieval works (FR-DM-005) ⏳ Pending
- [ ] Storage creation for uploads works (FR-DM-006) ⏳ Pending
- [ ] Derivatives URN extraction works (FR-DM-007) ⏳ Pending

### After Phase 2.5 (MCP SSA + DM) ⏳ NEW
- [ ] MCP exposes SSA tools (FR-MCP-005) ⏳ Pending
- [ ] MCP exposes DM tools (FR-MCP-006) ⏳ Pending
- [ ] MCP does NOT expose Account Admin tools (per Q2) ✅ Verified

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Breaking changes during extraction | Comprehensive regression test suite |
| Performance regression | Benchmark suite, CI performance tests |
| License confusion | Clear feature flag documentation |
| Dependency conflicts | Workspace-level dependency management |
| Pro feature leakage | Separate private repository for raps-pro |

---

## Success Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Kernel LOC | <3000 | `tokei raps-kernel/src` |
| Kernel test coverage | >90% | `cargo tarpaulin -p raps-kernel` |
| Upload performance | 6x faster | Benchmark 100MB file |
| Startup time | <100ms | `hyperfine 'raps --help'` |
| Blocking calls | 0 | Clippy + async audit |
| Build time (core) | <30s | `cargo build --features core` |
| Docker image size | <50MB | `docker images` |
| **Kernel check time** | <5s | `hyperfine 'cargo check -p raps-kernel'` (incremental) |
| **Workspace check time** | <30s | `hyperfine 'cargo check'` (incremental) |
| **sccache hit rate** | >80% | `sccache --show-stats` in CI |

---

## Build Tooling Matrix

| Platform | Linker | Cache | Tests | Status |
|----------|--------|-------|-------|--------|
| Windows (local) | lld-link | sccache (opt) | nextest (opt) | ⚠️ Document |
| Windows (CI) | lld-link | sccache | nextest | ✅ Required |
| Linux (local) | mold (opt) | sccache (opt) | nextest (opt) | ⚠️ Document |
| Linux (CI) | mold | sccache | nextest | ✅ Required |
| macOS (local) | default | sccache (opt) | nextest (opt) | ⚠️ Document |
| macOS (CI) | default | sccache | nextest | ✅ Required |

---

## Plan Updates (2025-12-30)

### New Components Added

1. **raps-ssa** (Core tier) - Secure Service Account management
   - Service account CRUD operations
   - Key generation and rotation
   - JWT assertion generation (RS256)
   - Token exchange for 3LO access tokens

2. **raps-admin** (Community tier) - Account Administration
   - Project management (list, create, get)
   - User assignment and management
   - Company directory operations
   - Business units structure

3. **Enhanced raps-dm** - Data Management navigation
   - Hub navigation (list, get)
   - Project navigation (list, get)
   - Folder contents and top folders
   - Item versions and tip version retrieval
   - Storage creation for uploads
   - Derivatives URN extraction

4. **Enhanced raps-pro** - Complete Pro tier implementation
   - Analytics dashboard and reporting (FR-PRO-ANA-*)
   - Audit logging with integrity verification (FR-PRO-AUD-*)
   - Compliance policy enforcement (FR-PRO-CMP-*)
   - Multi-tenant management (FR-PRO-MTN-*)
   - Enterprise SSO integration (FR-PRO-SSO-*)

### MCP Coverage Updates

- **Added**: SSA tools (FR-MCP-005) - `ssa_create`, `ssa_list`, `ssa_key_create`, `ssa_token`
- **Added**: Data Management tools (FR-MCP-006) - `dm_hubs_list`, `dm_projects_list`, `dm_folders_list`, `dm_versions_list`, `dm_tip_version`
- **Excluded**: Account Admin tools (per Q2 clarification - less frequent operations)

### Phase Updates

- **Phase 1**: Added 1.4 (SSA extraction) and 1.5 (CLI updates)
- **Phase 2**: Added 2.4 (Account Admin), 2.5 (MCP SSA+DM), 2.6 (CLI commands)
- **Phase 4**: Expanded from 4 tasks to 7 tasks covering all Pro tier features

### Dependencies Added

- `jsonwebtoken` v9.3 - For SSA JWT assertion generation
- `sha2` v0.10 - For audit log integrity hashing
- `openidconnect` v5.0 - For Pro tier SSO support
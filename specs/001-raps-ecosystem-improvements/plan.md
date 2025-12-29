# Implementation Plan: RAPS Ecosystem Improvements

**Branch**: `001-raps-ecosystem-improvements` | **Date**: 2025-12-29 | **Updated**: 2025-12-29 | **Spec**: [spec.md](spec.md)  
**Status**: In Progress (~45% Complete)  
**Input**: Feature specification from `/specs/001-raps-ecosystem-improvements/spec.md`

## Summary

Comprehensive improvements to the RAPS (Rust Autodesk Platform Services) ecosystem addressing:
- **Microkernel Architecture**: Refactor to Unix-like kernel design for performance, security, and testability
- **Tiered Product Strategy**: Core → Community → Pro evolution path
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
| **TUI** | ❌ | ✅ | ✅ |
| **Enterprise Features** | ❌ | ❌ | ✅ |
| Usage Analytics | ❌ | ❌ | ✅ |
| Audit Logs | ❌ | ❌ | ✅ |
| Compliance Reports | ❌ | ❌ | ✅ |
| Multi-tenant Mgmt | ❌ | ❌ | ✅ |
| Priority Rate Limits | ❌ | ❌ | ✅ |
| Dedicated Support | ❌ | ❌ | ✅ |

---

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Check | Status |
|-----------|-------|--------|
| I. Rust Idiomatic Code Quality | All changes pass clippy, rustfmt, no unwrap in lib | ✅ Required |
| II. Cross-Repository Consistency | Shared crate extraction, version alignment | ✅ Addressed |
| III. Test-First Development | Tests before implementation for each phase | ✅ Required |
| IV. User Experience Consistency | Output schema formalization, error message consistency | ✅ Addressed |
| V. Performance & Resource Efficiency | Parallel uploads, async fixes, memory bounds | ✅ Primary focus |
| VI. Security & Secrets Handling | Container credential handling, secret redaction | ✅ Addressed |

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
├── contracts/           # API schemas (pending)
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

raps-dm/                             # ★ Data Management Service
├── Cargo.toml                       # Depends on raps-kernel
├── src/
│   ├── lib.rs
│   ├── hub.rs
│   ├── project.rs
│   ├── folder.rs
│   └── item.rs
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
    "raps-oss",
    "raps-derivative",
    "raps-dm",
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
```

### Feature Flags for Tier Selection

```toml
# raps/Cargo.toml
[package]
name = "raps"

[features]
default = ["community"]
core = ["raps-kernel", "raps-oss", "raps-derivative", "raps-dm"]
community = ["core", "raps-community"]
pro = ["community", "raps-pro"]

[dependencies]
raps-kernel = { path = "../raps-kernel" }
raps-oss = { path = "../raps-oss" }
raps-derivative = { path = "../raps-derivative" }
raps-dm = { path = "../raps-dm" }
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
    pub async fn list_projects(&self, hub_id: &str) -> Result<Vec<Project>> { ... }
    pub async fn list_folders(&self, project_id: &str, folder_id: &str) -> Result<Vec<Folder>> { ... }
}
```

#### 1.4 Update CLI to Use Services

```rust
// raps/src/commands/bucket.rs
use raps_oss::OssClient;

pub async fn handle_bucket_list(client: &OssClient, region: Option<String>) -> Result<()> {
    let buckets = client.list_buckets(region.as_deref()).await?;
    output::render(buckets)?;
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

#### 2.4 MCP Server Integration

```rust
// raps/src/mcp/server.rs
use raps_kernel::*;
use raps_oss::OssClient;
use raps_derivative::DerivativeClient;
#[cfg(feature = "community")]
use raps_community::*;

pub struct McpServer {
    kernel: Arc<Kernel>,
    oss: OssClient,
    derivative: DerivativeClient,
    #[cfg(feature = "community")]
    issues: IssuesClient,
    // ...
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

## Phase 4: Pro Tier Foundation (Week 9-10)

### Objective
Establish Pro tier infrastructure (proprietary crate).

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
```

#### 4.2 Usage Analytics

```rust
// raps-pro/src/analytics/mod.rs
pub struct AnalyticsClient {
    // Track command usage, API latency, error rates
}

impl AnalyticsClient {
    pub fn record_command(&self, cmd: &str, duration: Duration, exit_code: i32) { ... }
    pub fn generate_report(&self, period: DateRange) -> Result<UsageReport> { ... }
}
```

#### 4.3 Audit Logging

```rust
// raps-pro/src/audit/mod.rs
pub struct AuditLog {
    // Immutable append-only log for compliance
}

impl AuditLog {
    pub fn log_action(&self, action: AuditAction) -> Result<()> { ... }
    pub fn export(&self, format: ExportFormat) -> Result<Vec<u8>> { ... }
}
```

#### 4.4 License Enforcement

```rust
// raps-pro/src/license.rs
pub fn validate_license(key: &str) -> Result<License> {
    // Validate license key against server
    // Check expiration, seat count, features
}

pub fn check_feature(license: &License, feature: ProFeature) -> bool {
    license.features.contains(&feature)
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
    ├── 1.3 Extract raps-dm                                │
    └── 1.4 Update CLI to use services                     │
                                                           │
Phase 2 (Community) ───────────────────────────────────────┤
    │                                                       │
    ├── 2.1 Extract ACC modules                            │
    ├── 2.2 Extract Design Automation                      │
    ├── 2.3 Extract Plugin system                          │
    └── 2.4 MCP Server integration                         │
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
    ├── 4.2 Usage analytics
    ├── 4.3 Audit logging
    └── 4.4 License enforcement
```

### Parallel Opportunities

**Within Phase 0**:
- 0.1 and 0.2 must be sequential
- 0.3 and 0.4 can start as soon as structure exists

**Within Phase 1**:
- 1.1, 1.2, 1.3 can run in parallel (independent services)
- 1.4 depends on all services being extracted

**Within Phase 2**:
- All ACC modules (2.1) can be extracted in parallel
- 2.2, 2.3 can run parallel to 2.1

**Within Phase 4**:
- 4.2, 4.3 can run in parallel (independent features)

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

### After Phase 4 (Pro) ⚠️ PARTIAL
- [x] Feature flag `--features pro` builds ✅
- [ ] License validation works ⏳ Stub only
- [ ] Analytics/audit logs generated ⏳ Stub only

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
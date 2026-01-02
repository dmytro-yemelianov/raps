# Implementation Plan: Monorepo Microkernel App

**Branch**: `002-monorepo-microkernel-app` | **Date**: 2026-01-01 | **Spec**: [spec.md](spec.md)  
**Status**: Planning  
**Input**: Feature specification from `/specs/002-monorepo-microkernel-app/spec.md`

## Summary

Implementation of a monorepo microkernel architecture for RAPS (Rust Autodesk Platform Services CLI) that consolidates kernel, service modules, community features, and pro features into a single Rust workspace. The microkernel is designed to be **fast** (<5s incremental check), **secure** (no unsafe code, comprehensive secret redaction, HTTPS-only), and **minimal** (<3000 LOC) while providing a trusted foundation for all APS API interactions.

**Key Technical Approach:**
- Rust workspace with 8 crates: kernel, 4 service modules, 2 tier crates, CLI binary
- Microkernel architecture with strict dependency boundaries (kernel → services → tiers → CLI)
- Performance-optimized build configuration (sccache, lld-link/mold, debug=0)
- Security-first kernel design (deny unsafe_code, deny warnings, deny unwrap_used)

## Technical Context

**Language/Version**: Rust 1.88+ (Edition 2021)  
**Primary Dependencies**: 
- `tokio` 1.35 (async runtime)
- `reqwest` 0.11 (HTTP client with TLS)
- `serde` 1.0 (serialization)
- `thiserror` 1.0 (error types)
- `anyhow` 1.0 (application errors)
- `keyring` (secure credential storage)
- `tracing` (structured logging)

**Storage**: File-based config (TOML) + platform keyring for secrets  
**Testing**: `cargo nextest` (parallel test execution), `assert_cmd` (CLI testing), `predicates` (assertions)  
**Target Platform**: Windows (x64), macOS (x64, arm64), Linux (x64, arm64)  
**Project Type**: Multi-crate Rust workspace (monorepo)  
**Performance Goals**: 
- Kernel incremental check: <5s
- Full workspace incremental check: <30s
- CLI startup: <100ms
- Memory usage: <256MB peak
- Binary size: <15MB release

**Constraints**: 
- MSRV: Rust 1.88
- Kernel: <3000 LOC (target ~2000 LOC)
- No unsafe code in kernel
- No blocking I/O (all async)
- HTTPS-only for APS API communication

**Scale/Scope**: 
- 8 crates in workspace
- 50+ CLI commands
- 3 product tiers (Core, Community, Pro)
- Cross-platform support (Windows, macOS, Linux)

---

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### ✅ Principle I: Rust Idiomatic Code Quality
- **Zero Warnings**: Kernel uses `#![deny(warnings)]` - PASS
- **Error Handling**: `thiserror` for library errors, `anyhow` for application - PASS
- **Type Safety**: Newtype patterns for domain concepts (Urn, BucketKey, etc.) - PASS
- **Documentation**: All public APIs have rustdoc comments - PASS

### ✅ Principle II: Cross-Repository Consistency
- **Monorepo Structure**: All core components in single `raps` repository - PASS
- **Shared Versioning**: Workspace-level version management - PASS
- **Unified Error Codes**: Canonical error definitions in kernel - PASS

### ✅ Principle III: Test-First Development
- **Test Categories**: Unit (colocated), integration (tests/), contract (API validation) - PASS
- **Coverage**: Kernel >90% on critical paths - PASS
- **CI Gate**: `cargo nextest run` required before merge - PASS

### ✅ Principle IV: User Experience Consistency
- **Output Formats**: `--output {table,json,yaml,csv,plain}` - PASS
- **Error Messages**: Actionable format with remediation steps - PASS
- **Exit Codes**: Standardized codes (0=success, 2=args, 3=auth, etc.) - PASS

### ✅ Principle V: Performance & Resource Efficiency
- **Build Speed**: Kernel <5s, workspace <30s (incremental) - PASS
- **Startup Time**: <100ms for help/version commands - PASS
- **Memory Bounds**: <256MB peak usage - PASS
- **Build Tooling**: sccache, lld-link (Windows), mold (Linux) - PASS

### ✅ Principle VI: Security & Secrets Handling
- **Secret Storage**: Platform keyring via `keyring` crate - PASS
- **Secret Redaction**: All logging redacts tokens/keys - PASS
- **HTTPS Only**: All APS API communication uses TLS - PASS

### ✅ Principle VII: Microkernel Architecture
- **Monorepo Structure**: All crates in single workspace - PASS
- **Kernel Isolation**: Only Auth, HTTP, Config, Storage, Types, Error, Logging - PASS
- **Kernel Constraints**: `deny(warnings)`, `deny(unsafe_code)`, `deny(clippy::unwrap_used)` - PASS
- **Kernel Size**: <3000 LOC target - PASS
- **Service Independence**: Services depend only on kernel - PASS
- **Tier Separation**: Tiers depend only on kernel and services - PASS
- **No Blocking**: All I/O async - PASS

**Constitution Compliance**: ✅ ALL PRINCIPLES SATISFIED

---

## Project Structure

### Documentation (this feature)

```text
specs/002-monorepo-microkernel-app/
├── plan.md              # This file (/speckit.plan command output)
├── spec.md              # Feature specification
├── research.md          # Phase 0 output (to be created)
├── data-model.md        # Phase 1 output (to be created)
├── quickstart.md        # Phase 1 output (to be created)
├── contracts/           # Phase 1 output (to be created)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
raps/ (monorepo workspace root)
├── Cargo.toml                    # Workspace configuration
├── Cargo.lock                    # Locked dependency versions
├── .cargo/
│   └── config.toml               # Linker configuration (lld-link, mold)
│
├── raps-kernel/                  # Microkernel foundation crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                # Library root with deny attributes
│   │   ├── auth/                 # OAuth2 authentication
│   │   │   ├── mod.rs
│   │   │   ├── oauth2.rs         # 2-legged, 3-legged OAuth
│   │   │   └── device_code.rs     # Device code flow
│   │   ├── http/                 # HTTP client with retry
│   │   │   ├── mod.rs
│   │   │   ├── client.rs         # Async HTTP client wrapper
│   │   │   └── retry.rs          # Exponential backoff retry logic
│   │   ├── config/               # Configuration management
│   │   │   ├── mod.rs
│   │   │   └── config.rs         # TOML config parsing
│   │   ├── storage/              # Secure credential storage
│   │   │   ├── mod.rs
│   │   │   └── keyring.rs        # Platform keyring integration
│   │   ├── types/                # Domain primitives
│   │   │   ├── mod.rs
│   │   │   ├── urn.rs            # URN newtype
│   │   │   ├── bucket_key.rs     # Bucket key newtype
│   │   │   └── region.rs         # Region enum
│   │   ├── error.rs              # Error types with exit codes
│   │   └── logging.rs            # Tracing & secret redaction
│   └── tests/
│       └── integration_test.rs
│
├── raps-oss/                     # Object Storage Service crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── bucket.rs             # Bucket CRUD operations
│   │   ├── object.rs              # Object CRUD operations
│   │   └── upload.rs              # Parallel multipart uploads
│   └── tests/
│
├── raps-derivative/              # Model Derivative Service crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── translation.rs        # Translation job management
│   │   ├── manifest.rs            # Manifest queries
│   │   └── download.rs            # Derivative downloads
│   └── tests/
│
├── raps-dm/                      # Data Management Service crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── hub.rs                 # Hub operations
│   │   ├── project.rs             # Project operations
│   │   └── item.rs                # Folder/item operations
│   └── tests/
│
├── raps-ssa/                     # Secure Service Accounts crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── robot.rs               # Robot account management
│   │   └── jwt.rs                 # JWT assertion exchange
│   └── tests/
│
├── raps-community/               # Community tier features crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── account_admin.rs       # Account Admin API
│   │   ├── acc.rs                 # ACC modules (Issues, RFIs, etc.)
│   │   ├── design_automation.rs   # Design Automation API
│   │   └── reality_capture.rs    # Reality Capture API
│   └── tests/
│
├── raps-pro/                     # Pro tier features crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── analytics.rs           # Usage analytics
│   │   ├── audit.rs                # Audit logs
│   │   └── compliance.rs          # Compliance policies
│   └── tests/
│
└── raps/                         # CLI binary crate
    ├── Cargo.toml
    ├── src/
    │   ├── main.rs                # CLI entry point
    │   ├── commands/              # Command handlers
    │   │   ├── mod.rs
    │   │   ├── auth.rs
    │   │   ├── bucket.rs
    │   │   ├── object.rs
    │   │   └── ...
    │   └── output/                # Output formatting
    │       ├── mod.rs
    │       ├── table.rs
    │       ├── json.rs
    │       └── yaml.rs
    └── tests/
        └── integration/
```

**Structure Decision**: Multi-crate Rust workspace organized by architectural layer (kernel → services → tiers → CLI). Each crate is independently testable and buildable while maintaining strict dependency boundaries. Kernel is minimal and secure; services extend functionality; tiers add product features; CLI provides user interface.

---

## Microkernel Architecture Design

### Kernel Design Principles

The `raps-kernel` crate is the **trusted foundation** with these design principles:

1. **Minimal Surface Area**: Only essential services (auth, HTTP, config, storage, types, error, logging)
2. **Zero External Dependencies on Services**: Kernel has no knowledge of OSS, Derivative, DM, or SSA
3. **Security First**: 
   - `#![deny(unsafe_code)]` - No unsafe blocks
   - `#![deny(clippy::unwrap_used)]` - No unwrap/expect calls
   - `#![deny(warnings)]` - Zero warnings policy
   - Secret redaction in all logging
   - HTTPS-only for all network operations
4. **Performance Optimized**:
   - Async I/O only (no blocking)
   - Lazy initialization for optional features
   - Minimal allocations in hot paths
   - Streaming APIs for large data
5. **Auditable**: <3000 LOC, >90% test coverage, comprehensive documentation

### Kernel Module Breakdown

| Module | LOC Estimate | Purpose | Dependencies |
|--------|--------------|---------|--------------|
| `auth/` | ~400 | OAuth2 flows (2-legged, 3-legged, device code) | reqwest, serde, thiserror |
| `http/` | ~300 | HTTP client wrapper with retry logic | reqwest, tokio |
| `config/` | ~200 | TOML config parsing and validation | toml, serde |
| `storage/` | ~250 | Platform keyring integration | keyring |
| `types/` | ~300 | Domain primitives (Urn, BucketKey, etc.) | serde |
| `error.rs` | ~200 | Error types with exit codes | thiserror |
| `logging.rs` | ~150 | Tracing setup and secret redaction | tracing |
| **Total** | **~2000** | | |

### Service Module Design

Service crates (`raps-oss`, `raps-derivative`, `raps-dm`, `raps-ssa`) follow these patterns:

1. **Single Responsibility**: Each service implements one APS API domain
2. **Kernel Dependency Only**: Services depend only on `raps-kernel`, not each other
3. **Async First**: All operations are async, returning `Result<T, RapsError>`
4. **Type Safety**: Use kernel types (Urn, BucketKey) and extend with service-specific types

### Tier Crate Design

Tier crates (`raps-community`, `raps-pro`) aggregate features:

1. **Feature Aggregation**: Combine multiple service APIs into cohesive features
2. **Tier Boundaries**: Community tier depends on kernel + services; Pro tier adds enterprise features
3. **Feature Flags**: Use Cargo features to enable/disable tiers

---

## Performance Optimization Strategy

### Build Performance

**Target**: Kernel incremental check <5s, workspace incremental check <30s

**Techniques**:
1. **sccache**: Compilation caching across builds
   ```toml
   # .cargo/config.toml
   [build]
   rustc-wrapper = "sccache"
   ```

2. **Fast Linkers**: 
   - Windows: `lld-link` (via LLVM)
   - Linux: `mold` (fastest linker)
   ```toml
   [target.x86_64-pc-windows-msvc]
   linker = "lld-link"
   
   [target.x86_64-unknown-linux-gnu]
   linker = "mold"
   ```

3. **Reduced Debug Info**:
   ```toml
   [profile.dev]
   debug = 0  # Reduces PDB overhead on Windows
   
   [profile.test]
   debug = 0
   ```

4. **Incremental Compilation**: Enabled by default, verify with `CARGO_INCREMENTAL=1`

### Runtime Performance

**Target**: CLI startup <100ms, memory <256MB peak

**Techniques**:
1. **Lazy Initialization**: Don't initialize HTTP client until first use
2. **Streaming APIs**: Use `reqwest::Body::wrap_stream` for large uploads
3. **Connection Pooling**: Reuse HTTP connections via `reqwest::Client`
4. **Minimal Allocations**: Use `Cow<str>` for string operations where possible

---

## Security Hardening

### Kernel Security Measures

1. **No Unsafe Code**: `#![deny(unsafe_code)]` enforced at compile time
2. **No Unwrap Calls**: `#![deny(clippy::unwrap_used)]` prevents panics
3. **Secret Redaction**: All logging automatically redacts tokens, keys, credentials
4. **HTTPS Only**: Reject non-HTTPS endpoints at runtime
5. **Minimal OAuth Scopes**: Request only required scopes per operation
6. **Secure Storage**: Use platform keyring (Windows Credential Manager, macOS Keychain, Linux Secret Service)

### Secret Redaction Implementation

```rust
// raps-kernel/src/logging.rs
pub fn redact_secrets(text: &str) -> String {
    // Redact OAuth tokens, API keys, credentials
    // Pattern: /token|key|secret|password|credential/i
    // Replace with: "***REDACTED***"
}
```

---

## Workspace Configuration

### Root Cargo.toml Structure

```toml
[workspace]
members = [
    "raps-kernel",
    "raps-oss",
    "raps-derivative",
    "raps-dm",
    "raps-ssa",
    "raps-community",
    "raps-pro",
    "raps",
]

resolver = "2"

[workspace.package]
version = "3.3.0"
edition = "2021"
authors = ["RAPS Contributors"]
license = "Apache-2.0"
repository = "https://github.com/dmytro-yemelianov/raps"
homepage = "https://rapscli.xyz"
documentation = "https://docs.rapscli.xyz"

[workspace.dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }

# HTTP client
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Security
keyring = "2.0"

# CLI
clap = { version = "4.4", features = ["derive"] }

[workspace.lints.rust]
unsafe_code = "deny"
missing_docs = "warn"

[profile.dev]
debug = 0
opt-level = 0
incremental = true

[profile.test]
debug = 0
opt-level = 0

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
```

### Kernel Cargo.toml Example

```toml
[package]
name = "raps-kernel"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
documentation.workspace = true

[dependencies]
tokio.workspace = true
reqwest.workspace = true
serde.workspace = true
serde_json.workspace = true
toml.workspace = true
thiserror.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
keyring.workspace = true

[lib]
name = "raps_kernel"

[features]
default = []
```

---

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: CI

on: [push, pull_request]

jobs:
  check:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          override: true
      
      # Setup sccache
      - uses: mozilla-actions/sccache-action@v0.0.3
      
      # Setup fast linker (Linux)
      - name: Setup mold (Linux)
        if: matrix.os == 'ubuntu-latest'
        run: |
          curl -L https://github.com/rui314/mold/releases/download/v2.0.0/mold-2.0.0-x86_64-linux.tar.gz | tar xz
          echo "$PWD/mold-2.0.0-x86_64-linux/bin" >> $GITHUB_PATH
      
      # Check entire workspace
      - run: cargo check --workspace --all-targets
      
      # Lint
      - run: cargo clippy --workspace -- -D warnings
      
      # Format check
      - run: cargo fmt --check --all
      
      # Test
      - run: cargo nextest run --workspace
      
      # Build release
      - run: cargo build --release --workspace
```

---

## Migration Strategy

### Phase 1: Workspace Setup (Week 1)
- Create workspace `Cargo.toml` with all members
- Configure workspace dependencies
- Setup `.cargo/config.toml` for fast linkers
- Configure build profiles (debug=0, sccache)

### Phase 2: Kernel Migration (Week 2)
- Move kernel code into `raps-kernel/` crate
- Apply security constraints (`deny` attributes)
- Ensure kernel compiles with <3000 LOC
- Achieve >90% test coverage

### Phase 3: Service Modules Migration (Week 3-4)
- Move OSS, Derivative, DM, SSA into separate crates
- Update dependencies to use workspace references
- Verify no circular dependencies
- Test individual crate builds

### Phase 4: Tier Crates Migration (Week 5)
- Move Community and Pro features into tier crates
- Configure feature flags
- Verify tier boundaries (no cross-tier dependencies)

### Phase 5: CLI Integration (Week 6)
- Update CLI binary to depend on workspace crates
- Verify full workspace builds successfully
- Test all CLI commands end-to-end

### Phase 6: CI/CD Consolidation (Week 7)
- Update GitHub Actions to validate entire workspace
- Configure sccache in CI
- Setup fast linkers in CI
- Verify build performance targets

### Phase 7: Documentation & Cleanup (Week 8)
- Update README with monorepo structure
- Document workspace commands
- Archive separate repositories (if they exist)
- Update distribution repos to reference monorepo

---

## Complexity Tracking

> **No Constitution violations** - All principles satisfied with monorepo structure.

The monorepo structure enables:
- Atomic changes across architectural layers
- Unified dependency management
- Shared tooling and CI/CD
- Clear architectural boundaries through crate dependencies

**Simpler Alternative Rejected**: Multi-repo structure was rejected because:
- Cross-repo changes require coordination and can lead to inconsistency
- Dependency version conflicts across repositories
- Duplicate CI/CD configuration
- Slower developer feedback loops

---

## Next Steps

1. **Phase 0 Research**: Analyze current repository structure and identify migration paths
2. **Phase 1 Design**: Create data models, API contracts, and quickstart guide
3. **Phase 2 Tasks**: Generate detailed task list from user stories and requirements

**Ready for**: `/speckit.tasks` command to generate implementation tasks

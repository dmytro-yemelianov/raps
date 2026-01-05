# Research: RAPS Ecosystem Improvements

**Feature**: 001-raps-ecosystem-improvements  
**Date**: 2025-12-29  
**Updated**: 2025-12-29  
**Status**: Complete (Microkernel architecture now implemented)

## Current Ecosystem State

### Repository Structure

```
Autodesk APS/
├── raps-kernel/             # ★ Microkernel (1,873 LOC) - NEW
│   └── src/
│       ├── auth/           # OAuth2 authentication
│       ├── http/           # HTTP client with retry
│       ├── config/         # Configuration management
│       ├── storage/        # Secure credential storage
│       ├── types/          # Domain primitives
│       ├── error.rs        # Error types with exit codes
│       └── logging.rs      # Tracing & secret redaction
├── raps-oss/                # ★ OSS Service - NEW
├── raps-derivative/         # ★ Model Derivative Service - NEW
├── raps-dm/                 # ★ Data Management Service - NEW
├── raps-community/          # ★ Extended Features Features - NEW
├── raps-enterprise/                # ★ Enterprise Tier (stubs) - NEW
├── raps/                    # Core CLI (Rust) - v3.2.0
│   ├── src/
│   │   ├── api/            # API adapters (wrapping service crates)
│   │   ├── commands/       # CLI command handlers
│   │   ├── mcp/            # MCP server implementation
│   │   └── ...
│   ├── tests/              # Integration tests
│   └── issues/             # 15 tracked improvement issues
├── aps-tui/                 # Terminal UI (Rust) - early development
├── aps-wasm-demo/           # WebAssembly demo
├── raps-action/             # GitHub Action (composite)
├── raps-docker/             # Docker container
├── raps-website/            # Documentation site (Astro)
├── aps-sdk-openapi/         # APS OpenAPI specifications
├── homebrew-tap/            # Homebrew formula
└── scoop-bucket/            # Scoop manifest
```

### Version History

| Version | Date | Key Changes |
|---------|------|-------------|
| 3.1.0 | 2025-12-29 | Critical bug fix (auth dispatch), command tests, Rust 2024 |
| 3.0.0 | 2025-12-27 | MCP server with 14 tools |
| 2.1.0 | 2025-12-26 | Rapeseed branding, rapscli.xyz website |
| 2.0.0 | 2025-12-25 | Apache 2.0 license, documentation overhaul |
| 1.0.0 | 2025-12-25 | First stable release, full ACC CRUD, plugins |
| 0.7.0 | 2025-12-25 | Multipart uploads, translation presets, DA activities |

---

## Known Issues Analysis

### Performance Issues (7 issues)

#### Issue #70: Parallel Multipart Upload (HIGH)

**Current Code** (`src/api/oss.rs`):
```rust
// Sequential chunk upload
for (i, chunk) in chunks.iter().enumerate() {
    let part_number = i + 1;
    // ... upload one at a time
}
```

**Problem**: 100MB file with 5MB chunks = 20 sequential HTTP requests. Network RTT dominates.

**Solution**: Use `FuturesUnordered` for concurrent uploads:
```rust
use futures::stream::FuturesUnordered;
use futures::StreamExt;

let mut uploads = FuturesUnordered::new();
for (i, chunk) in chunks.iter().enumerate() {
    let permit = semaphore.acquire().await?;
    uploads.push(async move {
        let result = upload_chunk(chunk, i).await;
        drop(permit);
        result
    });
}
while let Some(result) = uploads.next().await {
    result?;
}
```

#### Issue #73: Blocking Async (MEDIUM)

**Affected Files**:
- `src/api/auth.rs` - `tiny_http::Server::recv()` blocks Tokio runtime
- `src/commands/object.rs` - `dialoguer::Select::interact()` blocks

**Solution**: Wrap with `spawn_blocking`:
```rust
let selection = tokio::task::spawn_blocking(move || {
    Select::new().items(&items).interact()
}).await??;
```

#### Issue #74: Serial Pagination (MEDIUM)

**Current Behavior**: Lists fetch all pages before returning any data.

**Solution**: Implement streaming with `--limit` flag:
```rust
pub fn list_objects_stream(&self) -> impl Stream<Item = Object> {
    stream::unfold(None, |marker| async move {
        let page = self.list_page(marker).await.ok()?;
        Some((stream::iter(page.items), page.next_marker))
    }).flatten()
}
```

### Architecture Issues (4 issues)

#### Issue #76: Hardcoded URLs (HIGH)

**Current**: URLs scattered across API clients:
```rust
// src/api/oss.rs
const BASE_URL: &str = "https://developer.api.autodesk.com/oss/v2";

// src/api/derivative.rs
const BASE_URL: &str = "https://developer.api.autodesk.com/modelderivative/v2";
```

**Solution**: Centralize in config:
```rust
// src/config.rs
pub struct ApsEndpoints {
    pub oss: String,
    pub derivative: String,
    pub auth: String,
    // ...
}

impl Default for ApsEndpoints {
    fn default() -> Self {
        Self {
            oss: "https://developer.api.autodesk.com/oss/v2".into(),
            derivative: "https://developer.api.autodesk.com/modelderivative/v2".into(),
            // ...
        }
    }
}
```

#### Issue #77: Inconsistent Retry Logic (HIGH)

**Current**: Retry logic duplicated with variations:
- `http.rs` has generic retry
- Some clients have custom retry
- Some have no retry

**Solution**: Unified retry middleware:
```rust
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub retryable_codes: Vec<StatusCode>,
}

impl HttpClient {
    pub async fn request_with_retry<T>(&self, req: Request) -> Result<T> {
        self.retry_policy.execute(|| self.send(req.clone())).await
    }
}
```

#### Issue #79: Output Schema Formalization (LOW)

**Current**: Ad-hoc JSON structures per command.

**Solution**: Define schemas with `schemars`:
```rust
use schemars::JsonSchema;

#[derive(Serialize, JsonSchema)]
pub struct BucketListOutput {
    pub buckets: Vec<BucketInfo>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_marker: Option<String>,
}
```

Generate schema docs:
```bash
raps schema bucket-list > docs/schemas/bucket-list.json
```

---

## Component Analysis

### CLI (raps crate)

**Strengths**:
- Comprehensive APS coverage (14 API domains)
- Multiple output formats (json, yaml, csv, table, plain)
- Profile management with secure storage
- Shell completions for all major shells
- Plugin system with aliases and hooks

**Weaknesses**:
- Sequential multipart uploads (4x slower than optimal)
- Blocking calls in async runtime
- Inconsistent retry/timeout handling
- No output schema documentation

**Dependencies** (from Cargo.toml):
```toml
clap = "4.4"          # CLI framework
reqwest = "0.11"      # HTTP client (rustls-tls)
tokio = "1.35"        # Async runtime
serde = "1.0"         # Serialization
rmcp = "0.12"         # MCP server SDK
keyring = "2.3"       # Secure credential storage
```

### MCP Server (raps serve)

**Current Tools** (14):
- `auth_test`, `auth_status`
- `bucket_list`, `bucket_create`, `bucket_get`, `bucket_delete`
- `object_list`, `object_delete`, `object_signed_url`, `object_urn`
- `translate_start`, `translate_status`
- `hub_list`, `project_list`

**Missing from CLI**:
- Object upload (complex due to streaming)
- Folder operations
- Issue/RFI operations
- Webhook management
- Design Automation

**Architecture**:
```rust
// src/mcp/server.rs
pub struct RapsServer {
    tools: McpToolRegistry,
    aps_client: Arc<Mutex<ApsClient>>,
}

// src/mcp/tools.rs
#[tool(description = "List OSS buckets")]
async fn bucket_list(&self, region: Option<String>) -> McpResult<Vec<Bucket>> {
    let client = self.aps_client.lock().await;
    client.oss.list_buckets(region).await
}
```

### GitHub Action (raps-action)

**Current Capabilities**:
- Install RAPS from GitHub releases
- Run arbitrary RAPS commands
- Support Linux/macOS runners
- Pass credentials via environment

**Limitations**:
- No Windows support (bash-only scripts)
- No binary caching across runs
- Limited output parsing

**Improvement Opportunities**:
1. Add Windows PowerShell scripts
2. Use `actions/cache` for binary
3. Parse JSON output into step outputs

### Docker Container (raps-docker)

**Current State**:
- Based on `debian:bookworm-slim`
- Single binary installation
- Non-root user for security
- Hardcoded to v2.0.0 (outdated!)

**Dockerfile Analysis**:
```dockerfile
ARG VERSION=2.0.0  # ⚠️ Should be 3.1.0
```

**Improvements Needed**:
1. Update to current version (3.1.0)
2. Add ARG for build-time version injection
3. Add HEALTHCHECK instruction
4. Create slim variant (<50MB)

### TUI (aps-tui)

**Current State**: Basic skeleton with:
- Event loop (tick/key/mouse/resize)
- Crossterm/ratatui integration
- Panel structure (WIP)

**Code Architecture**:
```rust
mod app;      // Application state
mod api;      // API clients (duplicated from raps!)
mod config;   // Configuration loading
mod ui;       // UI rendering
mod event;    // Event handling
```

**Critical Issue**: API code duplicated from `raps` crate. Need to extract shared library.

---

## Dependency Analysis

### Shared Dependencies Across Repos

| Crate | raps | aps-tui | Purpose |
|-------|------|---------|---------|
| tokio | 1.35 | 1.x | Async runtime |
| reqwest | 0.11 | 0.11 | HTTP client |
| serde | 1.0 | 1.0 | Serialization |
| anyhow | 1.0 | 1.0 | Error handling |
| dotenvy | 0.15 | 0.15 | .env loading |

### Recommended: Extract `raps-core` Crate

```
raps-core/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── api/           # All API clients
    ├── auth/          # Authentication logic
    ├── config/        # Configuration management
    ├── error.rs       # Shared error types
    └── types.rs       # Domain types (Urn, BucketKey, etc.)
```

Both `raps` and `aps-tui` would depend on `raps-core`:
```toml
[dependencies]
raps-core = { path = "../raps-core" }
```

---

## Performance Benchmarks

### Current Measurements (v3.1.0)

| Operation | Time | Notes |
|-----------|------|-------|
| `raps --help` | 78ms | Cold start |
| `raps auth test` | 287ms | 2-legged OAuth |
| `raps bucket list` | 1.2s | ~50 buckets |
| Upload 5MB file | 8s | Single chunk |
| Upload 100MB file | 127s | 20 sequential chunks |
| `raps serve` startup | 45ms | MCP server ready |

### Target Performance (Constitution)

| Operation | Target | Current | Gap |
|-----------|--------|---------|-----|
| CLI startup | <100ms | 78ms | ✅ |
| Auth test | <500ms | 287ms | ✅ |
| 100MB upload | <30s | 127s | ❌ 4.2x |
| MCP startup | <100ms | 45ms | ✅ |

### Upload Performance Model

Sequential (current):
```
Time = chunks × (upload_time + RTT)
     = 20 × (3s + 200ms)
     = 64s theoretical (127s actual due to overhead)
```

Parallel (target, 5 concurrent):
```
Time = ceil(chunks / 5) × max(upload_time, RTT) + overhead
     = 4 × 3.2s + 2s
     = 14.8s theoretical (~20s practical)
```

Expected improvement: **6x faster** for large files.

---

## Security Considerations

### Current Implementation

1. **Credential Storage**: Keyring (platform-native) with file fallback
2. **Secret Redaction**: Regex-based redaction in debug output
3. **HTTPS**: Enforced via reqwest with rustls-tls
4. **OAuth Scopes**: Minimal scopes requested per command

### Recommendations

1. **Docker Credentials**: Ensure `RAPS_NO_KEYCHAIN=1` for container (no persistent storage)
2. **GitHub Action Secrets**: Use `${{ secrets.* }}` for credentials
3. **MCP Server**: Validate tool parameters to prevent injection

---

## Recommendations Summary

### ✅ COMPLETED

1. ~~Extract `raps-core` shared library~~ → Implemented as `raps-kernel` microkernel
2. ~~Centralize URL configuration (Issue #76)~~ → `raps-kernel/src/config/endpoints.rs`
3. ~~Unify retry logic (Issue #77)~~ → `raps-kernel/src/http/retry.rs`
4. Build performance infrastructure → sccache, nextest, mold/lld-link in CI

### Immediate (P1)

1. Fix parallel multipart upload (Issue #70) - Implementation in `raps-oss/src/upload.rs`
2. Wrap blocking calls with spawn_blocking (Issue #73)
3. Update Docker image to v3.2.0 (critical!)
4. Add `middleware.rs` to kernel HTTP module

### Short-term (P2)

5. Add Windows support to GitHub Action
6. Non-interactive mode audit (Issue #82)
7. Complete MCP-CLI parity (T044-T046)

### Medium-term (P3)

8. Formalize output schemas (Issue #79)
9. Expand MCP tools to match CLI
10. Complete TUI feature set

---

## References

- [RAPS Source](../raps/src/)
- [Issues Directory](../raps/issues/)
- [APS OpenAPI Specs](../aps-sdk-openapi/)
- [Constitution](../.specify/memory/constitution.md)



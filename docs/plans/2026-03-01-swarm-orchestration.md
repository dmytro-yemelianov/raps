# RAPS Swarm Orchestration — Refined Implementation Plan

> Based on "Distributed Orchestration Architecture.md" vision, adapted to the
> actual v4.18.0 codebase. Replaces the 6-agent model with incremental
> capability modules that compose naturally.

## Why Not 6 Agents From Day One

The original doc proposes 6 named agents communicating via typed channels.
The codebase already has the right primitives — `send_with_retry`, `api_health`,
`BulkExecutor` with semaphore concurrency, `HttpClientConfig`, `AuthClient`
with refresh coordination. Building an agent framework on top adds abstraction
without adding capability.

**Instead:** Build each orchestration capability as a standalone kernel module.
Wire them into existing code paths. Compose them into a `SwarmRuntime` only
when the individual pieces prove themselves.

## What Already Exists (v4.18.0)

| Capability | Location | Readiness |
|---|---|---|
| HTTP retry with backoff + Retry-After | `raps-kernel/src/http.rs:160-223` | Production |
| API health (latency, jitter, health status) | `raps-kernel/src/api_health.rs` | Production |
| Content-addressed download cache + prune | `raps-kernel/src/cache.rs` | Production |
| Auth token refresh coordination (mutex + flag) | `raps-kernel/src/auth/three_leg.rs` | Works, improvable |
| Bulk executor (semaphore, per-item retry, progress) | `raps-admin/src/bulk/executor.rs` | Production |
| Pipeline engine (variables, parallel, for-each, retry) | `raps-cli/src/commands/pipeline.rs` | Production |
| TUI dashboard (7 tabs, 30+ views, async fetch) | `raps-cli/src/commands/dashboard/` | Production |
| MCP server (105 tools, lazy client init) | `raps-cli/src/mcp/` | Production |
| Doctor command (6 health checks) | `raps-cli/src/commands/doctor.rs` | Production |
| URL allowlist (credential leak prevention) | `raps-kernel/src/http.rs:28-59` | Production |
| Profiler (HTTP call tracking) | `raps-kernel/src/profiler.rs` | Production |

## What Doesn't Exist Yet

| Capability | Impact | Effort |
|---|---|---|
| Circuit breaker (per-endpoint) | High — prevents cascade failures | Small |
| Contextual retry (failure-type-aware) | High — 429 vs 503 vs translation failure | Medium |
| HTTP response cache (GET caching) | High — eliminates redundant API calls | Medium |
| Rate limit budget (proactive, per-API) | High — prevents 429s entirely | Medium |
| Region auto-detection | Medium — eliminates misrouted requests | Small |
| Checkpoint/resume for batch ops | High — crash recovery for bulk ops | Medium |
| Metrics store (SQLite) | Medium — operational visibility | Medium |
| Audit logging | Medium — enterprise compliance | Small |
| Compound MCP operations | Very High — killer feature for AI agents | Large |
| Singleton token manager (notify-based) | Medium — replaces spin-wait | Small |
| Swarm TUI tab | Medium — unified monitoring | Medium |

---

## Phase 0: Foundation Modules (raps-kernel)

> No new crates. Each module is a file in raps-kernel/src/ that existing
> code can adopt incrementally with minimal churn.

### 0.1 Circuit Breaker

**File:** `raps-kernel/src/circuit_breaker.rs`

Extend the existing `api_health` module with per-endpoint circuit breaking.

```rust
pub struct CircuitBreaker {
    state: AtomicU8,              // Closed=0, Open=1, HalfOpen=2
    failure_count: AtomicU32,
    last_failure: AtomicU64,      // millis since epoch
    config: CircuitBreakerConfig,
}

pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,    // failures before opening (default 5)
    pub reset_timeout: Duration,   // time before probe (default 30s)
    pub failure_window: Duration,  // window for counting failures (default 60s)
}

impl CircuitBreaker {
    pub fn check(&self) -> Result<(), CircuitOpen>;
    pub fn record_success(&self);
    pub fn record_failure(&self);
}
```

**Integration point:** Wrap inside `send_with_retry`. Before sending, call
`breaker.check()`. On success/failure, call the corresponding record method.

**Registry:** `CircuitBreakerRegistry` holds a `DashMap<String, CircuitBreaker>`
keyed by API endpoint group (e.g. "model-derivative", "data-management", "oss").

### 0.2 Contextual Retry Policies

**File:** `raps-kernel/src/retry_policy.rs`

Replace the one-size-fits-all backoff with failure-type-aware strategies.

```rust
pub enum FailureType {
    RateLimited,                   // 429
    ServerError,                   // 500
    ServiceUnavailable,            // 503
    GatewayTimeout,                // 504
    Unauthorized,                  // 401
    TranslationInternalFailure,    // manifest message
    TranslationDownloadFailure,    // manifest message
    RegionMismatch,                // error pattern
    NetworkTimeout,                // reqwest error
}

pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    pub pre_retry: Option<PreRetryAction>,
}

pub enum PreRetryAction {
    RefreshToken,
    SwitchRegion,
    ReUpload,
    CheckExisting,
}
```

**Default policy table** (from the architecture doc — this is excellent as-is):

| Failure | Detection | Recovery |
|---|---|---|
| 429 | HTTP status | Wait Retry-After, else exponential from 1s |
| TranslationWorker-InternalFailure | Manifest msg | Re-upload source, retry translation (max 2) |
| 401 Unauthorized | HTTP status | Refresh token, retry once |
| 500 | HTTP status | Exponential 1s→16s, max 5 |
| 503 | HTTP status | Trigger circuit breaker eval |
| Network timeout | Connection error | Retry with increased timeout, 3 attempts |
| 409 Conflict (translation) | HTTP status | Switch to polling |

**Integration:** Enhance `send_with_retry` to accept an optional `RetryPolicy`.
Existing callers keep current behavior (backward compatible).

### 0.3 HTTP Response Cache

**File:** `raps-kernel/src/response_cache.rs`

In-memory LRU cache for idempotent GET responses. Not a separate agent —
a middleware layer around the HTTP client.

```rust
pub struct ResponseCache {
    entries: Mutex<LruCache<CacheKey, CachedResponse>>,
    config: ResponseCacheConfig,
}

pub struct CacheKey {
    method: Method,
    url: String,
    // No auth headers in key — same endpoint, same response
}

pub struct CachedResponse {
    status: u16,
    headers: HeaderMap,
    body: Bytes,
    cached_at: Instant,
    ttl: Duration,
}

impl ResponseCache {
    pub fn get(&self, key: &CacheKey) -> Option<CachedResponse>;
    pub fn put(&self, key: CacheKey, response: CachedResponse);
    pub fn invalidate_prefix(&self, url_prefix: &str);
    pub fn stats(&self) -> CacheStats;
}
```

**TTL defaults by endpoint group:**

| Endpoint | TTL | Rationale |
|---|---|---|
| Manifest status | 10s | Polled frequently during translation |
| Bucket listing | 300s | Changes rarely |
| Hub/project listing | 300s | Changes rarely |
| Property queries | 600s | Static after translation |
| Folder contents | 120s | Changes occasionally |

**Invalidation:** Any write (POST/PUT/PATCH/DELETE) to a resource prefix
invalidates all cached GETs for that prefix.

**Integration:** New function `send_cached` that checks cache before
calling `send_with_retry`. Opt-in per call site.

### 0.4 Rate Limit Budget

**File:** `raps-kernel/src/rate_budget.rs`

Track rate limit headers from APS responses. Queue requests proactively
when budget is nearly exhausted.

```rust
pub struct RateBudget {
    remaining: AtomicU32,
    limit: AtomicU32,
    reset_at: AtomicU64,         // unix millis
}

pub struct RateBudgetRegistry {
    budgets: DashMap<String, RateBudget>,
}

impl RateBudgetRegistry {
    /// Update budget from response headers
    pub fn record(&self, endpoint: &str, headers: &HeaderMap);
    /// Check if we should wait before sending
    pub fn check(&self, endpoint: &str) -> RateStatus;
    /// Wait until budget allows (async)
    pub async fn wait_for_budget(&self, endpoint: &str);
}

pub enum RateStatus {
    Ok { remaining: u32 },
    NearLimit { remaining: u32 },  // < 10% remaining
    Exhausted { retry_after: Duration },
}
```

**Known APS limits** (hardcoded fallbacks when headers missing):

| API | Limit | Window |
|---|---|---|
| Authentication | 500/min | 1 min |
| Data Management | 100/min | 1 min |
| Model Derivative | 20/min | 1 min |
| OSS | 500/min | 1 min |
| Design Automation | 50 concurrent | — |

**Integration:** `send_with_retry` calls `budget.check()` before sending,
`budget.record()` after receiving response.

### 0.5 Region Auto-Detection

**File:** `raps-kernel/src/region.rs`

```rust
pub enum Region { US, EMEA }

pub fn detect_from_hub(hub_attributes: &serde_json::Value) -> Option<Region>;
pub fn detect_from_urn(urn: &str) -> Option<Region>;
pub fn endpoint_for_region(base: &str, region: Region) -> String;
```

**Detection logic:**
1. Hub metadata → `extension.data.region` field
2. URN prefix patterns (EMEA-originated URNs)
3. Bucket creation region from bucket details
4. Default: US

**Integration:** Proxy layer in HTTP client injects `x-ads-region` header
when region is detected. Cached per hub/bucket in `DashMap`.

### 0.6 Improved Token Manager

**File:** Modify `raps-kernel/src/auth/three_leg.rs`

Replace spin-wait (100ms sleep loop) with `tokio::sync::Notify`:

```rust
pub struct TokenCache {
    token: Option<StoredToken>,
    refreshing: bool,
    notify: Arc<Notify>,  // wake waiters when refresh completes
}
```

Add proactive refresh: when token has < 5 min remaining and a request
comes in, spawn background refresh. Current request uses existing token.

---

## Phase 1: Orchestration Layer

> After Phase 0 modules are proven, compose them.

### 1.1 Enhanced HTTP Client Stack

**File:** `raps-kernel/src/http.rs` (extend existing)

Compose the Phase 0 modules into the existing `send_with_retry`:

```
Request → Rate Budget Check → Response Cache Check → Circuit Breaker Check
        → Region Routing → Send → Record Budget → Record Health
        → Cache Response → Circuit Breaker Update → Return
```

This is NOT a new "agent" — it's the same `send_with_retry` function
enhanced with optional middleware. Existing callers get the improvements
automatically through `HttpClientConfig` flags.

```rust
pub struct HttpClientConfig {
    // Existing fields...
    pub max_retries: u32,
    pub timeout: u64,
    // New Phase 1 fields
    pub cache_enabled: bool,        // default true
    pub circuit_breaker: bool,      // default true
    pub rate_budget: bool,          // default true
    pub auto_region: bool,          // default true
}
```

### 1.2 Checkpoint Store

**File:** `raps-kernel/src/checkpoint.rs`

Persistent state for resumable batch operations. JSON files (not SQLite —
avoids new dependency, matches existing state.rs pattern in raps-admin).

```rust
pub struct CheckpointStore {
    dir: PathBuf,  // ~/.local/share/raps/checkpoints/
}

pub struct Checkpoint {
    pub workflow_id: String,
    pub workflow_type: String,
    pub total_units: usize,
    pub completed: Vec<usize>,    // indices of completed items
    pub failed: Vec<(usize, String)>,  // index + error
    pub created_at: String,
    pub updated_at: String,
}

impl CheckpointStore {
    pub fn save(&self, cp: &Checkpoint) -> Result<()>;
    pub fn load(&self, workflow_id: &str) -> Result<Option<Checkpoint>>;
    pub fn find_resumable(&self, workflow_type: &str, input_hash: &str) -> Result<Option<Checkpoint>>;
    pub fn remove(&self, workflow_id: &str) -> Result<()>;
}
```

**Integration:** Wire into `BulkExecutor` — save checkpoint after each
completed item. On startup, check for incomplete checkpoints matching
the same input.

### 1.3 Metrics Collector

**File:** `raps-kernel/src/metrics.rs`

Lightweight metrics aggregation. In-memory with periodic JSON dump
(not SQLite for Phase 1 — minimize deps).

```rust
pub struct MetricsCollector {
    api_requests: DashMap<String, ApiMetrics>,
    translations: Mutex<Vec<TranslationMetric>>,
    flush_path: PathBuf,  // ~/.local/share/raps/metrics/
}

pub struct ApiMetrics {
    pub request_count: AtomicU64,
    pub error_count: AtomicU64,
    pub total_latency_ms: AtomicU64,
    pub cache_hits: AtomicU64,
}

pub struct TranslationMetric {
    pub urn: String,
    pub file_type: String,
    pub duration_ms: u64,
    pub status: String,
    pub region: String,
}
```

Periodically flush to JSON. Doctor command reads these for reporting.
Move to SQLite in Phase 2 when we actually need queries.

### 1.4 Audit Logger

**File:** `raps-kernel/src/audit.rs`

JSONL append-only log. One line per operation. Daily rotation.

```rust
pub fn log_operation(entry: &AuditEntry) -> Result<()>;

pub struct AuditEntry {
    pub timestamp: String,
    pub operation: String,
    pub resource: String,
    pub result: String,
    pub duration_ms: u64,
    pub user: Option<String>,
}
```

Path: `~/.local/share/raps/audit/YYYY-MM-DD.jsonl`
Configurable retention (default 90 days). Auto-prune old files.

### 1.5 Compound MCP Operations

**File:** `raps-cli/src/mcp/compound.rs`

This is the highest-value feature. New MCP tools that compose existing
atomic tools into workflows:

| Tool | Steps | Value |
|---|---|---|
| `raps.analyze_model(file)` | Upload → Translate → Extract Props → Metadata | AI agent does 1 call instead of 5 |
| `raps.prepare_for_viewing(file)` | Upload → Translate SVF2 → Wait → Viewer URL | Most common workflow, single call |
| `raps.batch_process(files, actions)` | Upload all → Distribute → Collect → Report | Bulk ops from AI agent |
| `raps.compare_versions(urn1, urn2)` | Extract props both → Diff → Report | Version comparison |

Each compound tool internally uses the enhanced HTTP stack (cache, circuit
breaker, rate budget, retry). The AI agent gets reliability for free.

### 1.6 Swarm CLI Commands

```bash
raps swarm status          # Circuit breaker states, rate budgets, cache stats
raps swarm metrics         # API latency, translation stats, error rates
raps swarm resume          # Find and resume incomplete batch operations
raps swarm queue           # Show pending/active work (when queue exists)
```

### 1.7 TUI Swarm Tab (F8)

New dashboard tab showing:
- Circuit breaker states per API (Closed/Open/HalfOpen)
- Rate limit budget utilization
- Response cache hit rate
- Recent translations with timing
- Active batch operations with progress

Uses same `mpsc` pattern as existing tabs for async data loading.

---

## Phase 2: Distribution & Serverless

> Only after Phase 0+1 are stable. Adds network-separated components.

### 2.1 Work Queue with Priority

Move from direct execution to queue-based dispatching. Use Redis Streams
(not custom channels — Redis is already needed for shared cache).

Three priority levels: Critical (interactive), Normal (batch), Background
(scheduled). Existing `BulkExecutor` becomes a queue consumer.

### 2.2 Docker Images

```
rapscli/raps-core    ← Full CLI + all Phase 0/1 modules (~15MB alpine)
rapscli/raps-worker  ← Queue consumer, stateless, horizontally scalable
rapscli/raps-proxy   ← Response cache + rate budget (Redis-backed)
rapscli/raps-webhook ← Webhook receiver + relay
```

### 2.3 Serverless Functions

**Cloudflare Workers** (edge, 0ms cold start):
- Webhook gateway (`hooks.rapscli.xyz`)
- Translation status cache proxy
- URN encode/decode utility API

**Fly.io Machines** (compute, scale to zero):
- Fire-and-forget translation orchestrator
- Batch processor
- DA workitem runner
- Scheduled pipeline executor

### 2.4 docker-compose.yml

Team deployment: shared proxy + worker pool + webhook relay + Redis + dashboard.
As specified in the architecture doc — that section is solid.

---

## Phase 3: Kubernetes + Multi-Tenant

As specified in the architecture doc. Helm chart, HPA on queue depth,
namespace isolation, Prometheus + Grafana. No changes to the original plan.

---

## Phase 4: Platform / SaaS

As specified. `api.rapscli.xyz`, pay-per-operation, web console,
cross-platform expansion (PTC Onshape, Dassault, Siemens).

---

## Implementation Order (Phase 0+1 Detail)

Priority: highest impact, lowest risk first. Each item is independently
shippable and testable.

| # | Module | Depends On | New Deps | Est. Scope |
|---|---|---|---|---|
| 1 | Circuit breaker | — | dashmap | ~200 LOC + tests |
| 2 | Contextual retry policies | — | — | ~300 LOC + tests |
| 3 | Wire circuit breaker + retry into send_with_retry | 1, 2 | — | ~100 LOC |
| 4 | Rate limit budget | — | dashmap | ~200 LOC + tests |
| 5 | Wire rate budget into send_with_retry | 4 | — | ~50 LOC |
| 6 | Region auto-detection | — | — | ~150 LOC + tests |
| 7 | HTTP response cache | — | lru | ~300 LOC + tests |
| 8 | Wire response cache into send_with_retry | 7 | — | ~80 LOC |
| 9 | Token manager improvement (Notify) | — | — | ~50 LOC (modify existing) |
| 10 | Checkpoint store | — | — | ~250 LOC + tests |
| 11 | Wire checkpoint into BulkExecutor | 10 | — | ~100 LOC |
| 12 | Audit logger | — | — | ~150 LOC + tests |
| 13 | Metrics collector | — | dashmap | ~250 LOC + tests |
| 14 | `raps swarm status/metrics` commands | 1,4,7,13 | — | ~200 LOC |
| 15 | Compound MCP tools | 1-9 | — | ~500 LOC + tests |
| 16 | TUI Swarm tab (F8) | 1,4,7,13 | — | ~400 LOC |
| 17 | `raps swarm resume` | 10 | — | ~100 LOC |

**Total Phase 0+1:** ~3,400 LOC + tests
**New workspace deps:** `dashmap`, `lru` (both lightweight, no async runtime)

---

## Key Differences From Original Architecture Doc

| Original | Refined | Rationale |
|---|---|---|
| 6 named agents | Composable kernel modules | Agents are premature abstraction; modules compose via existing call paths |
| Typed mpsc channels between agents | Direct function calls + shared registries | No inter-agent messaging needed in single-process; channels add complexity |
| SwarmRuntime struct from day one | Compose at HTTP client layer | The "swarm" is just the enhanced send_with_retry — no framework needed |
| SQLite for metrics + checkpoints (Phase 1) | JSON files (Phase 1), SQLite (Phase 2) | Avoid new dep; match existing state.rs pattern; upgrade when queries needed |
| All 6 agents in Phase 1 | Incremental modules, 17 shippable steps | Each step adds value independently; can stop/pivot at any point |
| ServerlessDispatchAgent in Phase 1 | Phase 2 only | No infrastructure changes in Phase 1 |
| New raps-swarm crate | raps-kernel extensions | Capabilities belong in kernel; avoid crate proliferation |
| 105 → compound MCP tools separately | Compound tools are Phase 1 priority | Highest value for AI agent users; builds on all other modules |

---

## New Workspace Dependencies

```toml
# Add to [workspace.dependencies] in Cargo.toml
dashmap = "6"           # Lock-free concurrent HashMap (circuit breakers, rate budgets, metrics)
lru = "0.12"            # LRU cache for HTTP response caching
```

Both are zero-async-runtime, small, well-maintained crates.
No SQLite, no Redis, no message bus in Phase 0+1.

---

## Open Questions

1. **dashmap vs existing Mutex<HashMap>**: dashmap is cleaner for concurrent
   access but adds a dep. Alternative: `std::sync::RwLock<HashMap>`.

2. **Response cache scope**: Per-process only? Or should the download cache
   (already on disk) also cache API responses for cross-invocation benefit?

3. **Compound MCP tool granularity**: Start with 2 (analyze_model +
   prepare_for_viewing) or build all 4 at once?

4. **Metrics format**: JSON lines vs structured JSON per day? JSON lines
   is simpler and append-friendly.

5. **Phase 2 message bus**: Redis Streams (simpler, already needed for cache)
   vs NATS JetStream (purpose-built, more features). Recommend Redis for
   simplicity.

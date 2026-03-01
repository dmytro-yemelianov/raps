# Claude Code Task: RAPS Agent Swarm — Distributed Orchestration Architecture

## Context

RAPS CLI (rapscli.xyz) is a Rust-based command-line tool for Autodesk Platform Services (APS) currently at v4.13.0 with 100+ commands, 51 MCP tools, a TUI dashboard (7 tabs, 33 views), Reedline shell, and Python bindings via PyO3. It talks directly to APS endpoints from a single binary.

This document specifies the next architectural evolution: an **intermediate swarm of orchestrated agents** that transforms RAPS from a direct API caller into a resilient, distributed orchestration platform. The architecture spans from in-process agents (Phase 1) through Docker/serverless deployment (Phase 2-3) to a full Kubernetes-based multi-tenant platform (Phase 4).

**Key constraint:** Phase 1 must preserve the single-binary distribution model. Agents are logical separations within one process, communicating through typed channels, not network calls. Distributed infrastructure comes in later phases.

-----

## Architecture Overview

```
┌──────────────────────────────────────────────────────────┐
│          RAPS CLI / MCP Interface / Python Bindings       │
└───────────────────────────┬──────────────────────────────┘
                            │
                 ┌──────────▼───────────┐
                 │  Coordination Agent  │  ← Decomposes high-level ops
                 │  (Brain)             │
                 └──────────┬───────────┘
                            │
       ┌──────────┬─────────┼─────────┬──────────────┐
       ▼          ▼         ▼         ▼              ▼
┌──────────┐┌─────────┐┌────────┐┌──────────┐┌───────────┐
│CDN/Proxy ││Distribu-││Reliabi-││Observabi-││Serverless │
│Agent     ││tion     ││lity    ││lity      ││Dispatch   │
│          ││Agent    ││Agent   ││Agent     ││Agent      │
└────┬─────┘└───┬─────┘└───┬────┘└────┬─────┘└─────┬─────┘
     │          │          │          │             │
     └────┬─────┴─────┬────┘          │             │
          ▼           ▼               ▼             ▼
   ┌───────────┐ ┌─────────┐  ┌───────────┐ ┌─────────────────┐
   │ APS APIs  │ │ Local   │  │ Metrics / │ │ Serverless      │
   │ (Autodesk)│ │ Queue   │  │ Dashboard │ │ Workers         │
   └───────────┘ └─────────┘  └───────────┘ │ (AWS/CF/Fly.io) │
                                            └─────────────────┘
```

-----

## Agent Specifications

### Agent 1: CDN/Proxy Agent

**Purpose:** Smart request routing, caching, region detection, connection pooling, rate limit budgeting.

**Problem it solves:** Every RAPS command currently hits Autodesk endpoints directly. Repeated reads (manifest polling, property queries, bucket listings) waste tokens and hit rate limits. Region mismatches (US vs EMEA) cause silent failures. No connection reuse between sequential CLI invocations.

#### Responsibilities

1. **Response caching for idempotent reads.**
- Cache GET responses for: manifest status, property queries, bucket listings, project listings, hub info.
- Cache key: `{method}:{endpoint}:{query_params_hash}`.
- Default TTL: 300 seconds (configurable via `raps config set cache.ttl <seconds>`).
- Cache invalidation: on any write operation to the same resource, or on explicit `--no-cache` flag.
- Storage: in-memory LRU cache (Phase 1), Redis (Phase 2+).
   
   **Concrete impact:** `raps translate status <urn>` currently polls the manifest endpoint every N seconds. With caching, 60 polls over 5 minutes become 3-4 actual API calls. The rest are served from cache with sub-millisecond latency.
1. **Automatic region detection and routing.**
- Inspect URN prefix and bucket metadata to determine US vs EMEA origin.
- Route requests to correct regional endpoint automatically.
- For ACC/BIM360 data: detect account region from hub metadata.
- Eliminate the need for `--region` flags or manual `x-ads-region` headers.
- Decision logic:
  
  ```
  Is the resource from ACC/BIM360?
  ├── Yes → Check hub region metadata → Route accordingly
  └── No (OSS) → Check bucket creation region → Route accordingly
  
  If URN starts with specific EMEA prefixes → developer.api.autodesk.com + x-ads-region: EMEA
  Default → developer.api.autodesk.com (US)
  ```
- This architecturally solves the Region Mismatch problem documented in RAPS marketing materials, replacing a diagnostic tool with automatic prevention.
1. **Connection pooling and TLS session reuse.**
- Maintain a pool of warm HTTPS connections to APS endpoints.
- Reuse TLS sessions across sequential RAPS commands within the same process.
- Pool configuration: max 20 connections per endpoint, idle timeout 120s.
- Eliminates TLS handshake latency on sequential operations like `raps upload` → `raps translate` → `raps status`.
1. **Rate limit budget management.**
- Track consumed rate limit budget per API based on response headers (`X-RateLimit-Remaining`, `Retry-After`).
- Known limits to enforce proactively:
  
  |API                         |Limit        |Window  |
  |----------------------------|-------------|--------|
  |Authentication              |500/min      |1 minute|
  |Data Management             |100/min      |1 minute|
  |Model Derivative (translate)|20/min       |1 minute|
  |OSS                         |500/min      |1 minute|
  |Design Automation           |50 concurrent|—       |
- When a budget is nearly exhausted, queue requests locally rather than hitting 429.
- Distribute budget fairly between concurrent operations (e.g., interactive command gets priority over background batch).

#### Data Structures

```rust
struct ProxyAgent {
    cache: Arc<RwLock<LruCache<CacheKey, CachedResponse>>>,
    connection_pool: ConnectionPool,
    rate_budgets: HashMap<ApiEndpoint, RateBudget>,
    region_cache: HashMap<String, Region>,  // URN/bucket → detected region
}

struct CacheKey {
    method: HttpMethod,
    endpoint: String,
    params_hash: u64,
}

struct CachedResponse {
    body: Bytes,
    headers: HeaderMap,
    cached_at: Instant,
    ttl: Duration,
}

struct RateBudget {
    remaining: AtomicU32,
    window_reset: Instant,
    limit: u32,
}

enum Region {
    US,
    EMEA,
}
```

#### Configuration

```toml
# ~/.config/raps/swarm.toml
[proxy]
cache_enabled = true
cache_ttl_seconds = 300
cache_max_entries = 10000
auto_region_detection = true
connection_pool_size = 20
connection_idle_timeout_seconds = 120
rate_limit_proactive = true  # queue before hitting 429
```

-----

### Agent 2: Distribution Agent

**Purpose:** Decompose bulk operations into work units, schedule respecting rate limits, manage priority queues, handle fan-out/fan-in patterns.

**Problem it solves:** Bulk operations (`raps acc bulk-add-users`, batch translations) execute sequentially or with basic concurrency. No priority system — an interactive `raps translate` command waits behind 200 queued batch translations. No awareness of rate limit constraints at scheduling time.

#### Responsibilities

1. **Work unit decomposition.**
- `raps bulk-translate *.rvt` (200 files) → 200 individual TranslationJob work units.
- `raps acc bulk-add-users users.csv` (500 users) → batches of N users per API call.
- `raps props extract-all --project X` → parallel property extraction jobs per model.
- Each work unit is independently retryable (delegates to Reliability Agent).
1. **Rate-aware scheduling.**
- Consult Proxy Agent for current rate budget before dispatching.
- Schedule translation waves: 20 files per wave, 60-second intervals (Model Derivative: 20/min).
- Schedule DA workitems: maintain pool of max 50 concurrent, backfill as slots open.
- Scheduling algorithm:
  
  ```
  while work_queue is not empty:
      budget = proxy.get_rate_budget(target_api)
      if budget.remaining > 0:
          dispatch next work unit
          budget.remaining -= 1
      else:
          sleep until budget.window_reset
  ```
1. **Priority queue.**
- Three priority levels: `Critical` (interactive user commands), `Normal` (explicit batch requests), `Background` (scheduled/automated jobs).
- Interactive commands (`raps translate model.rvt`) always classified as Critical.
- Batch commands (`raps bulk-translate`) classified as Normal.
- Scheduled pipelines (cron, webhook-triggered) classified as Background.
- Critical jobs can preempt Normal/Background queue positions.
1. **Fan-out / Fan-in aggregation.**
- For compound operations: track all child jobs, collect results, produce aggregate report.
- Example: `raps bulk-translate *.rvt --report` → translate 200 files → collect 200 results → produce summary (X succeeded, Y failed, Z skipped, total time, total tokens).
- Support partial completion: report is available even if some jobs fail.
1. **DA workitem pool management.**
- Maintain a pool tracking active Design Automation workitems (max 50 concurrent).
- When a workitem completes, automatically dispatch next queued item.
- Track workitem duration for progress estimation.

#### Data Structures

```rust
struct DistributionAgent {
    work_queue: PriorityQueue<WorkUnit>,
    active_jobs: HashMap<JobId, ActiveJob>,
    da_pool: DaPool,
    aggregators: HashMap<BatchId, ResultAggregator>,
}

#[derive(PartialOrd, Ord)]
enum Priority {
    Critical = 0,   // interactive
    Normal = 1,     // explicit batch
    Background = 2, // scheduled
}

struct WorkUnit {
    id: JobId,
    batch_id: Option<BatchId>,
    priority: Priority,
    operation: Operation,
    target_api: ApiEndpoint,
    created_at: Instant,
}

enum Operation {
    Translate { urn: String, format: OutputFormat, options: TranslateOptions },
    Upload { file_path: PathBuf, bucket: String },
    ExtractProperties { urn: String, output: PathBuf },
    AddUser { project_id: String, user: UserInfo },
    DaWorkItem { activity: String, inputs: Vec<DaInput> },
    // ... extensible
}

struct DaPool {
    max_concurrent: u32,
    active: HashSet<WorkItemId>,
    queued: VecDeque<WorkUnit>,
}

struct ResultAggregator {
    batch_id: BatchId,
    total: usize,
    completed: AtomicUsize,
    failed: AtomicUsize,
    results: Vec<Option<JobResult>>,
}
```

#### User-Facing Commands

```bash
# Batch translate with distribution
raps bulk-translate ./models/ --format svf2 --priority normal --report

# Check queue status
raps swarm queue status
# Output:
# Queue depth: 147 jobs
# Active: 20 (translations), 3 (DA workitems)
# Critical: 0 | Normal: 127 | Background: 20
# ETA: ~8 minutes

# Adjust priority of running batch
raps swarm queue priority <batch-id> critical

# Pause/resume queue
raps swarm queue pause
raps swarm queue resume
```

-----

### Agent 3: Reliability Agent

**Purpose:** Contextual retry, circuit breaking, resumable workflows, singleton token management.

**Problem it solves:** Current retry logic is basic exponential backoff. No awareness of failure context (a `TranslationWorker-InternalFailure` needs a different recovery strategy than a 429). No circuit breaker — if Model Derivative is down, RAPS keeps sending requests and failing. Token refresh race conditions in concurrent scenarios. No workflow resume after crash.

#### Responsibilities

1. **Contextual retry with failure-type awareness.**
- Each failure type maps to a specific recovery strategy:
   
   |Failure                            |Detection                     |Recovery Strategy                                                                                               |
   |-----------------------------------|------------------------------|----------------------------------------------------------------------------------------------------------------|
   |`429 Too Many Requests`            |HTTP status                   |Wait `Retry-After` header duration, then retry. If no header, exponential backoff starting at 1s.               |
   |`TranslationWorker-InternalFailure`|Manifest message              |Re-upload source file first (may have been garbage-collected), then retry translation. Max 2 re-upload attempts.|
   |`TranslationWorker-FailedDownload` |Manifest message              |Re-upload source file with new signed URL. Retry once.                                                          |
   |Region mismatch (various 4xx)      |Error message pattern matching|Auto-detect correct region via Proxy Agent, retry in alternate region.                                          |
   |`401 Unauthorized`                 |HTTP status                   |Refresh token via Token Manager, retry once. If still 401, surface to user.                                     |
   |`500 Internal Server Error`        |HTTP status                   |Exponential backoff: 1s, 2s, 4s, 8s, 16s. Max 5 attempts.                                                       |
   |`503 Service Unavailable`          |HTTP status                   |Trigger circuit breaker evaluation. Retry after circuit opens.                                                  |
   |Network timeout                    |Connection error              |Retry with increased timeout. 3 attempts.                                                                       |
   |`409 Conflict` (translation)       |HTTP status                   |Check if translation already in progress. If yes, switch to polling instead of retrying.                        |
- Retry budget per job: max 5 total retries across all failure types. Prevents infinite retry loops.
- Each retry logs: attempt number, failure type, recovery strategy chosen, wait duration.
1. **Circuit breaker.**
- Per-API circuit breaker with three states: Closed (normal), Open (failing), Half-Open (probing).
- Transition rules:
  
  ```
  Closed → Open: 5 failures within 60 seconds to the same API
  Open → Half-Open: After 30 seconds, allow 1 probe request
  Half-Open → Closed: Probe succeeds
  Half-Open → Open: Probe fails, reset 30-second timer
  ```
- When circuit is Open: all new requests to that API are queued locally (not rejected). They wait for circuit to close.
- Dashboard integration: TUI shows circuit breaker states for all APIs.
- User notification: `⚠ Model Derivative API circuit breaker OPEN — requests queued. Probing every 30s.`
1. **Resumable workflows with checkpoint.**
- For batch operations (bulk translate, bulk user add, pipeline runs):
  - Checkpoint state after each completed work unit.
  - Store checkpoint in local SQLite: `~/.local/share/raps/checkpoints.db`
  - Schema:
    
    ```sql
    CREATE TABLE checkpoints (
        workflow_id TEXT PRIMARY KEY,
        workflow_type TEXT NOT NULL,
        total_units INTEGER NOT NULL,
        completed_units INTEGER NOT NULL,
        state BLOB NOT NULL,  -- serialized workflow state (bincode)
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    
    CREATE TABLE work_unit_results (
        workflow_id TEXT NOT NULL,
        unit_index INTEGER NOT NULL,
        status TEXT NOT NULL,  -- 'completed', 'failed', 'pending'
        result BLOB,           -- serialized result
        error TEXT,
        PRIMARY KEY (workflow_id, unit_index)
    );
    ```
- On crash recovery:
  
  ```bash
  raps swarm resume
  # Output:
  # Found incomplete workflow: bulk-translate (147/200 completed)
  # Resuming from unit 148...
  ```
- Auto-resume on next `raps bulk-translate` if same input is detected (content hash match).
1. **Singleton Token Manager.**
- Single source of truth for all auth tokens across all agents.
- Solves the refresh token race condition:
  
  ```rust
  struct TokenManager {
      current_token: RwLock<Option<Token>>,
      refresh_in_progress: Mutex<Option<Shared<BoxFuture<Token>>>>,
  }
  
  impl TokenManager {
      async fn get_valid_token(&self) -> Result<Token> {
          // Fast path: current token is valid
          if let Some(token) = self.current_token.read().await.as_ref() {
              if !token.is_expired() && !token.expires_within(Duration::from_secs(300)) {
                  return Ok(token.clone());
              }
          }
          
          // Slow path: need refresh — but only one concurrent refresh
          let mut refresh_lock = self.refresh_in_progress.lock().await;
          if let Some(existing_future) = refresh_lock.as_ref() {
              // Another task is already refreshing — wait for its result
              drop(refresh_lock);
              return existing_future.clone().await;
          }
          
          // We're the first to refresh
          let refresh_future = async { /* actual refresh logic */ }.boxed().shared();
          *refresh_lock = Some(refresh_future.clone());
          drop(refresh_lock);
          
          let result = refresh_future.await;
          
          // Clear the in-progress marker
          *self.refresh_in_progress.lock().await = None;
          
          result
      }
  }
  ```
- Proactive refresh: when token has < 5 minutes remaining, trigger background refresh before any request needs it.
- All agents call `token_manager.get_valid_token()` instead of managing their own tokens.

#### Data Structures

```rust
struct ReliabilityAgent {
    circuit_breakers: HashMap<ApiEndpoint, CircuitBreaker>,
    token_manager: Arc<TokenManager>,
    checkpoint_db: SqlitePool,
    retry_policies: HashMap<FailureType, RetryPolicy>,
}

struct CircuitBreaker {
    state: AtomicU8,  // 0=Closed, 1=Open, 2=HalfOpen
    failure_count: AtomicU32,
    failure_window_start: Instant,
    last_failure: Instant,
    open_since: Option<Instant>,
    probe_interval: Duration,
}

enum FailureType {
    RateLimited,
    TranslationInternalFailure,
    TranslationDownloadFailure,
    RegionMismatch,
    Unauthorized,
    ServerError,
    ServiceUnavailable,
    NetworkTimeout,
    Conflict,
}

struct RetryPolicy {
    max_attempts: u32,
    backoff: BackoffStrategy,
    pre_retry_action: Option<PreRetryAction>,
}

enum BackoffStrategy {
    Fixed(Duration),
    Exponential { base: Duration, max: Duration },
    HeaderBased,  // use Retry-After header
}

enum PreRetryAction {
    ReUploadFile,
    RefreshToken,
    SwitchRegion,
    CheckExistingTranslation,
}
```

-----

### Agent 4: Observability Agent

**Purpose:** Metrics collection, API performance tracking, translation analytics, audit logging, TUI dashboard integration.

**Problem it solves:** APS provides near-zero operational visibility. Developers have no insight into translation success rates, API response time trends, token consumption patterns, or historical performance data. No audit trail for enterprise compliance.

#### Responsibilities

1. **Translation analytics.**
- Track per-translation: file type, file size, translation time, success/failure, output format, region.
- Aggregate into statistics:
  - Success rate by file type (e.g., “Revit: 94%, IFC: 87%, DWG: 99%”)
  - Average translation time by file size bucket (e.g., “<10MB: 2min, 10-50MB: 8min, 50-200MB: 25min, >200MB: 65min”)
  - Failure reason distribution (e.g., “InternalFailure: 40%, Timeout: 30%, Corrupted: 20%, Region: 10%”)
- Prediction: after sufficient data points (>100 translations per file type), estimate completion time for new translations based on file type + size.
- Storage: local SQLite `~/.local/share/raps/metrics.db`.
1. **API performance monitoring.**
- Track per-request: endpoint, method, response time, status code, response size.
- Compute rolling averages: p50, p95, p99 response times per API.
- Detect degradation: alert when p95 exceeds 2x the rolling 7-day average.
- Track rate limit utilization: “Data Management: 67/100 budget used this minute.”
1. **Token consumption tracking.**
- Track cloud credits (tokens) consumed per operation.
- Token costs per operation type:
  
  |Operation                          |Token Cost        |
  |-----------------------------------|------------------|
  |Model Derivative (Revit/Navisworks)|1.5 tokens        |
  |Model Derivative (other formats)   |0.5 tokens        |
  |Design Automation (per hour)       |6 tokens          |
  |Viewer sessions                    |Free              |
  |Data Management                    |Free              |
  |OSS Storage                        |Free (with limits)|
- Daily/weekly/monthly consumption reports.
- Budget alerts: “⚠ 80% of monthly token budget consumed (15 days remaining).”
1. **Audit logging (enterprise feature).**
- Log all operations with: timestamp, user identity, operation type, target resource, result, duration.
- Format: structured JSON lines, one per operation.
- Rotation: daily files, configurable retention (default 90 days).
- Path: `~/.local/share/raps/audit/YYYY-MM-DD.jsonl`
- Sample entry:
  
  ```json
  {
    "ts": "2026-03-01T14:30:00Z",
    "user": "dmytro@yemelianov.tech",
    "op": "translate",
    "resource": "urn:adsk.objects:os.object:bucket/model.rvt",
    "result": "success",
    "duration_ms": 45230,
    "tokens_consumed": 1.5,
    "region": "EMEA",
    "retry_count": 0
  }
  ```
1. **TUI Dashboard integration.**
- New tab: “Swarm” in existing TUI dashboard (8th tab).
- Views:
  
  |View             |Content                                                      |
  |-----------------|-------------------------------------------------------------|
  |Agent Status     |Per-agent health: running/paused/error, uptime, last activity|
  |Queue            |Depth, priority breakdown, ETA, throughput (jobs/min)        |
  |Circuit Breakers |Per-API state (Closed/Open/HalfOpen), failure counts         |
  |Cache            |Hit rate, size, eviction rate                                |
  |Translation Stats|Success rate chart, avg time by type, recent translations    |
  |Token Usage      |Daily consumption, budget remaining, cost projection         |
  |Audit Feed       |Live stream of recent operations                             |

#### Data Structures

```rust
struct ObservabilityAgent {
    metrics_db: SqlitePool,
    audit_writer: AuditWriter,
    api_stats: DashMap<ApiEndpoint, ApiStats>,
    translation_stats: DashMap<FileType, TranslationStats>,
    token_tracker: TokenTracker,
}

struct ApiStats {
    request_count: AtomicU64,
    error_count: AtomicU64,
    latency_histogram: Histogram,  // hdrhistogram crate
    rate_limit_hits: AtomicU64,
}

struct TranslationStats {
    total: AtomicU64,
    succeeded: AtomicU64,
    failed: AtomicU64,
    total_duration: AtomicU64,  // milliseconds
    size_buckets: Vec<SizeBucket>,
}

struct TokenTracker {
    daily_usage: DashMap<NaiveDate, f64>,
    monthly_budget: Option<f64>,
    alert_threshold: f64,  // 0.0-1.0
}

struct AuditWriter {
    current_file: Mutex<BufWriter<File>>,
    current_date: NaiveDate,
    retention_days: u32,
}
```

#### Metrics Database Schema

```sql
CREATE TABLE api_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    status_code INTEGER NOT NULL,
    response_time_ms INTEGER NOT NULL,
    response_size_bytes INTEGER,
    cache_hit BOOLEAN NOT NULL DEFAULT FALSE,
    region TEXT
);

CREATE TABLE translations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    urn TEXT NOT NULL,
    file_type TEXT NOT NULL,
    file_size_bytes INTEGER,
    output_format TEXT NOT NULL,
    status TEXT NOT NULL,  -- 'success', 'failed', 'timeout'
    duration_ms INTEGER,
    failure_reason TEXT,
    tokens_consumed REAL,
    region TEXT,
    retry_count INTEGER DEFAULT 0,
    started_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE TABLE token_usage (
    date TEXT NOT NULL,
    operation_type TEXT NOT NULL,
    tokens_consumed REAL NOT NULL,
    PRIMARY KEY (date, operation_type)
);

-- Indexes for common queries
CREATE INDEX idx_translations_file_type ON translations(file_type);
CREATE INDEX idx_translations_status ON translations(status);
CREATE INDEX idx_api_requests_endpoint ON api_requests(endpoint, timestamp);
```

-----

### Agent 5: Serverless Dispatch Agent

**Purpose:** Offload long-running, scheduled, and webhook operations to cloud infrastructure. Enable fire-and-forget workflows, geographic distribution, and team-shared services.

**Problem it solves:** Some operations don’t make sense locally: 2+ hour Revit translations require keeping a laptop open; scheduled nightly pipelines need a persistent runner; webhook receivers need a public HTTPS endpoint (currently requires ngrok); teams duplicate work translating the same models independently.

#### Architecture

```
┌──────────────────────────────────────────────────────┐
│              RAPS CLI (local machine)                 │
│  raps translate model.rvt --serverless               │
└───────────────────┬──────────────────────────────────┘
                    │ dispatch via HTTPS
                    ▼
┌──────────────────────────────────────────────────────┐
│           Serverless Dispatch Agent                   │
│                                                       │
│  ┌────────────────────────────────────────────────┐  │
│  │ Job Queue (Redis / SQS / Cloudflare Queue)     │  │
│  └──────────────────┬─────────────────────────────┘  │
│                     │                                 │
│    ┌────────────────┼────────────────┐               │
│    ▼                ▼                ▼               │
│ ┌────────┐    ┌──────────┐    ┌──────────┐          │
│ │Worker  │    │ Worker   │    │ Worker   │          │
│ │US-East │    │ EU-West  │    │ AP-South │          │
│ │        │    │          │    │ (future) │          │
│ └───┬────┘    └────┬─────┘    └────┬─────┘          │
│     │              │               │                 │
│     ▼              ▼               ▼                 │
│  APS US         APS EMEA      APS (future)          │
└──────────────────────────────────────────────────────┘
                    │
                    ▼ results via notification
┌──────────────────────────────────────────────────────┐
│  Notification Layer                                   │
│  ├── Webhook → user's server                         │
│  ├── Email notification                              │
│  ├── Slack / Teams message                           │
│  └── raps status --poll (CLI polling via WebSocket)  │
└──────────────────────────────────────────────────────┘
```

#### Serverless Function Types

**Type 1: Edge Functions (Cloudflare Workers)**

Best for: sub-second latency, lightweight operations, global distribution, webhook receiving.

Functions to deploy as Cloudflare Workers:

|Function         |Route                        |Purpose                                                                                      |Cold Start|
|-----------------|-----------------------------|---------------------------------------------------------------------------------------------|----------|
|`webhook-gateway`|`hooks.rapscli.xyz/*`        |Receive APS webhooks, validate HMAC signature, filter events, relay to user endpoint or queue|0ms       |
|`urn-api`        |`api.rapscli.xyz/v1/urn/*`   |URN encode/decode as public utility API                                                      |0ms       |
|`status-proxy`   |`api.rapscli.xyz/v1/status/*`|Cached translation status with smart polling reduction                                       |0ms       |
|`auth-proxy`     |`api.rapscli.xyz/v1/auth/*`  |Token exchange proxy (stores encrypted client credentials in Durable Objects)                |0ms       |

Webhook Gateway implementation spec:

```
Receive request:
1. Validate x-ads-signature HMAC against stored webhook secret
2. Parse event type from payload
3. Check if event type is in user's subscription list
4. If subscribed:
   a. Enqueue to Cloudflare Queue for async processing
   b. Relay to user's configured callback URL
   c. Store event in Durable Object for later retrieval
5. Return HTTP 200 (CRITICAL: Autodesk requires exactly 200, not 201/202)
```

**Type 2: Compute Functions (AWS Lambda / Fly.io Machines)**

Best for: operations that take seconds to minutes, need full Rust binary capability.

|Function                  |Trigger      |Timeout|Purpose                                      |
|--------------------------|-------------|-------|---------------------------------------------|
|`translation-orchestrator`|Queue message|15 min |Start translation, poll to completion, notify|
|`batch-processor`         |Queue message|15 min |Process batch of translations/uploads        |
|`property-extractor`      |Queue message|5 min  |Extract and format model properties          |
|`report-generator`        |Queue message|5 min  |Generate batch operation reports             |
|`scheduled-pipeline`      |Cron trigger |15 min |Run scheduled scan + translate pipelines     |

Translation Orchestrator spec:

```
Input: TranslationJob { urn, output_format, notification_config, retry_policy }
1. Detect region (check URN, query bucket metadata)
2. Start translation: POST /modelderivative/v2/designdata/job
3. Poll manifest with exponential backoff:
   - Initial: 5s
   - Max: 60s
   - Multiplier: 1.5x
   - Timeout: configurable (default 2 hours)
4. On completion:
   a. Record metrics (duration, file type, region, status)
   b. Send notification via configured channel
   c. If part of batch: update batch aggregator
5. On failure:
   a. Apply contextual retry (see Reliability Agent)
   b. If retries exhausted: mark failed, notify with error details
```

**Type 3: Long-Running Services (Fly.io Machines / Container)**

Best for: operations exceeding 15-minute Lambda limits, persistent services.

|Service             |Purpose                                                  |Scaling               |
|--------------------|---------------------------------------------------------|----------------------|
|`da-runner`         |Design Automation workitem orchestration (can take hours)|Scale to 0 when idle  |
|`pipeline-scheduler`|Manage cron-triggered pipelines, maintain schedule state |Always-on (1 instance)|
|`cache-server`      |Shared translation cache for teams                       |Scale by team size    |

#### Serverless Use Cases with CLI Interface

**Use Case 1: Fire-and-forget translation**

```bash
raps translate model.rvt --serverless --notify slack:#raps-builds

# Output:
# ✓ File uploaded: urn:adsk.objects:os.object:bucket/model.rvt
# ✓ Job dispatched to serverless worker: job_abc123
# ✓ Worker region: EU-West (detected EMEA account)
# ✓ Notification: Slack #raps-builds
# ✓ Track progress: raps job status job_abc123
#
# You can close your terminal. You'll be notified when done.
```

**Use Case 2: Nightly batch pipeline**

```bash
raps pipeline create nightly-models \
  --source "acc://project-x/03 - Design Models/" \
  --trigger cron "0 2 * * *" \
  --filter "*.rvt,*.nwc" \
  --action translate --format svf2 \
  --action extract-props --output s3://reports/ \
  --notify email:team@company.com \
  --notify slack:#builds

# Output:
# ✓ Pipeline "nightly-models" created
# ✓ Schedule: daily at 02:00 UTC
# ✓ Source: ACC project "Project X" / 03 - Design Models
# ✓ Filters: *.rvt, *.nwc
# ✓ Actions: translate (SVF2) → extract properties
# ✓ Deployed to: pipeline-scheduler (Fly.io)
```

**Use Case 3: Serverless webhook endpoint**

```bash
raps webhook serve --serverless

# Output:
# ✓ Webhook endpoint deployed: https://hooks.rapscli.xyz/usr_abc123
# ✓ Use this URL when registering APS webhooks
# ✓ Events will be relayed to: http://localhost:3000/webhook (when online)
# ✓ Events are queued when you're offline — retrieve with:
#   raps webhook drain
```

**Use Case 4: Shared team translation cache**

```bash
# Team setup (admin)
raps cache init --team "acme-engineering" --serverless

# Developer usage (automatic, zero config after team join)
raps translate model.rvt
# → Cache miss, translating... (45 minutes)
# → Result cached to team "acme-engineering"

# Another developer, same file:
raps translate model.rvt
# → Cache hit from team "acme-engineering" (0.2 seconds)
# → 0 tokens consumed
```

#### Infrastructure Selection by Function Type

|Criterion              |Cloudflare Workers                        |AWS Lambda                      |Fly.io Machines                     |
|-----------------------|------------------------------------------|--------------------------------|------------------------------------|
|**Cold start**         |~0ms (V8 isolate)                         |~200ms (Rust)                   |~300ms (container)                  |
|**Max execution**      |30s CPU (free), 15min (paid)              |15 min                          |Unlimited                           |
|**Rust support**       |Via WASM (limited)                        |Native (provided.al2023)        |Native (Docker)                     |
|**Persistent storage** |Durable Objects, KV                       |DynamoDB, S3                    |Volumes                             |
|**Global distribution**|300+ edge locations                       |Per-region                      |35 regions                          |
|**Pricing**            |$5/mo + usage                             |Pay-per-invoke                  |$0 free tier, pay for uptime        |
|**Best for RAPS**      |Webhook gateway, caching, lightweight APIs|Batch processing, scheduled jobs|Long-running DA, persistent services|

**Recommended hybrid approach:**

- **Cloudflare Workers:** webhook-gateway, urn-api, status-proxy, auth-proxy
- **Fly.io Machines:** translation-orchestrator, batch-processor, da-runner, pipeline-scheduler, cache-server
- **AWS Lambda:** only if customer requires AWS-native (enterprise option)

-----

### Agent 6: MCP Coordination Agent

**Purpose:** Provide compound high-level operations that decompose into sequences of lower-level MCP tools. Enable AI agents to interact with APS through intent-based commands rather than procedural API sequences.

**Problem it solves:** RAPS has 51 MCP tools. An AI agent must figure out the correct sequence, handle intermediate state, and manage errors across multi-step workflows. This is fragile and model-dependent.

#### Compound Operations

|Compound MCP Tool                        |Decomposition                                                  |Steps   |
|-----------------------------------------|---------------------------------------------------------------|--------|
|`raps.analyze_model(file)`               |Upload → Translate → Extract Props → Get Metadata → Summarize  |5       |
|`raps.prepare_for_viewing(file)`         |Upload → Translate to SVF2 → Wait → Return viewer URL          |4       |
|`raps.batch_process(files, actions)`     |Upload all → Distribute translations → Collect results → Report|Variable|
|`raps.compare_versions(urn1, urn2)`      |Extract props both → Diff properties → Report changes          |3       |
|`raps.setup_project(name, users)`        |Create bucket → Create project → Add users → Configure webhooks|4       |
|`raps.monitor_changes(project, callback)`|Register webhooks → Start listening → Relay events             |Ongoing |

#### Coordination Logic

```rust
struct CoordinationAgent {
    proxy: Arc<ProxyAgent>,
    distribution: Arc<DistributionAgent>,
    reliability: Arc<ReliabilityAgent>,
    observability: Arc<ObservabilityAgent>,
}

impl CoordinationAgent {
    /// High-level: analyze a model file
    async fn analyze_model(&self, file_path: PathBuf) -> Result<ModelAnalysis> {
        // 1. Upload (Proxy Agent handles caching/connection reuse)
        let urn = self.upload_file(&file_path).await?;
        
        // 2. Translate (Reliability Agent handles retry/circuit breaking)
        let translation = self.reliability.with_retry(
            || self.start_translation(&urn, OutputFormat::Svf2)
        ).await?;
        
        // 3. Wait for completion (Proxy Agent caches status, reducing polls)
        self.wait_for_translation(&translation.urn).await?;
        
        // 4. Extract properties + metadata in parallel (Distribution Agent)
        let (props, metadata) = tokio::join!(
            self.extract_properties(&urn),
            self.get_metadata(&urn)
        );
        
        // 5. Record metrics (Observability Agent)
        self.observability.record_analysis(&urn, &file_path).await;
        
        Ok(ModelAnalysis {
            urn,
            properties: props?,
            metadata: metadata?,
            translation_time: translation.duration,
        })
    }
}
```

#### MCP Tool Registration

When RAPS starts as an MCP server, register both atomic and compound tools:

```json
{
  "tools": [
    // Existing 51 atomic tools...
    { "name": "raps.upload", "description": "Upload a file to APS" },
    { "name": "raps.translate", "description": "Start a translation job" },
    
    // New compound tools
    {
      "name": "raps.analyze_model",
      "description": "Upload, translate, and extract full analysis of a CAD model. Handles authentication, region detection, retries, and progress tracking automatically.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "file_path": { "type": "string", "description": "Path to CAD file (RVT, IFC, DWG, etc.)" },
          "include_properties": { "type": "boolean", "default": true },
          "include_metadata": { "type": "boolean", "default": true }
        },
        "required": ["file_path"]
      }
    },
    {
      "name": "raps.prepare_for_viewing",
      "description": "Upload and translate a file for web viewing. Returns a viewer-ready URN.",
      "inputSchema": {
        "type": "object",
        "properties": {
          "file_path": { "type": "string" },
          "format": { "type": "string", "enum": ["svf", "svf2"], "default": "svf2" }
        },
        "required": ["file_path"]
      }
    }
  ]
}
```

-----

## Runtime Architecture

### Phase 1: In-Process Swarm (Single Binary)

All agents run as `tokio::spawn` tasks within the same process. Communication via typed `mpsc` and `broadcast` channels. Zero additional infrastructure required.

```rust
use tokio::sync::{mpsc, broadcast};

struct SwarmRuntime {
    proxy: Arc<ProxyAgent>,
    distribution: Arc<DistributionAgent>,
    reliability: Arc<ReliabilityAgent>,
    observability: Arc<ObservabilityAgent>,
    coordinator: Arc<CoordinationAgent>,
    // Serverless Dispatch not present in Phase 1
}

// Inter-agent communication
enum SwarmMessage {
    // Proxy → Distribution
    RateBudgetUpdate { api: ApiEndpoint, remaining: u32 },
    
    // Distribution → Reliability
    ExecuteJob { job: WorkUnit },
    
    // Reliability → Proxy
    ApiRequest { request: HttpRequest, response_tx: oneshot::Sender<HttpResponse> },
    
    // Reliability → Observability
    RetryEvent { job_id: JobId, attempt: u32, failure: FailureType },
    CircuitBreakerStateChange { api: ApiEndpoint, new_state: CircuitState },
    
    // Any → Observability
    MetricEvent(Metric),
    AuditEvent(AuditEntry),
    
    // Coordinator → Distribution
    SubmitBatch { batch: Vec<WorkUnit>, priority: Priority },
}

impl SwarmRuntime {
    async fn start() -> Self {
        let (metric_tx, metric_rx) = broadcast::channel(10000);
        let (job_tx, job_rx) = mpsc::channel(1000);
        let (api_tx, api_rx) = mpsc::channel(500);
        
        // Start agents as background tasks
        let proxy = Arc::new(ProxyAgent::new(api_rx));
        let reliability = Arc::new(ReliabilityAgent::new(api_tx.clone()));
        let distribution = Arc::new(DistributionAgent::new(job_tx, proxy.clone()));
        let observability = Arc::new(ObservabilityAgent::new(metric_rx));
        let coordinator = Arc::new(CoordinationAgent::new(
            proxy.clone(), distribution.clone(),
            reliability.clone(), observability.clone(),
        ));
        
        tokio::spawn(proxy.clone().run());
        tokio::spawn(distribution.clone().run());
        tokio::spawn(reliability.clone().run());
        tokio::spawn(observability.clone().run());
        
        Self { proxy, distribution, reliability, observability, coordinator }
    }
}
```

**Lazy initialization:** The full swarm only starts when needed. Simple commands (`raps auth login`, `raps urn encode`) bypass the swarm entirely. Bulk commands and monitored operations (`--wait`, `--diagnose`, `bulk-*`) trigger swarm startup.

```rust
// In main CLI dispatcher
async fn execute_command(cmd: Command) -> Result<()> {
    match cmd {
        // Simple commands — no swarm needed
        Command::Auth(AuthCmd::Login { .. }) => auth::login(args).await,
        Command::Urn(UrnCmd::Encode { input }) => urn::encode(input),
        
        // Commands that benefit from swarm
        Command::Translate(args) if args.wait || args.diagnose => {
            let swarm = SwarmRuntime::start().await;
            swarm.coordinator.translate_and_monitor(args).await
        }
        
        // Commands that require swarm
        Command::BulkTranslate(args) => {
            let swarm = SwarmRuntime::start().await;
            swarm.coordinator.bulk_translate(args).await
        }
        
        // Default: use swarm if available, direct call otherwise
        cmd => {
            if SwarmRuntime::is_beneficial(&cmd) {
                let swarm = SwarmRuntime::start().await;
                swarm.execute(cmd).await
            } else {
                direct_execute(cmd).await
            }
        }
    }
}
```

-----

## Docker Deployment (Phase 2)

### Container Images

```
rapscli/
├── raps-core          ← Full CLI + all agents (alpine-based, ~15MB)
├── raps-proxy         ← CDN/Cache agent + Redis sidecar
├── raps-worker        ← Translation/DA worker (stateless, horizontally scalable)
├── raps-webhook       ← Webhook receiver + relay endpoint
├── raps-dashboard     ← TUI dashboard as web UI (terminal-in-browser or custom web UI)
└── raps-mock          ← Existing raps-mock for integration testing
```

### Dockerfiles

**raps-core (multi-stage build):**

```dockerfile
# Build stage
FROM rust:1.78-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static
WORKDIR /build
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage
FROM alpine:3.19
RUN apk add --no-cache ca-certificates
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/raps /usr/local/bin/raps
ENTRYPOINT ["raps"]
```

**raps-worker (specialized for batch jobs):**

```dockerfile
FROM rapscli/raps-core:latest
ENV RAPS_MODE=worker
ENV RAPS_QUEUE_URL=redis://queue:6379
ENV RAPS_PROXY_URL=http://raps-proxy:8080
ENTRYPOINT ["raps", "swarm", "worker", "start"]
```

### docker-compose.yml (Team Deployment)

```yaml
version: '3.8'

services:
  # Shared proxy/cache — reduces APS API calls across team
  raps-proxy:
    image: rapscli/raps-proxy:latest
    ports:
      - "8080:8080"
    volumes:
      - cache-data:/data
    environment:
      CACHE_TTL: 300
      CACHE_MAX_ENTRIES: 50000
      RATE_LIMIT_BUDGET_ENABLED: "true"
      REDIS_URL: redis://redis:6379
    depends_on:
      - redis
    healthcheck:
      test: ["CMD", "raps", "health", "check"]
      interval: 30s
      timeout: 5s
      retries: 3

  # Worker pool — horizontally scalable
  raps-worker:
    image: rapscli/raps-worker:latest
    deploy:
      replicas: 4
      resources:
        limits:
          memory: 256M
          cpus: "0.5"
    environment:
      PROXY_URL: http://raps-proxy:8080
      QUEUE_URL: redis://redis:6379
      APS_CLIENT_ID: ${APS_CLIENT_ID}
      APS_CLIENT_SECRET: ${APS_CLIENT_SECRET}
    depends_on:
      - raps-proxy
      - redis

  # Webhook receiver with TLS
  raps-webhook:
    image: rapscli/raps-webhook:latest
    ports:
      - "443:443"
    environment:
      WEBHOOK_SECRET: ${WEBHOOK_SECRET}
      RELAY_TARGET: ${WEBHOOK_RELAY_TARGET}
      QUEUE_URL: redis://redis:6379
    volumes:
      - ./certs:/certs:ro

  # Job queue + pub/sub
  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes

  # Web dashboard
  raps-dashboard:
    image: rapscli/raps-dashboard:latest
    ports:
      - "3000:3000"
    environment:
      PROXY_URL: http://raps-proxy:8080
      QUEUE_URL: redis://redis:6379
      METRICS_DB: /data/metrics.db
    volumes:
      - metrics-data:/data

volumes:
  cache-data:
  redis-data:
  metrics-data:
```

### CI/CD Integration Example

```yaml
# .github/workflows/translate-models.yml
name: Nightly Model Translation
on:
  schedule:
    - cron: '0 2 * * *'  # 2 AM UTC daily
  workflow_dispatch:

jobs:
  translate:
    runs-on: ubuntu-latest
    container:
      image: rapscli/raps-worker:latest
    steps:
      - name: Configure RAPS
        run: |
          raps auth login \
            --client-id ${{ secrets.APS_CLIENT_ID }} \
            --client-secret ${{ secrets.APS_CLIENT_SECRET }}

      - name: Scan for changed models
        run: |
          raps acc scan-changes \
            --project "${{ vars.ACC_PROJECT }}" \
            --since "24h" \
            --filter "*.rvt,*.nwc" \
            --output changed-files.json

      - name: Translate changed models
        run: |
          raps bulk-translate \
            --from-file changed-files.json \
            --format svf2 \
            --wait \
            --report translation-report.json

      - name: Extract properties
        run: |
          raps props extract-all \
            --from-file changed-files.json \
            --output properties/

      - name: Upload report
        uses: actions/upload-artifact@v4
        with:
          name: translation-report
          path: |
            translation-report.json
            properties/
```

-----

## Kubernetes Deployment (Phase 3)

### Cluster Architecture

```
┌──────────────── Kubernetes Cluster ─────────────────────┐
│                                                          │
│  ┌─── Namespace: raps-system ──────────────────────┐    │
│  │                                                  │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐      │    │
│  │  │ raps-    │  │ raps-    │  │ raps-    │      │    │
│  │  │ proxy    │  │ coord    │  │ dashboard│      │    │
│  │  │ (2 pods) │  │ (2 pods) │  │ (1 pod)  │      │    │
│  │  └────┬─────┘  └────┬─────┘  └──────────┘      │    │
│  │       │              │                           │    │
│  │  ┌────▼──────────────▼────┐                     │    │
│  │  │    Message Bus         │                     │    │
│  │  │    (NATS JetStream /   │                     │    │
│  │  │     Redis Streams)     │                     │    │
│  │  └────┬──────────────┬────┘                     │    │
│  │       │              │                           │    │
│  │  ┌────▼─────┐  ┌─────▼────┐                     │    │
│  │  │ Worker   │  │ Worker   │  ◄── HPA: 2-20 pods│    │
│  │  │ Pool US  │  │ Pool EU  │      scale on queue │    │
│  │  └──────────┘  └──────────┘      depth          │    │
│  └──────────────────────────────────────────────────┘    │
│                                                          │
│  ┌─── Namespace: raps-tenant-acme ─────────────────┐    │
│  │  Isolated credentials, network policies          │    │
│  │  ┌──────────┐  ┌──────────┐                     │    │
│  │  │ Workers  │  │ Sealed   │                     │    │
│  │  │ (3 pods) │  │ Secrets  │                     │    │
│  │  └──────────┘  └──────────┘                     │    │
│  └──────────────────────────────────────────────────┘    │
│                                                          │
│  ┌─── Namespace: raps-tenant-globex ───────────────┐    │
│  │  Different client, different credentials         │    │
│  │  ...                                             │    │
│  └──────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

### Helm Chart Values

```yaml
# helm/raps/values.yaml
global:
  image:
    registry: ghcr.io/dmytro-yemelianov
    tag: "4.13.0"
  env:
    RUST_LOG: "raps=info,swarm=debug"

# ─────────────────────────────────────
# Proxy / Cache
# ─────────────────────────────────────
proxy:
  replicas: 2
  resources:
    requests: { memory: "64Mi", cpu: "50m" }
    limits: { memory: "256Mi", cpu: "500m" }
  cache:
    backend: redis
    ttl: 300
    maxEntries: 100000
  rateLimiting:
    enabled: true
    strategy: distributed  # coordinate budgets across pods via Redis
  service:
    type: ClusterIP
    port: 8080

# ─────────────────────────────────────
# Workers (auto-scaling)
# ─────────────────────────────────────
workers:
  autoscaling:
    enabled: true
    minReplicas: 2
    maxReplicas: 20
    metrics:
      - type: External
        external:
          metric:
            name: raps_queue_depth
          target:
            type: AverageValue
            averageValue: "5"  # target: 5 pending jobs per worker
  resources:
    requests: { memory: "128Mi", cpu: "100m" }
    limits: { memory: "512Mi", cpu: "1000m" }
  regions:
    us:
      enabled: true
      replicas: 2
      nodeSelector:
        topology.kubernetes.io/region: us-east-1
      env:
        ADS_REGION: "US"
    emea:
      enabled: true
      replicas: 2
      nodeSelector:
        topology.kubernetes.io/region: eu-west-1
      env:
        ADS_REGION: "EMEA"

# ─────────────────────────────────────
# Coordinator
# ─────────────────────────────────────
coordinator:
  replicas: 2
  resources:
    requests: { memory: "64Mi", cpu: "50m" }
    limits: { memory: "256Mi", cpu: "500m" }

# ─────────────────────────────────────
# Webhook Ingress
# ─────────────────────────────────────
webhook:
  enabled: true
  ingress:
    enabled: true
    className: nginx
    host: hooks.rapscli.xyz
    tls:
      enabled: true
      secretName: webhook-tls
    annotations:
      cert-manager.io/cluster-issuer: letsencrypt-prod

# ─────────────────────────────────────
# Dashboard
# ─────────────────────────────────────
dashboard:
  enabled: true
  replicas: 1
  ingress:
    host: dashboard.rapscli.xyz
    tls: true

# ─────────────────────────────────────
# Message Bus
# ─────────────────────────────────────
messageBus:
  type: redis  # Options: redis, nats
  redis:
    enabled: true
    architecture: standalone  # or replication for HA
    auth:
      enabled: true
      existingSecret: raps-redis-secret

# ─────────────────────────────────────
# Multi-tenant support
# ─────────────────────────────────────
tenants:
  enabled: false  # true for SaaS mode
  isolation: namespace  # namespace or networkPolicy
  defaults:
    workerReplicas: 2
    maxWorkerReplicas: 10

# ─────────────────────────────────────
# Monitoring
# ─────────────────────────────────────
monitoring:
  prometheus:
    enabled: true
    serviceMonitor:
      enabled: true
      interval: 30s
  grafana:
    dashboards:
      enabled: true
      # Pre-built dashboards: swarm overview, translation metrics, API health
```

### Key K8s Resources

**HPA for workers (queue-depth scaling):**

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: raps-workers
  namespace: raps-system
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: raps-workers
  minReplicas: 2
  maxReplicas: 20
  metrics:
    - type: External
      external:
        metric:
          name: raps_queue_depth
          selector:
            matchLabels:
              queue: translation-jobs
        target:
          type: AverageValue
          averageValue: "5"
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 30   # scale up quickly
      policies:
        - type: Pods
          value: 4
          periodSeconds: 60
    scaleDown:
      stabilizationWindowSeconds: 300  # scale down slowly (avoid thrashing)
      policies:
        - type: Pods
          value: 2
          periodSeconds: 120
```

**CronJob for scheduled pipelines:**

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: nightly-translate
  namespace: raps-system
spec:
  schedule: "0 2 * * *"
  concurrencyPolicy: Forbid
  successfulJobsHistoryLimit: 7
  failedJobsHistoryLimit: 3
  jobTemplate:
    spec:
      backoffLimit: 2
      activeDeadlineSeconds: 7200  # 2 hour max
      template:
        spec:
          restartPolicy: OnFailure
          containers:
            - name: raps-pipeline
              image: rapscli/raps-worker:latest
              command: ["raps", "pipeline", "run", "nightly-translate"]
              envFrom:
                - secretRef:
                    name: aps-credentials
              resources:
                requests: { memory: "128Mi", cpu: "100m" }
                limits: { memory: "512Mi", cpu: "500m" }
```

**NetworkPolicy for tenant isolation:**

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: tenant-isolation
  namespace: raps-tenant-acme
spec:
  podSelector: {}
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              app.kubernetes.io/part-of: raps-system
  egress:
    - to:
        - namespaceSelector:
            matchLabels:
              app.kubernetes.io/part-of: raps-system
    - to:
        - ipBlock:
            cidr: 0.0.0.0/0
      ports:
        - protocol: TCP
          port: 443  # Only HTTPS to APS APIs
```

-----

## Infrastructure Selection Matrix

```
                         Complexity ─────────────────────────►

               Solo              Team              Enterprise
               developer         5-50 people       Multi-tenant
          ┌────────────────┬──────────────────┬──────────────────┐
          │                │                  │                  │
          │  raps CLI      │ docker-compose   │  Kubernetes      │
          │  (single bin)  │ + Functions      │  + Functions     │
          │                │                  │  + Helm          │
   Cost   │ • Everything   │ • Shared cache   │ • Auto-scaling   │
     │    │   local        │ • Worker pool    │ • Multi-tenant   │
     │    │ • Zero config  │ • Webhook relay  │ • HA/failover    │
     ▼    │ • Zero infra   │ • CI/CD ready    │ • Audit/comply   │
          │                │                  │ • Geo-routing    │
          │  $0            │  $5-50/mo        │  $100-500/mo     │
          └────────────────┴──────────────────┴──────────────────┘
```

-----

## Phased Implementation Plan

### Phase 1: In-Process Agents (Target: Q2 2026)

**Goal:** All agents run within single RAPS binary. No infrastructure changes. No breaking changes to existing CLI.

**Deliverables:**

1. `SwarmRuntime` struct with tokio channel-based inter-agent communication.
1. `ReliabilityAgent` — contextual retry, circuit breaker, singleton token manager.
1. `ProxyAgent` — in-memory LRU cache, region auto-detection, rate budget tracking.
1. `DistributionAgent` — priority queue, rate-aware scheduling, fan-out/fan-in.
1. `ObservabilityAgent` — SQLite metrics, audit log, TUI “Swarm” tab.
1. `CoordinationAgent` — compound MCP operations.
1. Lazy swarm initialization — only start agents when needed.
1. New CLI commands: `raps swarm status`, `raps swarm queue`, `raps swarm metrics`.

**What NOT to build in Phase 1:**

- No ServerlessDispatchAgent
- No Docker images
- No external Redis/queue dependency
- No network-based inter-agent communication

**Testing:**

- Unit tests per agent with mock channels.
- Integration tests: full swarm with raps-mock backend.
- Benchmark: measure overhead of swarm vs direct API calls (target: <5ms per operation).

### Phase 2: Docker + Serverless (Target: Q3 2026)

**Goal:** Team-scale deployment. Shared caching. Serverless webhook and fire-and-forget.

**Deliverables:**

1. Docker images for each component (multi-stage Rust builds, alpine-based).
1. `docker-compose.yml` for team deployment.
1. Redis integration for shared cache and job queue.
1. Cloudflare Worker: webhook-gateway.
1. Fly.io Machine: translation-orchestrator (fire-and-forget).
1. `ServerlessDispatchAgent` in CLI — dispatches to cloud workers.
1. `raps pipeline create` command for scheduled operations.
1. CI/CD examples for GitHub Actions and GitLab CI.

### Phase 3: Kubernetes (Target: Q1 2027)

**Goal:** Enterprise multi-tenant deployment. Auto-scaling. Full observability.

**Deliverables:**

1. Helm chart with all components.
1. HPA configuration for workers (queue-depth scaling).
1. Multi-tenant namespace isolation with NetworkPolicies.
1. Prometheus metrics export + Grafana dashboards.
1. CronJob templates for scheduled pipelines.
1. Sealed Secrets integration for credential management.
1. Web-based dashboard (replace/supplement TUI).

### Phase 4: Platform / SaaS (Target: Q3 2027)

**Goal:** RAPS as a managed service. API-first. Cross-platform expansion.

**Deliverables:**

1. `api.rapscli.xyz` — public API with API keys.
1. Pay-per-operation pricing model.
1. Web console for management.
1. PTC Onshape API integration (first cross-platform expansion).
1. Dassault 3DEXPERIENCE API integration.
1. Siemens Teamcenter API integration.

-----

## Configuration Reference

### Swarm Configuration File

```toml
# ~/.config/raps/swarm.toml

[swarm]
enabled = true          # false = direct API calls, no agents
lazy_init = true        # only start agents when needed
log_level = "info"      # debug for troubleshooting

[proxy]
cache_enabled = true
cache_backend = "memory"  # "memory" (Phase 1) or "redis" (Phase 2+)
cache_ttl_seconds = 300
cache_max_entries = 10000
auto_region = true
connection_pool_size = 20
rate_limit_proactive = true

[proxy.redis]  # Phase 2+
url = "redis://localhost:6379"
prefix = "raps:cache:"

[distribution]
max_queue_size = 10000
default_priority = "normal"
da_max_concurrent = 50

[reliability]
max_retries = 5
circuit_breaker_threshold = 5       # failures before opening
circuit_breaker_timeout_seconds = 30  # probe interval when open
token_proactive_refresh_seconds = 300 # refresh 5 min before expiry

[reliability.checkpoint]
enabled = true
db_path = "~/.local/share/raps/checkpoints.db"

[observability]
metrics_enabled = true
metrics_db_path = "~/.local/share/raps/metrics.db"
audit_enabled = true
audit_path = "~/.local/share/raps/audit/"
audit_retention_days = 90

[observability.token_budget]
monthly_limit = 1000.0  # optional: alert at threshold
alert_threshold = 0.8   # alert at 80% consumed

[serverless]  # Phase 2+
enabled = false
provider = "fly"        # "fly", "cloudflare", "aws"
webhook_url = "https://hooks.rapscli.xyz"

[serverless.fly]
app_name = "raps-worker"
region = "iad"  # primary region

[serverless.cloudflare]
account_id = ""
worker_name = "raps-webhook"

[serverless.notifications]
slack_webhook = ""
email = ""
```

-----

## Rust Crate Dependencies (New)

```toml
# Cargo.toml additions for swarm feature

[features]
swarm = [
    "dep:lru",
    "dep:dashmap",
    "dep:hdrhistogram",
    "dep:sqlx",
    "dep:bincode",
]
swarm-serverless = ["swarm", "dep:reqwest"]  # Phase 2
swarm-docker = ["swarm"]                      # same binary, Docker packaging

[dependencies]
# Phase 1: In-process agents
lru = { version = "0.12", optional = true }
dashmap = { version = "5", optional = true }
hdrhistogram = { version = "7", optional = true }
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio"], optional = true }
bincode = { version = "1", optional = true }

# Already in RAPS (confirm versions)
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
```

-----

## Strategic Value Summary

|Without Agent Swarm              |With Agent Swarm                        |
|---------------------------------|----------------------------------------|
|CLI that wraps API calls         |Reliable orchestration platform         |
|Single developer, single terminal|Teams with distributed workflows        |
|Rate limit → error               |Rate limit → automatic queuing          |
|Translation failed → manual retry|Translation failed → contextual recovery|
|Polling every 5s                 |Smart cache + webhook relay             |
|No operational visibility        |Full metrics, analytics, audit trail    |
|Only Autodesk                    |Foundation for PTC/Dassault/Siemens     |
|Dev tool                         |Enterprise infrastructure               |

**For the May 2026 Autodesk demo:** Position RAPS not as “a CLI that calls APS APIs” but as “an intelligent orchestration layer that makes APS reliable, observable, and scalable.” This is infrastructure Autodesk might want to recommend or integrate, not just a convenience wrapper.

-----

## Open Questions for Implementation

1. **Message bus for Phase 2:** Redis Streams vs NATS JetStream? Redis is simpler and already needed for caching. NATS is more purpose-built for messaging but adds another dependency.
1. **Checkpoint serialization:** bincode (fast, compact, Rust-only) vs MessagePack (cross-language compatible for future Python bindings access)?
1. **Serverless provider lock-in:** Should the serverless dispatch be provider-agnostic from the start (trait-based abstraction), or optimize for Fly.io first and abstract later?
1. **TUI vs Web dashboard:** Phase 3 web dashboard — build custom (Rust/HTMX?) or use Grafana with custom dashboards? Grafana is less work but less integrated.
1. **Feature flag granularity:** Should each agent be independently toggleable, or is swarm on/off sufficient?
1. **Metrics export format:** Prometheus (pull-based, K8s native) vs OpenTelemetry (push-based, vendor-agnostic)? Both? OTel collector can export to Prometheus.
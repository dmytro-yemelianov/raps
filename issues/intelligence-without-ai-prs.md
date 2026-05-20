# Intelligence Without AI — Proposed PRs

## High Priority

### PR-1: Duplicate detection before upload
Before uploading, compute SHA-256 of local file and compare against existing object ETag/checksum.
Skip upload and reuse existing URN if identical. Avoids redundant translations and OSS storage costs.
- Crates: `raps-oss`
- Commands: `objects upload`, `objects batch-upload`

### PR-2: Pipeline dependency graph resolution
Parse `inputs`/`outputs` in pipeline YAML steps and auto-sort execution order using topological sort.
Detect circular dependencies and fail fast with a clear error.
- Crates: `raps-pipeline` (or wherever pipeline lives)
- Commands: `pipeline run`

### PR-3: Pre-flight pipeline validation
Before executing a pipeline, resolve all variable references, validate that referenced buckets/projects
exist via API, and surface all errors at once rather than failing mid-run.
Dry-run mode should be a full validation pass.
- Crates: `raps-pipeline`
- Commands: `pipeline run --dry-run`

### PR-4: Bandwidth-aware chunk sizing for multipart uploads
Measure actual upload throughput during multipart uploads. Adjust chunk size dynamically:
fast connection → larger chunks, degraded connection → smaller chunks.
- Crates: `raps-oss`
- Commands: `objects upload` (multipart path)

### PR-5: Translation state machine (end-to-end workflow)
After submitting a translation, automatically poll status and trigger configurable downstream steps
(e.g. download derivative, bind to ACC folder) when status reaches `success` or `failed`.
Single command for full model pipeline.
- Crates: `raps-md` (model derivative)
- Commands: `translation submit --watch --then download`

---

## Medium Priority

### PR-6: Rate limit awareness
Parse `X-RateLimit-Remaining` / `X-RateLimit-Reset` headers. Auto-throttle concurrent requests
before hitting limits rather than reacting after a 429.
- Crates: `raps-kernel` (HTTP client layer)
- Commands: all

### PR-7: Adaptive retry backoff
Track historical failure patterns per endpoint. If an endpoint consistently times out, extend
timeout proactively. Persist stats to `~/.raps/endpoint-stats.json`.
- Crates: `raps-kernel`
- Commands: all

### PR-8: ETA estimation for uploads and translations
Based on observed throughput over last N chunks, display accurate time-to-completion.
For translations, use historical job duration by file size + format.
- Crates: `raps-oss`, `raps-md`
- Commands: `objects upload`, `translation status --watch`

### PR-9: Fuzzy command correction
Levenshtein distance matching on mistyped subcommands.
`raps bukcets list` → `Did you mean: buckets list?`
- Crates: `raps` (CLI entry point)

### PR-10: Format auto-detection for translation
Inspect file extension and magic bytes to automatically select the right output format
(SVF2, OBJ, STL, STEP) without requiring `--format` flag.
- Crates: `raps-md`
- Commands: `translation submit`

---

## Lower Priority

### PR-11: `raps doctor` — config drift detection
Compare current credentials/config against what the APS token actually scopes permit.
Warn about missing scopes before a long operation fails mid-way.
- Crates: `raps-kernel`, `raps` (CLI)
- Commands: `raps doctor` (new top-level command)

### PR-12: Operation history and replay
Log commands + parameters to `~/.raps/history.json`.
Add `raps history`, `raps replay <n>` for repeating operations.
- Crates: `raps` (CLI)
- Commands: `raps history`, `raps replay`

### PR-13: Profile auto-switching via project context
Detect hub/project context from `.raps-project` file in current directory or env vars.
Automatically select matching named profile without `--profile` flag.
- Crates: `raps-kernel`
- Commands: all

### PR-14: Webhook health monitoring
Aggregate delivery success/failure counts from APS callback data.
Surface `raps webhooks status` showing endpoint health and recent delivery stats.
- Crates: `raps-webhooks`
- Commands: `webhooks status` (new subcommand)

### PR-15: Cost/size estimation before upload
Calculate expected OSS storage and estimated translation time based on file size + format.
Warn (or prompt) if above a configurable threshold.
- Crates: `raps-oss`, `raps-md`
- Commands: `objects upload`, `translation submit`

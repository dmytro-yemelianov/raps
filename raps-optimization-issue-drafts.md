# RAPS Optimization — GitHub Issue Drafts

This document contains ready-to-post GitHub issue drafts for performance and reliability optimizations in the RAPS CLI.  
Each issue includes: Problem, Goal, Proposal, CLI/UX notes, Acceptance criteria, and Out of scope.

---

## 1) Parallel multipart uploads for OSS (major throughput win)

### Problem
Multipart uploads currently upload parts sequentially, underutilizing bandwidth/CPU and making large uploads slower than necessary.

### Goal
Upload multipart parts concurrently (bounded concurrency) while preserving resumability, correctness, and predictable UX.

### Proposal
- Implement parallel part upload for OSS multipart uploads with a semaphore (bounded concurrency).
- Preserve ordering rules: parts may upload out-of-order, but completion must include the full ordered list of `{partNumber, ETag}`.
- Reuse buffers / avoid per-part allocations where feasible.
- Keep `--resume` functionality working (state file updates).

### CLI / UX
- Add an option: `raps object upload --multipart-parallel <N>` (or reuse global `--concurrency`).
- Deterministic progress/logging; avoid garbled concurrent output.

### Implementation notes
- Generate (or refresh) signed URLs for parts as needed.
- Resume state should track uploaded part numbers + ETags.
- Avoid writing state on every part if expensive; consider batching or periodic flush.

### Acceptance criteria
- Throughput improves significantly for large files (e.g., 2–5x on typical home/office internet).
- `--resume` works when interrupted mid-upload.
- Correct ETag list passed to complete-multipart.
- Tests: unit test for state merging + integration test (mocked) for parallel completion ordering.

### Out of scope
- Cross-file parallelism (handled by `upload-batch`).

---

## 2) Cache downloads + derivative artifacts with hardlink materialization

### Problem
Repeated workflows re-download the same large artifacts (OSS downloads, Model Derivative outputs), wasting time and bandwidth—especially in CI/CD and pipelines.

### Goal
Introduce a content-addressed cache so repeated runs become near-instant.

### Proposal
- Add cache layer for:
  - OSS object downloads
  - Model Derivative downloads (SVF/SVF2 packs, thumbnails, export formats)
  - Optional: manifest/status JSON responses
- Cache key:
  - Prefer `ETag` / last-modified + size
  - Fallback to hashing when needed
- Materialize to destination via hardlink when possible; fallback to copy.

### CLI / UX
- Global flags:
  - `--cache on|off` (default on for downloads)
  - `--cache-dir <path>`
  - `--refresh` (ignore cache)
  - `--offline` (fail if not cached)
- Command flags override global defaults.

### Acceptance criteria
- Second run avoids network calls (except `HEAD` if needed).
- Cache works across commands that reference the same artifact.
- Hardlink is used when filesystem supports it; graceful fallback otherwise.
- Cache growth is bounded (TTL and/or size cap) and behavior is documented.

### Out of scope
- Distributed/shared remote cache.

---

## 3) Stream list outputs + add `--limit/--pages` to avoid fetch-all bottlenecks

### Problem
List commands that fetch all pages before printing can be slow and memory-heavy on large datasets.

### Goal
Make list commands responsive and scalable with streaming output and paging controls.

### Proposal
- Add streaming output mode to print rows as pages arrive (optionally buffered).
- Add paging controls:
  - `--limit <N>` (total items)
  - `--pages <N>` (max pages)
  - `--page-size <N>` (API request size where supported)
- Support streaming-friendly formats (e.g., `--jsonl` for newline-delimited JSON).

### CLI / UX
- Add `--stream` flag OR default streaming when output is `--jsonl`.
- Keep current default behavior for human-friendly table output, but enable fast CI usage.

### Acceptance criteria
- `--limit 100` returns quickly even for huge inventories.
- Memory usage does not scale with total inventory when streaming is enabled.
- Docs updated with CI-friendly examples.

### Out of scope
- Full interactive pagination UI (TUI).

---

## 4) Add HTTP Range “inspect” commands for large bundles (zip/pack) without full download

### Problem
Users often need to inspect contents of large package-like artifacts (zip/bundles) but are forced to download the entire file.

### Goal
Enable fast inspection using HTTP Range requests wherever supported.

### Proposal
- Implement `Range` support for signed URLs and object URLs where possible.
- Add commands (naming bikeshedding):
  - `raps object inspect-zip <bucket/key> --list`
  - `raps translate inspect <urn> --list-files`
- Technique:
  - Fetch end-of-file to read zip central directory.
  - Fetch only required ranges for small internal files.

### Acceptance criteria
- Can list zip contents with <1% of bytes downloaded for typical archives.
- Graceful fallback when server does not support ranges.
- Clear messaging when inspect is not possible.

### Out of scope
- Full random-access extraction for all archive formats.

---

## 5) “Strict / fast mode” for CI: no prompts, no guessing, fail fast

### Problem
In CI/CD, interactive prompts and ambiguous defaults can cause hangs or non-deterministic behavior.

### Goal
Provide a mode optimized for automation: strict, deterministic, and fast.

### Proposal
- Introduce `--strict` and/or strengthen `--non-interactive`:
  - Fail immediately when required inputs are missing.
  - Disable any auto-selection behavior.
  - Require explicit bucket/project IDs when ambiguity exists.
- Optional: add `--no-fallbacks` to disable multi-path heuristics unless explicitly requested.

### Acceptance criteria
- All commands honor strict mode consistently (no hidden prompts).
- Errors include exact missing parameters and a concrete example of how to fix.
- Docs include CI examples.

### Out of scope
- Breaking changes to default interactive behavior (unless behind flags).

---

## 6) Unify concurrency + retry policy across commands (one mental model)

### Problem
Parallelism and retry behavior differ between commands, confusing users and leaving performance/reliability on the table.

### Goal
Centralize concurrency and retry handling so network-heavy operations behave consistently.

### Proposal
- Global knobs:
  - `--concurrency <N>` applies to all parallelizable operations.
  - `--retries <N>`, `--retry-backoff <strategy>`, `--retry-status <codes>`
- Idempotent GETs retry by default; unsafe operations use conservative defaults or require opt-in.
- Adopt a shared `execute_with_retry` wrapper across clients.

### Acceptance criteria
- All relevant commands honor global concurrency and retry flags.
- Docs explain defaults and when retries happen.
- Tests for retry classification (429/5xx/network).

### Out of scope
- Circuit breaker / global rate limiter.

---

## 7) Reduce resumable upload state I/O (batch state writes)

### Problem
Resumable uploads that write state after every part can incur heavy disk I/O overhead for very large files.

### Goal
Keep resumability while reducing filesystem churn.

### Proposal
- Save state:
  - every X parts and/or every Y seconds
  - always save on completion
  - best-effort save on Ctrl+C / termination signals
- Use atomic writes (tmp + rename) to avoid corruption.
- Add flags:
  - `--state-flush-interval`
  - `--state-flush-parts`

### Acceptance criteria
- Measurable speedup on slower disks for large files.
- State remains correct after interruption.
- No corrupted state files (or corruption is detectable + recoverable).

### Out of scope
- Storing state in sqlite (possible future improvement).

---

## 8) Fix blocking calls inside async paths (`spawn_blocking` where appropriate)

### Problem
Some flows (interactive prompts, OAuth callback waiting, heavy filesystem work) can block async runtime threads.

### Goal
Make async behavior robust and avoid unintended stalls.

### Proposal
- Wrap blocking sections in `tokio::task::spawn_blocking`:
  - `dialoguer` prompts
  - blocking HTTP server recv loops (if used)
  - heavy filesystem operations if measurable
- Alternatively, replace specific flows with async-native implementations when feasible.

### Acceptance criteria
- No long blocking waits on async worker threads.
- No functional regressions in auth flows and prompts.
- Add a smoke test or regression test around the affected flows.

### Out of scope
- Full rewrite of the interactive subsystem.

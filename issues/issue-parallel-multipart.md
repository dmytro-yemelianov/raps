## Problem
Multipart uploads currently upload parts sequentially, underutilizing bandwidth/CPU and making large uploads slower than necessary.

## Goal
Upload multipart parts concurrently (bounded concurrency) while preserving resumability, correctness, and predictable UX.

## Proposal
- Implement parallel part upload for OSS multipart uploads with a semaphore (bounded concurrency).
- Preserve ordering rules: parts may upload out-of-order, but completion must include the full ordered list of `{partNumber, ETag}`.
- Reuse buffers / avoid per-part allocations where feasible.
- Keep `--resume` functionality working (state file updates).

## CLI / UX
- Add an option: `raps object upload --multipart-parallel <N>` (or reuse global `--concurrency`).
- Deterministic progress/logging; avoid garbled concurrent output.

## Implementation notes
- Generate (or refresh) signed URLs for parts as needed.
- Resume state should track uploaded part numbers + ETags.
- Avoid writing state on every part if expensive; consider batching or periodic flush.

## Acceptance criteria
- Throughput improves significantly for large files (e.g., 2–5x on typical home/office internet).
- `--resume` works when interrupted mid-upload.
- Correct ETag list passed to complete-multipart.
- Tests: unit test for state merging + integration test (mocked) for parallel completion ordering.

## Out of scope
- Cross-file parallelism (handled by `upload-batch`).

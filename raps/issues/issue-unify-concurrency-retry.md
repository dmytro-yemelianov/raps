## Problem
Parallelism and retry behavior differ between commands, confusing users and leaving performance/reliability on the table.

## Goal
Centralize concurrency and retry handling so network-heavy operations behave consistently.

## Proposal
- Global knobs:
  - `--concurrency <N>` applies to all parallelizable operations.
  - `--retries <N>`, `--retry-backoff <strategy>`, `--retry-status <codes>`
- Idempotent GETs retry by default; unsafe operations use conservative defaults or require opt-in.
- Adopt a shared `execute_with_retry` wrapper across clients.

## Acceptance criteria
- All relevant commands honor global concurrency and retry flags.
- Docs explain defaults and when retries happen.
- Tests for retry classification (429/5xx/network).

## Out of scope
- Circuit breaker / global rate limiter.

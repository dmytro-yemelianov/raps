## Problem
Repeated workflows re-download the same large artifacts (OSS downloads, Model Derivative outputs), wasting time and bandwidth—especially in CI/CD and pipelines.

## Goal
Introduce a content-addressed cache so repeated runs become near-instant.

## Proposal
- Add cache layer for:
  - OSS object downloads
  - Model Derivative downloads (SVF/SVF2 packs, thumbnails, export formats)
  - Optional: manifest/status JSON responses
- Cache key:
  - Prefer `ETag` / last-modified + size
  - Fallback to hashing when needed
- Materialize to destination via hardlink when possible; fallback to copy.

## CLI / UX
- Global flags:
  - `--cache on|off` (default on for downloads)
  - `--cache-dir <path>`
  - `--refresh` (ignore cache)
  - `--offline` (fail if not cached)
- Command flags override global defaults.

## Acceptance criteria
- Second run avoids network calls (except `HEAD` if needed).
- Cache works across commands that reference the same artifact.
- Hardlink is used when filesystem supports it; graceful fallback otherwise.
- Cache growth is bounded (TTL and/or size cap) and behavior is documented.

## Out of scope
- Distributed/shared remote cache.

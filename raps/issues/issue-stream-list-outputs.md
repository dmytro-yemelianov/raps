## Problem
List commands that fetch all pages before printing can be slow and memory-heavy on large datasets.

## Goal
Make list commands responsive and scalable with streaming output and paging controls.

## Proposal
- Add streaming output mode to print rows as pages arrive (optionally buffered).
- Add paging controls:
  - `--limit <N>` (total items)
  - `--pages <N>` (max pages)
  - `--page-size <N>` (API request size where supported)
- Support streaming-friendly formats (e.g., `--jsonl` for newline-delimited JSON).

## CLI / UX
- Add `--stream` flag OR default streaming when output is `--jsonl`.
- Keep current default behavior for human-friendly table output, but enable fast CI usage.

## Acceptance criteria
- `--limit 100` returns quickly even for huge inventories.
- Memory usage does not scale with total inventory when streaming is enabled.
- Docs updated with CI-friendly examples.

## Out of scope
- Full interactive pagination UI (TUI).

## Problem
Resumable uploads that write state after every part can incur heavy disk I/O overhead for very large files.

## Goal
Keep resumability while reducing filesystem churn.

## Proposal
- Save state:
  - every X parts and/or every Y seconds
  - always save on completion
  - best-effort save on Ctrl+C / termination signals
- Use atomic writes (tmp + rename) to avoid corruption.
- Add flags:
  - `--state-flush-interval`
  - `--state-flush-parts`

## Acceptance criteria
- Measurable speedup on slower disks for large files.
- State remains correct after interruption.
- No corrupted state files (or corruption is detectable + recoverable).

## Out of scope
- Storing state in sqlite (possible future improvement).

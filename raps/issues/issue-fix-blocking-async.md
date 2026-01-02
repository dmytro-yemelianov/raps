## Problem
Some flows (interactive prompts, OAuth callback waiting, heavy filesystem work) can block async runtime threads.

## Goal
Make async behavior robust and avoid unintended stalls.

## Proposal
- Wrap blocking sections in `tokio::task::spawn_blocking`:
  - `dialoguer` prompts
  - blocking HTTP server recv loops (if used)
  - heavy filesystem operations if measurable
- Alternatively, replace specific flows with async-native implementations when feasible.

## Acceptance criteria
- No long blocking waits on async worker threads.
- No functional regressions in auth flows and prompts.
- Add a smoke test or regression test around the affected flows.

## Out of scope
- Full rewrite of the interactive subsystem.

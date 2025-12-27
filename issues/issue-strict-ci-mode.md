## Problem
In CI/CD, interactive prompts and ambiguous defaults can cause hangs or non-deterministic behavior.

## Goal
Provide a mode optimized for automation: strict, deterministic, and fast.

## Proposal
- Introduce `--strict` and/or strengthen `--non-interactive`:
  - Fail immediately when required inputs are missing.
  - Disable any auto-selection behavior.
  - Require explicit bucket/project IDs when ambiguity exists.
- Optional: add `--no-fallbacks` to disable multi-path heuristics unless explicitly requested.

## Acceptance criteria
- All commands honor strict mode consistently (no hidden prompts).
- Errors include exact missing parameters and a concrete example of how to fix.
- Docs include CI examples.

## Out of scope
- Breaking changes to default interactive behavior (unless behind flags).

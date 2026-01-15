# Implementation Plan: Non-interactive Mode

**Branch**: `005-non-interactive-mode` | **Date**: 2026-01-15 | **Spec**: [specs/005-non-interactive-mode/spec.md](../spec.md)
**Input**: Feature specification from `/specs/005-non-interactive-mode/spec.md`

## Summary

Ensure all CLI commands support full non-interactive execution. This involves auditing "create" commands to ensure all required parameters have flags, and "delete" commands to ensure `--yes` is respected and required in non-interactive mode. The `raps-kernel` prompts module should enforce `non_interactive` mode checks.

## Technical Context

**Language/Version**: Rust 1.88
**Primary Dependencies**: `raps-kernel`, `clap`
**Project Type**: Rust Workspace (CLI)

## Constitution Check

- **II. Automation-First Design**: ✅

## Project Structure

### Source Code

```text
raps-cli/src/commands/
├── bucket.rs            # Update create/delete
├── translate.rs         # Update start
├── issue.rs             # Update create
├── reality.rs           # Update create
├── folder.rs            # Update create
├── webhook.rs           # Update create
└── ...

raps-kernel/src/
└── prompts.rs           # Enhance to fail-fast in non-interactive
```

**Structure Decision**: Modify individual commands to accept Option<T> args, and if None, try prompt. If prompt fails (non-interactive), return error.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| None | | |
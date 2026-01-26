# Implementation Plan: RCW Migration Automation

**Branch**: `001-rcw-migration` | **Date**: 2026-01-23 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-rcw-migration/spec.md`

## Summary

Implement CLI commands for migrating Revit Cloud Worksharing (RCW) models from BIM 360 to ACC Docs using Design Automation. This extends the existing `raps da` command group with RCW-specific subcommands that orchestrate the migration workflow: configure automation environment, list eligible RCW models, migrate files, and monitor job status.

## Technical Context

**Language/Version**: Rust 1.88, Edition 2024
**Primary Dependencies**: clap 4.5, reqwest 0.11, tokio 1.49, serde, raps-kernel, raps-da, raps-dm
**Storage**: N/A (stateless CLI, uses APS cloud storage)
**Testing**: cargo test, cargo nextest, raps-mock for integration tests
**Target Platform**: Cross-platform CLI (Windows, macOS, Linux)
**Project Type**: Single workspace with multiple crates (microkernel architecture)
**Performance Goals**: Migration jobs are async; status polling every 5 seconds
**Constraints**: Requires both 2-legged and 3-legged OAuth tokens; RCW models only (C4RModel type)
**Scale/Scope**: Batch migrations of up to 50 files per invocation

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

No constitution defined for this project. Proceeding with standard Rust/RAPS conventions:

- [x] **Library-First**: New functionality in `raps-da` crate, CLI in `raps-cli`
- [x] **CLI Interface**: Commands follow existing `raps da` pattern
- [x] **Testing**: Unit tests in lib.rs, integration tests in tests/
- [x] **Simplicity**: Minimal new abstractions; extend existing DA client

## Project Structure

### Documentation (this feature)

```text
specs/001-rcw-migration/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── rcw-commands.md  # CLI command specifications
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
raps-da/src/
├── lib.rs               # Existing - extend with RCW types and methods
└── rcw.rs               # NEW - RCW migration specific logic

raps-dm/src/
└── lib.rs               # Existing - extend Version with relationships

raps-cli/src/
├── commands/
│   └── da.rs            # Existing - add RCW subcommands
└── main.rs              # Existing - no changes needed

tests/
├── integration/
│   └── rcw_migration.rs # NEW - integration tests for RCW commands
└── unit/
    └── rcw_types.rs     # NEW - unit tests for RCW types
```

**Structure Decision**: Extend existing `raps-da` crate with RCW-specific module. No new crates needed as RCW migration is a specialized use of Design Automation.

## Complexity Tracking

No constitution violations to justify.

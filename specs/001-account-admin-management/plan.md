# Implementation Plan: Account Admin Bulk Management Tool

**Branch**: `001-account-admin-management` | **Date**: 2026-01-16 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-account-admin-management/spec.md`

## Summary

This feature adds bulk user management capabilities for ACC/BIM 360 account administrators, enabling them to add, remove, and update user roles and folder permissions across thousands of projects in a single operation. The implementation will extend the existing `raps-acc` crate with new API clients for Account Admin API and Project Users API, plus a new `raps-admin` crate for bulk operation orchestration with progress tracking, retry logic, and state persistence.

## Technical Context

**Language/Version**: Rust 1.88+ (edition 2024)
**Primary Dependencies**: clap, reqwest, tokio, serde, indicatif, directories, keyring (existing workspace dependencies)
**Storage**: Local filesystem for operation state (user's config directory via `directories` crate)
**Testing**: cargo test, cargo nextest, raps-mock for API mocking
**Target Platform**: Windows, macOS, Linux (cross-platform CLI)
**Project Type**: Single workspace with multiple crates (existing microkernel architecture)
**Performance Goals**: Process 3,000 projects in <30 minutes, 5,000 projects in <45 minutes
**Constraints**: APS API rate limits (~600 requests/minute), configurable concurrency (default 10)
**Scale/Scope**: Support up to 5,000 projects per bulk operation

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-Native & Modular Workspace | ✅ PASS | New `raps-admin` crate for bulk operations, extends `raps-acc` |
| II. Automation-First Design | ✅ PASS | JSON/YAML output via `--output`, standardized exit codes, non-interactive batch mode |
| III. Secure by Default | ✅ PASS | Uses existing keyring storage, no new credential handling |
| IV. Comprehensive Observability | ✅ PASS | Progress bars via indicatif, operation audit logging |
| V. Quality & Reliability | ✅ PASS | Unit + integration tests required, existing CI gates |

**All gates PASS. Proceeding with design.**

## Project Structure

### Documentation (this feature)

```text
specs/001-account-admin-management/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
# New crate for account admin bulk operations
raps-admin/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Crate root, re-exports
│   ├── bulk/            # Bulk operation orchestration
│   │   ├── mod.rs
│   │   ├── executor.rs  # Parallel execution engine
│   │   ├── state.rs     # Operation state persistence
│   │   └── retry.rs     # Retry logic with backoff
│   ├── operations/      # Specific bulk operation types
│   │   ├── mod.rs
│   │   ├── add_user.rs
│   │   ├── remove_user.rs
│   │   ├── update_role.rs
│   │   └── folder_rights.rs
│   └── report.rs        # Result reporting and export
└── tests/
    ├── bulk_tests.rs
    └── state_tests.rs

# Extended raps-acc crate (existing)
raps-acc/
├── src/
│   ├── lib.rs           # Add new client exports
│   ├── admin.rs         # NEW: Account Admin API client
│   └── users.rs         # NEW: Project Users API client

# CLI commands (existing raps-cli)
raps-cli/
├── src/
│   └── commands/
│       └── admin.rs     # NEW: Admin subcommands

tests/
├── integration/
│   └── admin_bulk_tests.rs  # End-to-end bulk operation tests
```

**Structure Decision**: Follows existing microkernel architecture pattern. The `raps-admin` crate encapsulates bulk operation logic (orchestration, state management, retry) while `raps-acc` is extended with the underlying API clients. This maintains separation of concerns and allows independent testing.

## Constitution Check (Post-Design Re-evaluation)

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-Native & Modular Workspace | ✅ PASS | `raps-admin` crate with clear public APIs, independently testable |
| II. Automation-First Design | ✅ PASS | Exit codes defined (0-3), JSON/YAML/CSV output, `--yes` flag for non-interactive |
| III. Secure by Default | ✅ PASS | Reuses existing keyring auth, no new credential storage |
| IV. Comprehensive Observability | ✅ PASS | Progress bars, verbose/debug modes, HTTP logging via existing infrastructure |
| V. Quality & Reliability | ✅ PASS | Test files specified, uses raps-mock for API mocking |

**Post-design assessment**: All principles satisfied. Design follows existing patterns and maintains consistency with RAPS architecture.

## Complexity Tracking

No violations requiring justification. Design follows existing patterns.

# Implementation Plan: Fix API Alignment Bugs

**Branch**: `001-fix-api-alignment-bugs` | **Date**: 2026-02-24 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-fix-api-alignment-bugs/spec.md`

## Summary

Fix 6 bugs where RAPS code diverges from APS OpenAPI specifications. Two are BLOCKING (silent data truncation from missing pagination, wrong data center from hardcoded region). Four are HIGH severity (forced manifest deletion, conflicting project ID normalization, token refresh race condition, hardcoded MIME type). Changes span 5 service crates and the CLI, using patterns already established in the codebase (JSON:API links, OSS region headers, tokio synchronization).

## Technical Context

**Language/Version**: Rust 1.88, Edition 2024
**Primary Dependencies**: reqwest (HTTP), tokio (async runtime), clap (CLI), serde (serialization) — all existing workspace deps
**Storage**: N/A (no storage changes; token caching uses existing in-memory + keyring)
**Testing**: `cargo test`, `cargo nextest run`
**Target Platform**: Cross-platform (Linux, macOS, Windows) — same as existing CLI
**Project Type**: Rust workspace (10 crates)
**Performance Goals**: Pagination must handle 500+ items without excessive memory; token refresh must not block concurrent requests beyond the refresh duration
**Constraints**: No new external crate dependencies; no breaking public API changes except `translate()` signature and project ID function consolidation
**Scale/Scope**: 6 independent bug fixes across 5 crates + CLI, ~8 files modified

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Research Check

| Principle | Status | Notes |
|-----------|--------|-------|
| **I. Rust-Native & Modular Workspace** | PASS | All changes stay within their respective `raps-*` crates. Project ID functions become shared public API in raps-acc. No cross-crate boundary violations. |
| **II. Automation-First Design** | PASS | No interactive prompts added. New `--region` and `--force` CLI flags follow existing patterns. Exit codes unchanged. JSON/YAML output unaffected. |
| **III. Secure by Default** | PASS | Token refresh fix improves security (prevents unnecessary token clearing). No new credential handling. Secret redaction unchanged. |
| **IV. Comprehensive Observability** | PASS | Pagination loop can log page count. Region header is visible in debug logs. No new secrets to redact. |
| **V. Quality & Reliability** | PASS | Each fix requires unit tests. `cargo fmt`, `cargo clippy`, `cargo test` gates apply. CLI help text updated for new flags. Deprecation notice for force-translate default change. |

### Post-Design Re-check

| Principle | Status | Notes |
|-----------|--------|-------|
| **I. Rust-Native & Modular Workspace** | PASS | MdRegion enum in raps-derivative (not shared with raps-oss Region). Project ID functions public in raps-acc/src/lib.rs. TokenCache private to raps-kernel. MIME function private to raps-reality. Each crate independently testable. |
| **II. Automation-First Design** | PASS | `--region` accepts string value suitable for scripting. `--force` is a boolean flag. Both work in non-interactive mode. |
| **III. Secure by Default** | PASS | Mutex-based token refresh eliminates race condition that could cause unnecessary auth failures. |
| **IV. Comprehensive Observability** | PASS | Pagination logs page progress. Region value included in request headers visible in debug mode. |
| **V. Quality & Reliability** | PASS | Tests defined for: pagination loop (mock multi-page responses), region enum parsing, project ID normalization (both directions), MIME mapping (all extensions + fallback). Token refresh tested with concurrent simulation. |

**Gate result**: ALL PASS — no violations.

## Project Structure

### Documentation (this feature)

```text
specs/001-fix-api-alignment-bugs/
├── plan.md              # This file
├── research.md          # Phase 0: research decisions R1-R6
├── data-model.md        # Phase 1: entity definitions
├── quickstart.md        # Phase 1: implementation guide
├── contracts/
│   └── api-changes.md   # Phase 1: API contract changes C1-C7
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
raps-dm/
└── src/lib.rs                    # Fix 1: Add pagination loop to 3 list functions

raps-derivative/
└── src/lib.rs                    # Fix 2-3: MdRegion enum, translate() params, region header, force default

raps-acc/
└── src/
    ├── lib.rs                    # Fix 4: New strip_project_prefix(), ensure_project_prefix()
    ├── admin.rs                  # Fix 4: Replace private normalize_project_id() with shared functions
    ├── permissions.rs            # Fix 4: Replace private normalize_project_id() with shared functions
    └── users.rs                  # Fix 4: Replace private normalize_project_id() with shared functions

raps-kernel/
└── src/auth.rs                   # Fix 5: TokenCache struct, Mutex-based refresh coordination

raps-reality/
└── src/lib.rs                    # Fix 6: mime_type_from_extension() function, replace hardcoded MIME

raps-cli/
└── src/commands/translate.rs     # Fix 2-3: --region and --force CLI flags
```

**Structure Decision**: Existing Rust workspace structure. No new crates or directories. All changes are modifications to existing files within their respective crates.

## Complexity Tracking

> No constitution violations. No complexity justification needed.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| (none) | — | — |

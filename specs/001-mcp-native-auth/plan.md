# Implementation Plan: MCP Server Native Authentication Support

**Branch**: `001-mcp-native-auth` | **Date**: 2026-01-17 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-mcp-native-auth/spec.md`

## Summary

Enhance the RAPS MCP server to provide native authentication support with clear guidance for credential setup, proactive 3-legged auth suggestions, and a dedicated `auth_login` tool that supports both browser-based OAuth and device code flow for headless environments. The implementation extends existing `auth_test` and `auth_status` tools with richer guidance and adds new tools for auth initiation.

## Technical Context

**Language/Version**: Rust 1.88 (edition 2024)
**Primary Dependencies**: rmcp 0.12, raps-kernel (AuthClient), tokio 1.49, serde_json
**Storage**: Existing token storage via raps-kernel (keyring + file fallback)
**Testing**: cargo test, raps-mock for API mocking
**Target Platform**: Cross-platform CLI (Linux, macOS, Windows)
**Project Type**: Rust workspace - changes in raps-cli crate (MCP server module)
**Performance Goals**: N/A (auth operations are user-interactive, not high-throughput)
**Constraints**: MCP stdio transport (single-user per instance), headless environment support
**Scale/Scope**: Single crate modification (raps-cli/src/mcp/), ~300-400 lines of new/modified code

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-Native & Modular | ✅ PASS | Changes contained in raps-cli crate, uses existing raps-kernel AuthClient |
| II. Automation-First | ✅ PASS | All MCP tools return structured responses; device code flow supports headless/CI |
| III. Secure by Default | ✅ PASS | Uses existing secure token storage; no secrets in responses; auth URLs only |
| IV. Comprehensive Observability | ✅ PASS | Auth status provides clear visibility; error messages are actionable |
| V. Quality & Reliability | ✅ PASS | Will include unit tests for new tools; follows existing test patterns |

**Gate Result**: PASS - No violations. Proceed to Phase 0.

## Project Structure

### Documentation (this feature)

```text
specs/001-mcp-native-auth/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (MCP tool schemas)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
raps-cli/
├── src/
│   └── mcp/
│       ├── mod.rs           # Module declaration
│       ├── server.rs        # Main MCP server (MODIFY)
│       ├── tools.rs         # Tool list constant (MODIFY)
│       └── auth_guidance.rs # NEW: Auth instruction content
└── tests/
    └── mcp_auth_tests.rs    # NEW: Tests for auth tools
```

**Structure Decision**: Single crate modification within existing workspace. New module `auth_guidance.rs` isolates the static instruction content from server logic.

## Complexity Tracking

> No violations - table not required.

---

## Post-Design Constitution Re-Check

*Re-evaluated after Phase 1 design completion.*

| Principle | Status | Design Validation |
|-----------|--------|-------------------|
| I. Rust-Native & Modular | ✅ PASS | New `auth_guidance.rs` module maintains separation; uses existing AuthClient API |
| II. Automation-First | ✅ PASS | Device code flow enables headless/CI; all responses are parseable text |
| III. Secure by Default | ✅ PASS | No credentials in responses; only auth URLs and device codes (designed for display) |
| IV. Comprehensive Observability | ✅ PASS | `auth_status` provides full visibility into auth state and tool availability |
| V. Quality & Reliability | ✅ PASS | Contract schemas defined; test plan in quickstart.md |

**Post-Design Gate Result**: PASS - Design aligns with all constitution principles.

---

## Generated Artifacts

| Artifact | Path | Status |
|----------|------|--------|
| Research | `specs/001-mcp-native-auth/research.md` | ✅ Complete |
| Data Model | `specs/001-mcp-native-auth/data-model.md` | ✅ Complete |
| Contracts | `specs/001-mcp-native-auth/contracts/mcp-auth-tools.json` | ✅ Complete |
| Quickstart | `specs/001-mcp-native-auth/quickstart.md` | ✅ Complete |
| Agent Context | `CLAUDE.md` | ✅ Updated |

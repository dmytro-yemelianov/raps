# Implementation Plan: Custom API Calls

**Branch**: `007-custom-api-calls` | **Date**: 2026-01-22 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/007-custom-api-calls/spec.md`

## Summary

Add a custom API call capability to RAPS that allows users to invoke any APS API endpoint directly using the currently authenticated session. This feature will be exposed via CLI subcommands (`raps api get`, `raps api post`, etc.) and an MCP tool (`api_request`), supporting all HTTP methods, query parameters, request bodies, custom headers, and multiple output formats. Requests are restricted to APS domains only for security.

## Technical Context

**Language/Version**: Rust 1.88, Edition 2024
**Primary Dependencies**: clap 4.5 (CLI), reqwest 0.11 (HTTP), tokio 1.49 (async), serde/serde_json (serialization), rmcp 0.12 (MCP server)
**Storage**: N/A (uses existing keyring-based token storage from raps-kernel)
**Testing**: cargo test, assert_cmd 2.0 (CLI integration), predicates 3.1, nextest
**Target Platform**: Cross-platform (Linux, macOS, Windows)
**Project Type**: Single (workspace crate addition to existing CLI)
**Performance Goals**: <5s for typical API responses (per SC-001); MCP within 10% of CLI overhead (per SC-006)
**Constraints**: APS domains only (developer.api.autodesk.com, api.userprofile.autodesk.com, and related Autodesk API hosts)
**Scale/Scope**: Extends existing CLI with 1 new command group (5 subcommands) and 1 new MCP tool

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The constitution file contains placeholder content (not project-specific rules). Applying standard Rust CLI best practices:

| Principle | Status | Notes |
|-----------|--------|-------|
| Follows existing patterns | ✅ Pass | Uses same clap/subcommand architecture as existing commands |
| Reuses existing components | ✅ Pass | Leverages raps-kernel auth, HTTP client, config; raps-cli output formatting |
| Test coverage | ✅ Pass | Will include unit tests for validation, integration tests for CLI |
| Error handling | ✅ Pass | Uses existing anyhow/thiserror patterns, exit codes |
| Security | ✅ Pass | Domain restriction prevents credential leakage to external URLs |

No constitution violations. Proceeding with Phase 0.

## Project Structure

### Documentation (this feature)

```text
specs/007-custom-api-calls/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (CLI interface contracts)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
raps-cli/
├── src/
│   ├── commands/
│   │   ├── mod.rs           # Add api module export
│   │   └── api.rs           # NEW: Custom API command implementation
│   ├── mcp/
│   │   ├── server.rs        # Add api_request tool dispatch
│   │   └── tools.rs         # Add api_request to TOOLS constant
│   └── main.rs              # Add Api variant to Commands enum
└── tests/
    └── api_tests.rs         # NEW: Integration tests for api command

raps-kernel/
└── src/
    └── http.rs              # Add domain validation helper (allowed_domains)
```

**Structure Decision**: Extends existing single-workspace CLI structure. New command module `api.rs` follows the established pattern of other commands (bucket.rs, object.rs, etc.). Domain validation logic placed in raps-kernel for reuse by both CLI and MCP.

## Complexity Tracking

No constitution violations requiring justification. Implementation follows existing patterns with minimal added complexity:

- Single new command module (not a new crate)
- Reuses existing HTTP client, auth, and output formatting
- Domain validation is a simple allowlist check

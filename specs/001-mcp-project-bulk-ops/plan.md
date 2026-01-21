# Implementation Plan: MCP Project Management and Bulk Operations

**Branch**: `001-mcp-project-bulk-ops` | **Date**: 2026-01-19 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-mcp-project-bulk-ops/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

Add 15 new MCP tools to the RAPS MCP server for complete object CRUD operations, project management, ACC project administration, and item management. Implementation extends the existing `raps-cli/src/mcp/server.rs` pattern with async methods, dispatch cases, and JSON schemas using rmcp 0.12.

## Technical Context

**Language/Version**: Rust 1.88, Edition 2024
**Primary Dependencies**: rmcp 0.12 (MCP server), raps-kernel (auth), raps-oss (object storage), raps-dm (data management), raps-acc (ACC modules), raps-admin (bulk user ops)
**Storage**: N/A (delegates to APS APIs via existing workspace crates)
**Testing**: cargo test with raps-mock for integration tests
**Target Platform**: Cross-platform CLI (Windows, macOS, Linux)
**Project Type**: Rust workspace - single crate extension (raps-cli)
**Performance Goals**: SC-002: 10-file batch upload < 30s, SC-003: single ops < 5s, SC-009: project create < 60s
**Constraints**: 4-way concurrency for batch uploads, files up to 5GB with chunking, OSS rate limits managed by exponential backoff
**Scale/Scope**: 15 new MCP tools, ~1500 LOC in server.rs additions, updates to auth_guidance.rs

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Phase 0 Check

| Principle | Compliance | Notes |
|-----------|------------|-------|
| **I. Rust-Native & Modular Workspace** | ✅ PASS | Tools added to raps-cli, delegates to existing crates (raps-oss, raps-dm, raps-acc, raps-admin). No new crates required. |
| **II. Automation-First Design** | ✅ PASS | MCP tools inherently support structured JSON output. No interactive prompts in server code. |
| **III. Secure by Default** | ✅ PASS | Auth delegated to raps-kernel. 3-legged tokens for project ops. No new credential handling. |
| **IV. Comprehensive Observability** | ✅ PASS | Inherits HTTP logging from existing clients. Batch ops return detailed summaries. |
| **V. Quality & Reliability** | ✅ PASS | Integration tests required for each new tool. All code passes fmt/clippy gates. |

**Gate Status**: ✅ All principles satisfied. No violations requiring justification.

### Post-Phase 1 Re-Check

| Principle | Compliance | Post-Design Notes |
|-----------|------------|-------------------|
| **I. Rust-Native & Modular Workspace** | ✅ PASS | Research confirmed: 6 new crate methods needed (raps-oss: 1, raps-dm: 2, raps-acc: 3). Existing workspace structure preserved. |
| **II. Automation-First Design** | ✅ PASS | Contracts define structured responses. Batch ops return machine-parseable summaries. |
| **III. Secure by Default** | ✅ PASS | Auth requirements documented per tool (6 tools = 2-leg, 9 tools = 3-leg). No secret handling changes. |
| **IV. Comprehensive Observability** | ✅ PASS | Batch results include per-item status. Error responses include suggested remediation. |
| **V. Quality & Reliability** | ✅ PASS | Test plan includes 15 tool tests + auth guidance tests. All crate changes require tests. |

**Post-Design Gate Status**: ✅ All principles satisfied after design review.

## Project Structure

### Documentation (this feature)

```text
specs/001-mcp-project-bulk-ops/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (MCP tool schemas)
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
raps-cli/
├── src/
│   ├── mcp/
│   │   ├── mod.rs              # Module exports
│   │   ├── server.rs           # MCP server + tool implementations (PRIMARY EDIT)
│   │   ├── tools.rs            # Tool constants list
│   │   └── auth_guidance.rs    # Auth requirement mappings (UPDATE)
│   └── ...
└── tests/
    └── mcp_auth_tests.rs       # MCP auth tests (UPDATE)

raps-oss/src/                   # OssClient - may need upload/download methods
raps-dm/src/                    # DataManagementClient - project/folder/item methods
raps-acc/src/                   # AccClient - may have project admin methods
raps-admin/src/                 # AccountAdminClient - project user management
```

**Structure Decision**: Single workspace extension. All 15 MCP tools implemented in `raps-cli/src/mcp/server.rs` following existing pattern. Underlying API methods delegated to workspace crates. No new crates required.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

*No violations - all principles satisfied.*

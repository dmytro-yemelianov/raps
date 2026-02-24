# Implementation Plan: Fix MEDIUM and LOW Severity Review Findings

**Branch**: `001-fix-medium-low-review` | **Date**: 2026-02-24 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/001-fix-medium-low-review/spec.md`

## Summary

Fix 11 review findings (6 MEDIUM, 5 LOW) across 7 workspace crates. Changes range from single-line fixes (BIM360 folder type, webhook validation, filesize helpers) to new endpoint implementations (7 Model Derivative metadata endpoints, OSS batch operations, DA app bundle upload) and concurrency improvements (parallel user imports, polling timeouts).

## Technical Context

**Language/Version**: Rust 1.88, Edition 2024
**Primary Dependencies**: clap 4.5, reqwest 0.11 (rustls-tls), tokio 1.49, serde, rmcp 0.12, keyring 2.3, indicatif
**Storage**: OS keychain (default), file-based fallback, in-memory caches
**Testing**: `cargo test`, `cargo nextest run`, `cargo clippy -- -D warnings`, `cargo fmt`
**Target Platform**: Windows, macOS, Linux (cross-platform CLI)
**Project Type**: Rust workspace with 10 crates (microkernel architecture)
**Performance Goals**: Parallel user imports must achieve >50% speedup over sequential for 20+ users
**Constraints**: Must not break existing public APIs; all 500+ existing tests must pass
**Scale/Scope**: 11 findings across 7 crates, ~8 files modified, ~4 new subcommands

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Rust-Native & Modular Workspace | PASS | All changes in existing `raps-*` crates with clear public APIs |
| II. Automation-First Design | PASS | New commands support `--output` formats, proper exit codes, non-interactive mode |
| III. Secure by Default | PASS | Auth patterns unchanged; webhook auth clarified as 2-legged; no new credential handling |
| IV. Comprehensive Observability | PASS | Polling timeout reports context; new endpoints use existing HTTP logging |
| V. Quality & Reliability | PASS | Unit tests for all new code; clippy clean; fmt clean |

No violations. Gate passes.

## Project Structure

### Documentation (this feature)

```text
specs/001-fix-medium-low-review/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 research
├── data-model.md        # Phase 1 entities
├── quickstart.md        # Phase 1 integration scenarios
├── contracts/           # Phase 1 API contracts
│   ├── model-derivative-metadata.md
│   ├── oss-batch.md
│   └── da-appbundle-upload.md
├── checklists/
│   └── requirements.md  # Spec quality checklist
└── tasks.md             # Phase 2 task breakdown
```

### Source Code (repository root)

```text
raps-derivative/src/
└── lib.rs               # Add metadata/properties/tree methods + response types

raps-oss/src/
└── lib.rs               # Add batch_copy_objects(), batch_rename_object() methods

raps-acc/src/
└── users.rs             # Add concurrent import with tokio::JoinSet + semaphore

raps-da/src/
└── lib.rs               # Add upload_appbundle() method using UploadParameters

raps-dm/src/
└── lib.rs               # BIM360 folder extension detection (already done in prior session)

raps-webhooks/src/
└── lib.rs               # Add is_valid_event(), doc comments (already done in prior session)

raps-reality/src/
└── lib.rs               # Add filesize_bytes() helpers (already done in prior session)

raps-kernel/src/
└── storage.rs           # Document sync I/O rationale (already done in prior session)

raps-cli/src/commands/
├── translate.rs         # Add metadata/tree/properties subcommands + polling timeout
├── oss.rs               # Add batch-copy/batch-rename subcommands
├── da.rs                # Add appbundle upload subcommand
├── reality.rs           # Polling timeout (already done in prior session) + filesize display
└── webhook.rs           # Event validation (already done in prior session)
```

**Structure Decision**: Existing workspace structure maintained. All changes are additions to existing crate `lib.rs` files and CLI command modules. No new crates needed.

## Prior Work (from previous session)

Several LOW-severity fixes were partially implemented before the speckit workflow was started. These changes exist on the `001-fix-api-alignment-bugs` branch and need to be cherry-picked or re-applied:

| Finding | Change | Status |
|---------|--------|--------|
| #10 Polling timeout (reality) | Added 4-hour timeout to reality.rs polling loop | Done |
| #13 Webhook auth clarity | Added doc comment to WebhooksClient | Done |
| #14 BIM360 folder extension | Auto-detect `b.` prefix in create_folder | Done |
| #16 Webhook event validation | Added `is_valid_event()` + CLI validation | Done |
| #17 Filesize helpers | Added `filesize_bytes()` to PhotosceneResult/UploadedFile | Done |
| #12 Sync I/O documentation | Added rationale comment to storage.rs save_file | Done |

These must be carried forward into this branch. Remaining work:

| Finding | Description | Effort |
|---------|-------------|--------|
| #10 Polling timeout (translate) | Add client-side timeout to translate.rs polling | Small |
| #8 MD metadata endpoints | 7 new endpoints in raps-derivative + 4 CLI subcommands | Large |
| #9 OSS batch operations | batch_copy + batch_rename in raps-oss + CLI subcommands | Medium |
| #11 Parallel user imports | Concurrent imports with semaphore in raps-acc | Medium |
| #15 DA appbundle upload | Upload method + CLI subcommand | Small |

## Design Decisions

### D1: Model Derivative Metadata — 4 Client Methods

Based on APS OpenAPI spec, implement 4 methods (covering the 7 endpoints referenced in the review):

1. `get_metadata(urn)` → List model views/viewables (GET .../metadata)
2. `get_object_tree(urn, model_guid)` → Object tree hierarchy (GET .../metadata/{guid})
3. `get_properties(urn, model_guid)` → All properties (GET .../metadata/{guid}/properties)
4. `query_properties(urn, model_guid, query)` → Filtered properties (POST .../metadata/{guid}/properties:query)

Response types: `ModelViews`, `ObjectTree`, `PropertiesResult`. Region-aware using existing `MdRegion` header support.

### D2: OSS Batch Operations — Composed from Singles

No dedicated batch API exists in APS. Implement batch operations by composing existing single-object methods:

- `batch_copy_objects(source_bucket, dest_bucket, object_keys)` — Iterates `copy_object()` per key
- `batch_rename_object(bucket_key, renames: Vec<(old, new)>)` — Copy-then-delete per rename

Use `tokio::JoinSet` with a semaphore (concurrency=10) for parallel execution. Return per-item results.

Note: The APS OSS API does support `POST /buckets/{bucket}/objects/{object}/copyto/{new_object}` for individual copies. This needs to be implemented as a base method first.

### D3: Parallel User Imports — Semaphore-Bounded Concurrency

Replace sequential `for user in users` loop with `tokio::JoinSet` bounded by a semaphore:

- Default concurrency: 10 (matches APS rate limit guidance)
- Each task calls existing `add_user()` independently
- Results collected via `JoinSet::join_next()`
- `ImportUsersResult` unchanged — same success/failure tracking

### D4: DA App Bundle Upload — Multipart Form POST

Use `UploadParameters.endpoint_url` and `form_data` returned by `create_appbundle()`:

1. Build multipart form from `form_data` HashMap
2. Attach file as final form part
3. POST to `endpoint_url` (S3 pre-signed URL)
4. Return success/failure

### D5: Translation Polling Timeout

Add 2-hour client-side timeout to translate.rs `check_status()` polling loop, matching the pattern already applied to reality.rs. The APS API also returns a "timeout" status server-side, but the client-side timeout provides a safety net.

## Complexity Tracking

No constitution violations to justify.

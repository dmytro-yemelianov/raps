# Research: Fix MEDIUM and LOW Severity Review Findings

**Date**: 2026-02-24
**Feature**: 001-fix-medium-low-review

## R1: Model Derivative Metadata Endpoints

**Decision**: Implement 4 client methods mapping to the APS Model Derivative v2 metadata endpoints.

**Rationale**: The APS OpenAPI spec (`aps-sdk-openapi/modelderivative/modelderivative.yaml`) defines these endpoints clearly. The RAPS website cookbooks already reference CLI commands (`raps derivative metadata`, `raps derivative tree`, `raps derivative properties`) that don't exist yet. These are read-only GET endpoints (plus one POST for filtered properties) that follow the same auth and URL patterns as the existing `get_manifest()` method.

**Alternatives considered**:
- Combine all into a single "get all metadata" method — rejected because object trees and properties can be very large; users need granular access.
- Expose raw JSON passthrough — rejected because structured types enable `--output table/csv` formatting.

**Endpoints**:
| Method | APS Endpoint | Auth |
|--------|-------------|------|
| `get_metadata(urn)` | `GET /designdata/{urn}/metadata` | 2-leg or 3-leg, `data:read` |
| `get_object_tree(urn, guid)` | `GET /designdata/{urn}/metadata/{guid}` | 2-leg or 3-leg, `data:read` |
| `get_properties(urn, guid)` | `GET /designdata/{urn}/metadata/{guid}/properties` | 2-leg or 3-leg, `data:read` |
| `query_properties(urn, guid, payload)` | `POST /designdata/{urn}/metadata/{guid}/properties:query` | 2-leg or 3-leg, `data:read` |

## R2: OSS Batch Operations Strategy

**Decision**: Compose batch operations from individual API calls with bounded concurrency.

**Rationale**: The APS OSS API has no bulk/batch endpoint. Individual copy endpoint exists: `POST /oss/v2/buckets/{bucket}/objects/{objectKey}/copyto/{newObjName}`. Batch rename requires copy-then-delete (no rename endpoint exists). Using `tokio::JoinSet` with `Semaphore(10)` provides parallelism without overwhelming the API.

**Alternatives considered**:
- Sequential execution — rejected because copying 100+ objects one-by-one is unacceptably slow.
- Unlimited parallelism — rejected because it would trigger APS rate limiting (429s).
- Client-side batching with a single HTTP call — not possible, APS doesn't support it.

## R3: Parallel User Import Concurrency Limit

**Decision**: Use semaphore-bounded concurrency of 10 concurrent requests.

**Rationale**: APS ACC Admin API has a rate limit of approximately 30 requests/second for project-level operations. With 10 concurrent requests and ~300ms round-trip time, we stay well under the limit. This provides ~10x speedup over sequential for large batches.

**Alternatives considered**:
- Higher concurrency (20-50) — rejected because risk of 429 errors increases; 10 is conservative and still delivers >50% speedup.
- Adaptive rate limiting — over-engineered for this use case; fixed semaphore is simpler.
- Rayon (CPU parallelism) — rejected because this is I/O-bound work; tokio tasks are appropriate.

## R4: DA App Bundle Upload Mechanism

**Decision**: Use the `UploadParameters` returned by `create_appbundle()` to POST a multipart form to S3.

**Rationale**: The APS Design Automation API returns pre-signed S3 upload parameters when creating/versioning an app bundle. The `endpoint_url` is the S3 URL, and `form_data` contains the required form fields (key, policy, signature, etc.). The archive file is appended as the last form part. This is the standard APS pattern.

**Alternatives considered**:
- Direct S3 SDK upload — rejected because the APS API provides the pre-signed URL; using S3 SDK directly bypasses their auth/tracking.
- Stream upload with reqwest — possible but unnecessary; app bundles are typically small (<100MB) and upload once.

## R5: Polling Timeout Values

**Decision**: Default timeouts based on typical operation durations:
- Translation: 2 hours (most translations complete in minutes; complex models may take 1+ hour)
- Photogrammetry: 4 hours (already applied; photoscene processing can legitimately take hours)
- Design Automation: 30 minutes (already exists in da.rs)

**Rationale**: Timeouts are client-side only and do not cancel server operations. Users can always check status manually after timeout. Values are generous to avoid false timeouts on legitimate long-running jobs.

**Alternatives considered**:
- User-configurable timeout via CLI flag — reasonable but adds complexity; can be added later if needed.
- Shorter timeouts (30min for all) — rejected because photogrammetry and large model translations regularly exceed 30 minutes.

## R6: Streaming Upload Deferral

**Decision**: Streaming photo upload (finding #7) is excluded from this feature per clarification session.

**Rationale**: Streaming requires refactoring the multipart upload pipeline and the retry closure pattern in `send_with_retry()`. The memory issue only manifests with very large photo sets. File-size display improvements provide immediate value with minimal risk.

**Alternatives considered**:
- Include streaming — rejected per clarification; deferred to future feature.

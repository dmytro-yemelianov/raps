# Research: Fix API Alignment Bugs

**Date**: 2026-02-24
**Branch**: 001-fix-api-alignment-bugs

## R1: DM REST Pagination Pattern

**Decision**: Implement page-following loop in `list_projects()`, `list_folder_contents()`, `get_item_versions()` using the existing `JsonApiResponse<T>` struct's `links.next` field.

**Rationale**: The `JsonApiResponse` and `JsonApiLinks` structs already exist and parse `links.next`. The three methods simply discard the links and return only the first page's `data` array. The fix is a pagination loop that follows `links.next` URLs until exhausted, accumulating results. The GraphQL methods in the same crate already implement this exact pattern (cursor-based loop with accumulation).

**Alternatives considered**:
- Add `page[number]` / `page[limit]` query params and increment manually → Rejected: less robust than following the server's `links.next` URLs, which already include correct params
- Return an async iterator/stream → Rejected: over-engineering for a bug fix; all callers expect `Vec<T>`

**Key findings**:
- `JsonApiLinks` struct (lib.rs:146-158) already has `next: Option<JsonApiLink>` field
- APS DM API uses `page[number]` (0-based) and `page[limit]` query params
- Response includes `links.next.href` as full URL when more pages exist
- Max page cap of 100 prevents infinite loops from malformed responses
- Empty pages with `links.next` present → continue (per clarification)

## R2: Model Derivative Region Support

**Decision**: Add `region` parameter to `translate()` method, read from Config or CLI flag, default to US. Use `x-ads-region` header (same as OSS crate pattern).

**Rationale**: The OpenAPI spec defines 8 regions (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR) passed via `region` header on the POST /job endpoint. The OSS crate already has a `Region` enum with `Display` impl and `x-ads-region` header usage — this pattern can be extended for Model Derivative with the full region set.

**Alternatives considered**:
- Shared Region enum in raps-kernel → Rejected for this PR: would require changes to raps-oss and potentially break its 2-variant enum. Better as follow-up refactor
- Region in URL path → Rejected: OpenAPI spec uses header, not URL

**Key findings**:
- Region hardcoded at lib.rs:293 (`region: "us"`) in request body AND must be sent as header
- x-ads-force hardcoded at lib.rs:315 (`.header("x-ads-force", "true")`)
- OSS Region enum (raps-oss/src/lib.rs:67-88) has US and EMEA only
- Config struct has no region field — region should be a per-command parameter, not global config
- translate() signature: `(urn, format, root_filename)` → needs `region` and `force` params
- CLI translate Start subcommand (translate.rs:22-38) needs `--region` and `--force` flags

## R3: Force-Translate Default Change

**Decision**: Change default to `force=false` (omit `x-ads-force` header when false). Add `--force` CLI flag. Include deprecation notice.

**Rationale**: The OpenAPI spec explicitly states `x-ads-force` defaults to false and is optional. The current hardcoded `true` is a bug that causes unnecessary manifest deletion. Per clarification, this is a breaking change handled with a deprecation notice in the changelog.

**Alternatives considered**:
- Keep `force=true` default with opt-out → Rejected: contradicts API spec default
- Config-level default → Rejected: per-command flag is simpler and sufficient

**Key findings**:
- OpenAPI spec (lines 64-72): `x-ads-force` is optional, default `false`
- When true: "system will delete the existing manifest and create a new one"
- When false/absent: uses existing manifest if available

## R4: Project ID Normalization

**Decision**: Create two clearly-named public functions in `raps-acc/src/lib.rs` (or a new `project_id` module): `strip_project_prefix()` and `ensure_project_prefix()`. Remove the three private conflicting `normalize_project_id()` functions.

**Rationale**: Admin API and Project Users API expect raw UUIDs (no "b." prefix). Data Management API expects "b." prefix. Having a single function named `normalize_project_id` that does opposite things in different files is the root cause. Two explicitly-named functions eliminate ambiguity.

**Alternatives considered**:
- Single function with enum parameter for target API → Rejected: more complex, enum adds indirection
- Move to raps-kernel → Rejected: project ID prefixing is ACC-specific, not kernel concern

**Key findings**:
- admin.rs:819-825: `normalize_project_id()` **strips** "b." prefix
- permissions.rs:323-330: `normalize_project_id()` **adds** "b." prefix
- users.rs:416-422: `normalize_project_id()` **strips** "b." prefix (same as admin)
- Admin API URL: `/construction/admin/v1/projects/{projectId}` → expects raw UUID
- DM API URL: `/data/v1/projects/{projectId}` → expects "b."-prefixed ID
- ACC account IDs also have "a." prefix with base64 encoding — handled separately

## R5: Token Refresh Race Condition

**Decision**: Replace `RwLock<Option<StoredToken>>` with `tokio::sync::Mutex` and add a refresh-in-progress flag to coordinate concurrent refresh attempts.

**Rationale**: The current RwLock pattern releases the read lock before calling `refresh_token()`, creating a TOCTOU window where multiple tasks can all see an expired token and independently trigger refresh HTTP requests. A Mutex with a flag ensures only one refresh occurs while others wait.

**Alternatives considered**:
- `tokio::sync::Notify` for cooperative signaling → Viable but more complex; flag-based approach is clearer
- Keep RwLock + add separate `AtomicBool` for refreshing state → Rejected: still has race between checking flag and setting it

**Key findings**:
- Current: `cached_3leg_token: Arc<RwLock<Option<StoredToken>>>` (auth.rs:89-90)
- `get_3leg_token()` (lines 179-201): reads cache, releases lock, then calls `refresh_token()`
- `refresh_token()` (lines 671-723): on failure, acquires write lock and clears cache
- Race: between read-lock release (line 193) and write-lock acquire (line 693), multiple tasks can initiate refresh
- Fix: `tokio::sync::Mutex<TokenCache>` where `TokenCache { token: Option<StoredToken>, refreshing: bool }`
- Waiters loop with small sleep (100ms) checking if refresh completed

## R6: MIME Type Detection

**Decision**: Add a simple `mime_type_from_extension()` function in `raps-reality/src/lib.rs` that maps file extensions to MIME types. No external crate needed.

**Rationale**: The Reality Capture API only accepts image files, so the mapping set is small (~10 extensions). The filename with extension is already available in the upload loop. A match statement on the lowercased extension is sufficient.

**Alternatives considered**:
- Add `mime_guess` crate → Rejected: heavyweight dependency for ~10 entries
- Let user specify MIME type per file → Rejected: poor UX for batch uploads

**Key findings**:
- Hardcoded at lib.rs:301: `.mime_str("image/jpeg")`
- Filename extracted at lib.rs:283-286 with extension preserved
- `file_parts: Vec<(String, Vec<u8>)>` stores (filename, bytes) — extension available from filename
- No `mime` or `mime_guess` crate in workspace dependencies
- Fallback per spec: `application/octet-stream` for unrecognized extensions
- Supported formats needed: jpg/jpeg, png, tiff/tif, bmp, webp, gif, raw

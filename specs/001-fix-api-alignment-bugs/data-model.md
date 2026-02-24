# Data Model: Fix API Alignment Bugs

**Feature**: 001-fix-api-alignment-bugs
**Date**: 2026-02-24
**Source**: [spec.md](spec.md) Key Entities + [research.md](research.md)

## Entities

### 1. JsonApiLinks (existing — extend usage)

Represents the pagination state from a JSON:API response.

| Field | Type | Description |
|-------|------|-------------|
| `self_link` | `Option<JsonApiLink>` | URL of the current page |
| `first` | `Option<JsonApiLink>` | URL of the first page |
| `next` | `Option<JsonApiLink>` | URL of the next page (None = last page) |
| `prev` | `Option<JsonApiLink>` | URL of the previous page |

**Validation Rules**:
- `next` being `Some(...)` means more pages exist, regardless of whether the current page's `data` array is empty
- A maximum page cap (100) prevents infinite loops from malformed `next` pointers

**State Transitions**:
- `next = Some(url)` → fetch next page, accumulate data
- `next = None` → pagination complete, return accumulated results

**Relationships**: Contained within `JsonApiResponse<T>`. Used by `list_projects()`, `list_folder_contents()`, `get_item_versions()`.

---

### 2. MdRegion (new)

Represents an APS data center region for Model Derivative API routing.

| Variant | Display Value | Description |
|---------|--------------|-------------|
| `US` | `"US"` | United States (default) |
| `EMEA` | `"EMEA"` | Europe, Middle East, Africa |
| `AUS` | `"AUS"` | Australia |
| `CAN` | `"CAN"` | Canada |
| `DEU` | `"DEU"` | Germany |
| `IND` | `"IND"` | India |
| `JPN` | `"JPN"` | Japan |
| `GBR` | `"GBR"` | Great Britain |

**Validation Rules**:
- Must be one of the 8 defined variants
- Case-insensitive parsing from CLI input (e.g., "emea" → `EMEA`)
- Invalid region rejected at parse time with error listing valid values
- Default: `US` when not specified

**Relationships**: Used by `translate()` as a parameter. Sent via `x-ads-region` header. Separate from `raps-oss::Region` (which has only US/EMEA).

---

### 3. ProjectId (conceptual — represented by two functions)

Not a dedicated struct. Represented by two normalization functions that convert between formats.

| Format | Pattern | Used By |
|--------|---------|---------|
| BIM 360 prefixed | `b.{uuid}` | Data Management API |
| BIM 360 raw | `{uuid}` | Admin API, Project Users API |
| ACC prefixed | `a.{base64}` | Data Management API |
| ACC raw | `{base64}` | Admin API |

**Functions**:
- `strip_project_prefix(id) -> String` — removes "b." or "a." prefix if present
- `ensure_project_prefix(id) -> String` — adds "b." prefix if not already present

**Validation Rules**:
- Unknown format (no "b." or "a." prefix, not a recognized UUID) → pass through unchanged, let API validate
- Both functions are idempotent: stripping an already-stripped ID is a no-op; ensuring an already-prefixed ID is a no-op

**Relationships**: Called by `admin.rs`, `permissions.rs`, `users.rs` in raps-acc. Replaces three private `normalize_project_id()` functions.

---

### 4. TokenCache (new)

Coordinates concurrent access to the 3-legged OAuth token, preventing race conditions during refresh.

| Field | Type | Description |
|-------|------|-------------|
| `token` | `Option<StoredToken>` | The cached 3-legged OAuth token |
| `refreshing` | `bool` | Whether a refresh is currently in progress |

**State Transitions**:
```
Idle (token=Some, refreshing=false)
  → Token expired detected
    → Acquire mutex lock
      → If refreshing=true: release lock, wait 100ms, re-check
      → If refreshing=false: set refreshing=true, release lock, perform refresh
        → On success: acquire lock, update token, set refreshing=false
        → On failure: acquire lock, set refreshing=false, propagate error
```

**Validation Rules**:
- Only one refresh at a time (enforced by `refreshing` flag under mutex)
- Failed refresh does NOT clear a valid cached token
- Newly refreshed token that is already expired → treated as refresh failure

**Relationships**: Replaces `Arc<RwLock<Option<StoredToken>>>` in `auth.rs`. Wrapped in `tokio::sync::Mutex`.

---

### 5. MimeMapping (conceptual — simple function)

Maps file extensions to MIME types for Reality Capture uploads.

| Extension | MIME Type |
|-----------|-----------|
| `jpg`, `jpeg` | `image/jpeg` |
| `png` | `image/png` |
| `tiff`, `tif` | `image/tiff` |
| `bmp` | `image/bmp` |
| `webp` | `image/webp` |
| `gif` | `image/gif` |
| `raw` | `application/octet-stream` |
| (unknown) | `application/octet-stream` |

**Validation Rules**:
- Extension extracted from filename, lowercased before matching
- Unrecognized extensions fall back to `application/octet-stream`
- If API rejects the detected type, the error surfaces the detected type for debugging

**Relationships**: Used by `upload_photos()` in raps-reality. Replaces hardcoded `"image/jpeg"`.

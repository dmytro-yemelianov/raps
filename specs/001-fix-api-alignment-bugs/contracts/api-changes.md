# API Contract Changes: Fix API Alignment Bugs

**Feature**: 001-fix-api-alignment-bugs
**Date**: 2026-02-24

These are internal Rust API changes within the RAPS workspace crates, not external HTTP API contracts. The external APS APIs are already defined by OpenAPI specs — these changes align our code with those specs.

## C1: raps-dm — Pagination (Internal Behavior Change)

**Affected functions**:
- `list_projects(hub_id) -> Vec<Project>`
- `list_folder_contents(project_id, folder_id) -> Vec<Item>`
- `get_item_versions(project_id, item_id) -> Vec<Version>`

**Before**: Returns only first page of results (discards `links.next`).
**After**: Follows all `links.next` URLs, accumulates and returns complete result set.

**Signature**: No change. Return type remains `Vec<T>`.
**Behavior**: Transparent to callers — same return type, more complete data.
**Safety cap**: Max 100 pages per call to prevent runaway loops.

---

## C2: raps-derivative — translate() Signature Change

**Before**:
```rust
pub async fn translate(&self, urn: &str, format: &str, root_filename: Option<&str>) -> Result<TranslateResponse>
```

**After**:
```rust
pub async fn translate(&self, urn: &str, format: &str, root_filename: Option<&str>, region: MdRegion, force: bool) -> Result<TranslateResponse>
```

**New parameters**:
- `region: MdRegion` — data center region (default: `US`)
- `force: bool` — whether to delete existing manifest (default: `false`)

**HTTP changes**:
- Adds `x-ads-region: {region}` header to POST /job
- Sets `x-ads-force: true` header only when `force=true`; omits header when `false`
- Body `region` field uses the `MdRegion` display value instead of hardcoded `"us"`

---

## C3: raps-derivative — MdRegion Enum (New Public Type)

```rust
pub enum MdRegion {
    US, EMEA, AUS, CAN, DEU, IND, JPN, GBR
}
```

**Traits**: `Display`, `FromStr`, `Clone`, `Copy`, `Debug`, `Default` (→ US)
**FromStr**: Case-insensitive. Returns error listing valid values on invalid input.

---

## C4: raps-acc — Project ID Functions (New Public API)

**Removed**: Three private `normalize_project_id()` functions in `admin.rs`, `permissions.rs`, `users.rs`.

**Added** (in `raps-acc/src/lib.rs` or `project_id.rs` module):
```rust
pub fn strip_project_prefix(id: &str) -> String
pub fn ensure_project_prefix(id: &str) -> String
```

**Callers updated**: `admin.rs`, `permissions.rs`, `users.rs` call the shared functions instead of their private variants.

---

## C5: raps-kernel — TokenCache (Internal Change)

**Before**: `cached_3leg_token: Arc<RwLock<Option<StoredToken>>>`
**After**: `cached_3leg_token: Arc<tokio::sync::Mutex<TokenCache>>`

Where:
```rust
struct TokenCache {
    token: Option<StoredToken>,
    refreshing: bool,
}
```

**Public API impact**: None. `get_3leg_token()` and `refresh_token()` retain their signatures. The change is internal synchronization only.

---

## C6: raps-reality — MIME Detection (Internal Change)

**Before**: Hardcoded `.mime_str("image/jpeg")` for all uploads.
**After**: Calls `mime_type_from_extension(filename)` to detect correct MIME type.

**New function** (private to raps-reality):
```rust
fn mime_type_from_extension(filename: &str) -> &'static str
```

**Public API impact**: None. `upload_photos()` signature unchanged.

---

## C7: raps-cli — translate CLI Changes

**Before**:
```
raps translate start <URN> --format <FORMAT> [--root-filename <FILE>]
```

**After**:
```
raps translate start <URN> --format <FORMAT> [--root-filename <FILE>] [--region <REGION>] [--force]
```

**New flags**:
- `--region <REGION>` — APS region (US, EMEA, AUS, CAN, DEU, IND, JPN, GBR). Default: US.
- `--force` — Delete existing manifest before translating. Default: off.

**Breaking change**: Previous behavior always set `force=true`. New default is `force=false`. Documented with deprecation notice in help text.

# Quickstart: Fix API Alignment Bugs

**Feature**: 001-fix-api-alignment-bugs
**Date**: 2026-02-24

## Overview

This feature fixes 6 bugs where RAPS code diverges from APS OpenAPI specifications. Changes span 5 workspace crates: raps-dm, raps-derivative, raps-acc, raps-kernel, raps-reality, plus CLI flag additions in raps-cli.

## Prerequisites

- Rust 1.88+ (Edition 2024)
- Existing RAPS workspace builds cleanly (`cargo check --workspace`)
- No new external crate dependencies required

## Implementation Order

The fixes are independent and can be implemented in any order. Recommended sequence by priority and risk:

1. **Pagination (raps-dm)** — P1, highest user impact, isolated change
2. **Region support (raps-derivative)** — P1, requires CLI changes too
3. **Force-translate default (raps-derivative)** — P1, pairs with region work
4. **Project ID normalization (raps-acc)** — P2, self-contained refactor
5. **Token refresh race (raps-kernel)** — P2, most complex, isolated to auth
6. **MIME detection (raps-reality)** — P3, simplest change

## Quick Verification

After implementing each fix:

```bash
# Type-check the affected crate
cargo check -p raps-dm          # Fix 1
cargo check -p raps-derivative  # Fix 2-3
cargo check -p raps-acc         # Fix 4
cargo check -p raps-kernel      # Fix 5
cargo check -p raps-reality     # Fix 6
cargo check -p raps-cli         # CLI changes

# Run all tests
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings
```

## Key Files Per Fix

| Fix | Primary File(s) | Test Location |
|-----|-----------------|---------------|
| Pagination | `raps-dm/src/lib.rs` | `raps-dm/src/lib.rs` (unit tests) |
| Region | `raps-derivative/src/lib.rs`, `raps-cli/src/commands/translate.rs` | `raps-derivative/src/lib.rs` |
| Force-translate | `raps-derivative/src/lib.rs`, `raps-cli/src/commands/translate.rs` | `raps-derivative/src/lib.rs` |
| Project ID | `raps-acc/src/lib.rs`, `raps-acc/src/admin.rs`, `raps-acc/src/permissions.rs`, `raps-acc/src/users.rs` | `raps-acc/src/lib.rs` |
| Token refresh | `raps-kernel/src/auth.rs` | `raps-kernel/src/auth.rs` |
| MIME type | `raps-reality/src/lib.rs` | `raps-reality/src/lib.rs` |

## Architecture Notes

- **No new crate dependencies** — all fixes use existing types and patterns
- **No public API signature changes** except `translate()` (gains `region` + `force` params) and new `strip_project_prefix()`/`ensure_project_prefix()` functions
- **Breaking change**: `--force` flag default changes from implicit true to explicit false. Documented with deprecation notice.
- **Token refresh** is the only change touching shared infrastructure (raps-kernel auth). Other fixes are isolated to their respective service crates.

# Account Context Display — Design

**Date:** 2026-03-04
**Status:** Approved

## Problem

Autodesk exposes two hub tiers (Personal and Enterprise) through the same API endpoint without making the distinction obvious. RAPS admin commands require Enterprise ACC/BIM360 accounts, but users are confused when commands silently fail or produce cryptic errors because they're working in a personal hub context. The current UX does nothing to surface this distinction proactively.

## Goal

Make the Personal vs Enterprise distinction explicit and visually unambiguous at every relevant surface — not just on failure, but as a normal part of every command that touches a hub or account.

## Visual Language

All visuals target **80-column minimum** terminal width. Personal and enterprise hubs use distinct ASCII glyphs + color — never color alone.

### Hub tier badges

```
Personal:
  ○ PERSONAL    My Projects              a.aBcD…xyz  [US]
  (dim gray, hollow circle — "not full access")

Enterprise:
  ◆ ENTERPRISE  Acme Corp                01fb…ae05   [US]
  (bold cyan, filled diamond — "full capability")

Unknown:
  ? UNKNOWN     Some Hub                 b.xxxx…     [US]
  (dim, question mark)
```

### Admin context box (printed before admin command output)

```
┌─ Account Context ────────────────────────────────────────────────────────────┐
│  ◆ ENTERPRISE  Acme Corp                                                     │
│  Account ID:   01fb1602-2ec0-4b05-bf6e-39dc70b3ae05                         │
│  Region:       US                                                            │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Warning box (when admin is attempted with no enterprise hub)

```
┌─ ⚠  Enterprise Account Required ────────────────────────────────────────────┐
│  Admin API is not available for personal hubs.                               │
│  Register your app in ACC Custom Integrations to enable admin commands.      │
│  Docs: rapscli.xyz/docs/custom-integrations                                  │
└──────────────────────────────────────────────────────────────────────────────┘
```
(yellow border + bold yellow `⚠`)

### `raps status` full dashboard (80 cols)

```
════════════════════════════════════════════════════════════════════════════════
  RAPS Status
════════════════════════════════════════════════════════════════════════════════

  Auth ─────────────────────────────────────────────────────────────────────
  2-legged    ✓ Available      (client credentials)
  3-legged    ✓ Logged in      expires in 27m
  Profile     main             client_id: RCM7…YJYS

  Hubs ─────────────────────────────────────────────────────────────────────
  ○ PERSONAL    My Projects              a.aBcD…xyz       [US]
  ◆ ENTERPRISE  Acme Corp                01fb…ae05        [US]
                └─ Admin API: ✓ ready
                   Account ID: 01fb1602-2ec0-4b05-bf6e-39dc70b3ae05

  Context ──────────────────────────────────────────────────────────────────
  account_id    01fb1602-2ec0-4b05-bf6e-39dc70b3ae05    env:APS_ACCOUNT_ID
  hub_id        (not set)
  project_id    (not set)
════════════════════════════════════════════════════════════════════════════════
```

## Architecture

### New module: `raps-cli/src/context_banner.rs`

```rust
pub enum HubTier { Personal, Enterprise, Unknown }

pub struct HubEntry {
    pub id: String,
    pub name: String,
    pub tier: HubTier,
    pub region: Option<String>,
    pub account_id: Option<String>,  // enterprise only
}

pub struct ContextBanner {
    pub hubs: Vec<HubEntry>,
}

impl ContextBanner {
    pub fn from_hubs(hubs: &[Hub]) -> Self
    pub fn from_account(id: &str, name: &str, region: Option<&str>) -> Self

    pub fn print_inline(&self)            // single line per hub → stderr
    pub fn print_box(&self, account: &HubEntry)   // bordered box → stderr
    pub fn print_warning_no_enterprise()  // yellow warning box → stderr
}
```

### Hub tier classification

| Extension type | Tier |
|---|---|
| `hubs:autodesk.core:Hub` | Personal |
| `hubs:autodesk.bim360:Account` | Enterprise |
| `hubs:autodesk.acc:Account` | Enterprise |
| `hubs:autodesk.accproject:*` | Enterprise |
| anything else | Unknown |

### Where each surface plugs in

| Surface | File | What prints |
|---|---|---|
| `raps hub list` | `commands/hub.rs` | inline line per hub (before table) |
| `raps admin *` | `commands/admin/mod.rs` | bordered box (after account resolved) |
| `resolve_account_id` → 0 enterprise | `commands/admin/mod.rs` | yellow warning box |
| `raps status` | `commands/status.rs` (new) | full dashboard |

## Components

### New files

- `raps-cli/src/context_banner.rs` — all rendering logic
- `raps-cli/src/commands/status.rs` — `StatusCommand`, full dashboard

### Modified files

- `raps-cli/src/commands/hub.rs` — call `ContextBanner::from_hubs().print_inline()` before table
- `raps-cli/src/commands/admin/mod.rs` — call `banner.print_box()` after resolving account; call `print_warning_no_enterprise()` when 0 enterprise hubs found
- `raps-cli/src/main.rs` — add `Status` variant to `Commands`, wire `dm_client` + `auth_client`
- `raps-cli/src/commands/mod.rs` — add `pub mod status;`

### No changes to

- API clients, domain crates, auth layer
- `raps-kernel`, `raps-dm`, `raps-acc`, `raps-admin`

## Output channel

All banner/box output goes to **stderr**. Structured output (`--output json/csv`) is unaffected — banners only appear in interactive (table) mode.

## Constraints

- All boxes fixed at 80-column width
- No additional network calls — banners built from data already fetched by the command
- `raps status` makes one `list_hubs()` call + auth check; no project enumeration

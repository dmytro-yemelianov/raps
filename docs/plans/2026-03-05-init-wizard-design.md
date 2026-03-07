# `raps init` — Setup Wizard Design

**Date:** 2026-03-05
**Status:** Approved

## Problem

New users face a multi-step setup process with no guidance: create an APS app, copy credentials, configure a profile, run `raps auth login`, discover hub types (Personal vs Enterprise), register the app in ACC Custom Integrations. Each step requires knowing the right command sequence and the right URLs. Without guidance, users hit cryptic errors at each stage.

## Goal

`raps init` is a first-time setup wizard that walks through every step in order, links to the right portal pages, validates each step before continuing, and ends with a full `raps status` summary showing a correctly configured environment.

## Visual Flow (80 cols)

```
════════════════════════════════════════════════════════════════════════════════
  RAPS Init — First-time setup
════════════════════════════════════════════════════════════════════════════════

  This wizard will configure your APS credentials, test authentication,
  log you in, and set up hub context.

  Steps:
    [1] APS Credentials
    [2] Test 2-Legged Auth
    [3] 3-Legged Login
    [4] Hub Discovery
    [5] Enterprise Context  (if enterprise hub found)
    [6] Summary

────────────────────────────────────────────────────────────────────────────────
  [1/6] APS Credentials
────────────────────────────────────────────────────────────────────────────────

  Create an APS app (if you haven't yet):
    → https://aps.autodesk.com/myapps

  Profile name [main]:
  Client ID:
  Client Secret:

  ✓ Profile 'main' saved

────────────────────────────────────────────────────────────────────────────────
  [2/6] Test 2-Legged Auth
────────────────────────────────────────────────────────────────────────────────
  Testing client credentials...
  ✓ 2-legged auth OK

────────────────────────────────────────────────────────────────────────────────
  [3/6] 3-Legged Login
────────────────────────────────────────────────────────────────────────────────
  Log in to access hubs and user context (optional — press Enter to skip).
  Proceed with browser login? [Y/n]:
  ✓ Logged in as Jane Dev (jane@acme.com)

────────────────────────────────────────────────────────────────────────────────
  [4/6] Hub Discovery
────────────────────────────────────────────────────────────────────────────────
  ○ PERSONAL    My Projects             a.aBcD…xyz   [US]
  ◆ ENTERPRISE  Acme Corp               01fb…ae05    [US]

────────────────────────────────────────────────────────────────────────────────
  [5/6] Enterprise Context
────────────────────────────────────────────────────────────────────────────────
  ┌─ Account Context ──────────────────────────────────────────────────────────┐
  │  ◆ ENTERPRISE  Acme Corp                                                   │
  │  Account ID:   01fb1602-2ec0-4b05-bf6e-39dc70b3ae05                       │
  │  Region:       US                                                          │
  └────────────────────────────────────────────────────────────────────────────┘

  To use admin commands, register this app in ACC Custom Integrations:
    → https://acc.autodesk.com  (Account Admin → Custom Integrations)
    → Docs: rapscli.xyz/docs/custom-integrations

  Save APS_ACCOUNT_ID = 01fb1602-2ec0-4b05-bf6e-39dc70b3ae05

  How would you like to save it?
    1) Save to profile 'main' only  (default)
    2) Save to profile + print export line for ~/.bashrc / ~/.zshrc
    3) Save to profile + auto-append to detected shell rc file

  Choice [1]:

────────────────────────────────────────────────────────────────────────────────
  [6/6] Summary
────────────────────────────────────────────────────────────────────────────────
  (full raps status dashboard)
════════════════════════════════════════════════════════════════════════════════
  Setup complete. Run `raps status` anytime to check your configuration.
════════════════════════════════════════════════════════════════════════════════
```

## Architecture

### New file: `raps-cli/src/commands/init.rs`

```rust
pub async fn run_init(
    auth_client: &AuthClient,
    dm_client: &DataManagementClient,
) -> Result<()>
```

Approach: Linear state machine — steps as private async fns, state carried as local
variables. No new structs or trait abstractions.

### Step functions

| Fn | Inputs | What it does |
|---|---|---|
| `step_credentials()` | — | Prompt profile name (default: `main`), client_id, client_secret. Create/update profile via `raps_kernel::config::save_profiles`. |
| `step_test_auth(auth_client)` | configured auth_client | Call `auth_client.test_auth()`. Print ✓/✗. Non-fatal on failure — user can continue. |
| `step_login(auth_client)` | auth_client | Confirm prompt → `login()` or `login_device()`. Skippable (user answers N). Returns `bool` (logged_in). |
| `step_discover_hubs(dm_client, logged_in)` | dm_client, bool | If not logged in: print skip message. Else call `dm_client.list_hubs()` → `ContextBanner::from_hubs().print_inline()`. Returns hub list. |
| `step_enterprise_context(hubs, profile_name)` | Vec<Hub>, profile name | Find enterprise hubs. If 0: `print_warning_no_enterprise()` + links. If ≥1: `ContextBanner::from_account().print_box()` + save-choice prompt (options 1/2/3). |
| `step_summary(auth_client, dm_client)` | both clients | Call `run_status(auth_client, dm_client, OutputFormat::Table)`. |

### Save-choice behaviour (step 5)

- **Option 1 (default):** `raps config set context_account_id <id>` path via profiles — write `context_account_id` field to active profile in `profiles.json`.
- **Option 2:** Same as 1 + print `export APS_ACCOUNT_ID=<id>` for user to copy.
- **Option 3:** Same as 1 + detect shell rc file (`$SHELL` → `.bashrc`/`.zshrc`/`.profile`) + append `export APS_ACCOUNT_ID=<id>` line.

### Skip logic

- Step 3 skipped: steps 4+5 show "(skipped — not logged in)" banners.
- Step 2 fails: warn and continue (step may be retried by re-running `raps init`).
- Step 5 skipped: if 0 enterprise hubs found, show warning box + links, continue to summary.

### Wiring

| File | Change |
|---|---|
| `raps-cli/src/commands/mod.rs` | Add `pub mod init;` |
| `raps-cli/src/main.rs` | Add `Init` variant to `Commands` enum; dispatch to `commands::init::run_init(&auth_client, &dm_client)` |

### Constraints

- No `--output` flag: wizard is always interactive table mode
- `raps_kernel::prompts` for all interactive input
- Reuses `ContextBanner`, `print_warning_no_enterprise()`, `run_status()` — no duplicated rendering logic
- All URLs displayed with `→ ` prefix for visual scannability

## Links used in wizard

| Step | URL |
|---|---|
| Step 1 (create app) | `https://aps.autodesk.com/myapps` |
| Step 5 (custom integrations) | `https://acc.autodesk.com` (Account Admin → Custom Integrations) |
| Step 5 (docs) | `rapscli.xyz/docs/custom-integrations` |

## No changes to

- `raps-kernel`, `raps-dm`, `raps-acc`, `raps-admin`
- Existing auth, config, or hub commands

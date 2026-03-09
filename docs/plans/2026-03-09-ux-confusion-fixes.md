# UX Confusion Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate all identified command confusion points so users never need to know which underlying API is being called.

**Architecture:** Fix in order of impact. Each fix is self-contained. `raps project list` auto-routes based on hub ID prefix (`b.` = BIM 360 Admin API, everything else = DM API). `raps folder rights` renamed to `permissions`. `raps admin user` bulk vs single split clarified with `--bulk` flag and cleaner naming.

**Tech Stack:** Rust 1.88, clap, raps-dm (DataManagementClient), raps-acc (AccountAdminClient), raps-acc::admin::AdminProjectListOutput

---

## Fix 1: `raps project list` — auto-detect hub type

### Context

- `raps project list` → `raps-cli/src/commands/project.rs` `list_projects()`
- Uses `DataManagementClient` (DM API), 3-legged auth
- BIM 360 hubs have IDs starting with `b.` — strip prefix to get account ID for Admin API
- Admin project list is in `raps-cli/src/commands/admin/project.rs` `AdminProjectCommands::List`
- `AccountAdminClient::list_all_projects(&account_id)` returns `Vec<AdminProject>`
- `AdminProject` has: `id`, `name`, `status`, `platform`, `created_at`
- `execute()` in `project.rs` currently takes only `&DataManagementClient` — needs config+auth too

### Task 1.1: Update `ProjectCommands::execute` signature

**Files:**
- Modify: `raps-cli/src/commands/project.rs`
- Modify: `raps-cli/src/main.rs` (call site)

**Step 1: Find the execute call site in main.rs**

```bash
grep -n "ProjectCommands\|project.*execute" raps-cli/src/main.rs
```

**Step 2: Update execute signature in project.rs**

Change:
```rust
pub async fn execute(
    self,
    client: &DataManagementClient,
    output_format: OutputFormat,
) -> Result<()>
```

To:
```rust
pub async fn execute(
    self,
    client: &DataManagementClient,
    config: raps_kernel::config::Config,
    auth_client: raps_kernel::auth::AuthClient,
    output_format: OutputFormat,
) -> Result<()>
```

Update the match arms to pass config/auth_client to `list_projects`.

**Step 3: Update the call site in main.rs**

Pass `config.clone()` and `auth_client.clone()` to the execute call. Match the pattern used by `AdminProjectCommands::execute` elsewhere in main.rs.

**Step 4: Build to confirm it compiles**

```bash
cargo build -p raps-cli 2>&1 | grep "^error"
```

**Step 5: Commit**

```bash
git add raps-cli/src/commands/project.rs raps-cli/src/main.rs
git commit -m "refactor: pass config+auth to ProjectCommands::execute for hub-type routing"
```

---

### Task 1.2: Add BIM 360 routing to `list_projects`

**Files:**
- Modify: `raps-cli/src/commands/project.rs`

**Step 1: Add imports at top of project.rs**

```rust
use raps_acc::admin::AccountAdminClient;
use raps_kernel::config::Config;
use raps_kernel::auth::AuthClient;
use raps_kernel::http::HttpClientConfig;
```

**Step 2: Add helper to detect BIM 360 hub**

```rust
/// Returns Some(account_id) if hub_id is a BIM 360 Business hub (prefix "b.")
fn bim360_account_id(hub_id: &str) -> Option<String> {
    hub_id.strip_prefix("b.").map(|s| s.to_string())
}
```

**Step 3: Add unified output type**

```rust
#[derive(Serialize, schemars::JsonSchema)]
pub struct ProjectListOutput {
    pub id: String,
    pub name: String,
    pub status: Option<String>,
    pub platform: Option<String>,
    pub project_type: Option<String>,
}
```

(Replace the existing `ProjectListOutput` struct — add `status`, `platform` as `Option`, make `project_type` optional.)

**Step 4: Update `list_projects` signature and body**

```rust
async fn list_projects(
    client: &DataManagementClient,
    config: Config,
    auth_client: AuthClient,
    hub_id: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    let hub = match hub_id {
        Some(h) => h,
        None => interactive::prompt_for_hub(client).await?,
    };

    // BIM 360 hub → use Admin API
    if let Some(account_id) = bim360_account_id(&hub) {
        return list_projects_admin(config, auth_client, &account_id, output_format).await;
    }

    // ACC / personal hub → use DM API (existing logic)
    // ... existing code ...
}
```

**Step 5: Add `list_projects_admin` function**

```rust
async fn list_projects_admin(
    config: Config,
    auth_client: AuthClient,
    account_id: &str,
    output_format: OutputFormat,
) -> Result<()> {
    let http_config = HttpClientConfig::default();
    let admin_client = AccountAdminClient::new_with_http_config(
        config,
        auth_client,
        http_config,
    )?;

    let projects = tracked_op("Fetching projects", output_format, || async {
        admin_client.list_all_projects(account_id).await
            .context("Failed to list BIM 360 projects")
    }).await?;

    let outputs: Vec<ProjectListOutput> = projects.iter().map(|p| ProjectListOutput {
        id: p.id.clone(),
        name: p.name.clone(),
        status: Some(p.status.clone()),
        platform: Some(p.platform.clone()),
        project_type: None,
    }).collect();

    // ... same table/json rendering as existing DM path ...
    output_format.write(&outputs)?;
    Ok(())
}
```

**Step 6: Build and test**

```bash
cargo build -p raps-cli 2>&1 | grep "^error"
# Test BIM 360 hub
RAPS_NO_COLOR=1 raps project list b.01fb1602-2ec0-4b05-bf6e-39dc70b3ae05
# Test interactive (no hub_id) — should prompt and work for any hub type
RAPS_NO_COLOR=1 raps project list
```

**Step 7: Commit**

```bash
git add raps-cli/src/commands/project.rs
git commit -m "feat: auto-route raps project list to Admin API for BIM 360 hubs"
```

---

## Fix 2: `raps folder rights` → `raps folder permissions`

### Context

- `raps folder rights` in `raps-cli/src/commands/folder.rs` — **shows** current user's permissions
- `raps admin folder rights` in `raps-cli/src/commands/admin/mod.rs` — **sets** another user's permissions
- The word "rights" is ambiguous; "permissions" (read) vs "set-permissions" (write) is clearer

### Task 2.1: Rename `raps folder rights` → `raps folder permissions`

**Files:**
- Modify: `raps-cli/src/commands/folder.rs`

**Step 1: Rename variant and command name**

In `FolderCommands` enum, change:
```rust
/// Show permissions (rights) for a folder
Rights { ... }
```
To:
```rust
/// Show your permissions for a folder
#[command(name = "permissions")]
Permissions { ... }
```

Keep `alias = "rights"` so existing scripts don't break:
```rust
#[command(name = "permissions", alias = "rights")]
Permissions { ... }
```

**Step 2: Update the match arm**

```rust
FolderCommands::Permissions { ... } => { ... }
```

**Step 3: Build**

```bash
cargo build -p raps-cli 2>&1 | grep "^error"
```

**Step 4: Commit**

```bash
git add raps-cli/src/commands/folder.rs
git commit -m "fix: rename 'raps folder rights' to 'raps folder permissions' (alias kept)"
```

---

### Task 2.2: Rename `raps admin folder rights` → `raps admin folder set-permissions`

**Files:**
- Modify: `raps-cli/src/commands/admin/mod.rs`

**Step 1: Find the FolderCommands variant in admin/mod.rs**

```bash
grep -n "Rights\|FolderCommands" raps-cli/src/commands/admin/mod.rs
```

**Step 2: Rename with alias**

Change:
```rust
/// Update folder permissions for a user
Rights { ... }
```
To:
```rust
/// Set folder permissions for a user across projects
#[command(name = "set-permissions", alias = "rights")]
SetPermissions { ... }
```

**Step 3: Update match arm in admin/folder.rs**

```bash
grep -n "Rights\|FolderCommands" raps-cli/src/commands/admin/folder.rs
```

Change `FolderCommands::Rights` to `FolderCommands::SetPermissions`.

**Step 4: Build**

```bash
cargo build -p raps-cli 2>&1 | grep "^error"
```

**Step 5: Commit**

```bash
git add raps-cli/src/commands/admin/mod.rs raps-cli/src/commands/admin/folder.rs
git commit -m "fix: rename 'raps admin folder rights' to 'set-permissions' (alias kept)"
```

---

## Fix 3: Clarify `raps admin user` bulk vs single

### Context

Current confusing pairs:
- `raps admin user add` (bulk, multiple projects) vs `raps admin user add-to-project` (single project)
- `raps admin user remove` (bulk) vs `raps admin user remove-from-project` (single)
- `raps admin user update` (bulk) vs `raps admin user update-in-project` (single)

All defined in `raps-cli/src/commands/admin/mod.rs` `UserCommands` enum.

### Task 3.1: Add `[bulk]` tag to help text for bulk commands

The minimal-churn fix: update the doc comments to make bulk vs single explicit. No renames needed (aliases would be required to not break existing users).

**Files:**
- Modify: `raps-cli/src/commands/admin/mod.rs`

**Step 1: Update doc comments on bulk commands**

```rust
/// [bulk] Add a user to multiple projects (use add-to-project for a single project)
Add { ... }

/// [bulk] Remove a user from multiple projects (use remove-from-project for a single project)
Remove { ... }

/// [bulk] Update user roles across multiple projects (use update-in-project for a single project)
Update { ... }
```

**Step 2: Update doc comments on single commands**

```rust
/// Add a user to a single project by email (use 'add' for multiple projects at once)
#[command(name = "add-to-project")]
AddToProject { ... }

/// Remove a user from a single project (use 'remove' for multiple projects at once)
#[command(name = "remove-from-project")]
RemoveFromProject { ... }

/// Update a user's role in a single project (use 'update' for multiple projects at once)
#[command(name = "update-in-project")]
UpdateInProject { ... }
```

**Step 3: Build**

```bash
cargo build -p raps-cli 2>&1 | grep "^error"
```

**Step 4: Verify help text**

```bash
RAPS_NO_COLOR=1 raps admin user --help
```

Confirm `[bulk]` appears in the listing for add/remove/update.

**Step 5: Commit**

```bash
git add raps-cli/src/commands/admin/mod.rs
git commit -m "fix: clarify bulk vs single in 'raps admin user' help text"
```

---

## Fix 4: Auth requirement visible in help

### Context

`raps admin` commands require the user to be an Account Administrator in ACC/BIM 360. This is not stated in the help text. Users get cryptic 403s instead.

### Task 4.1: Add account admin note to `raps admin --help`

**Files:**
- Modify: `raps-cli/src/commands/admin/mod.rs`

**Step 1: Find the AdminCommands enum doc comment**

```bash
grep -n "pub enum AdminCommands\|/// Account admin\|/// Admin" raps-cli/src/commands/admin/mod.rs | head -5
```

**Step 2: Update the top-level doc comment**

```rust
/// Account admin bulk management
///
/// Requires Account Administrator role in ACC/BIM 360.
/// Set APS_ACCOUNT_ID or use --account to specify your account.
```

**Step 3: Build and verify**

```bash
cargo build -p raps-cli 2>&1 | grep "^error"
RAPS_NO_COLOR=1 raps admin --help | head -10
```

**Step 4: Commit**

```bash
git add raps-cli/src/commands/admin/mod.rs
git commit -m "fix: document Account Administrator requirement in raps admin --help"
```

---

## Fix 5: Release

**Step 1: Cut release**

Use the `cutting-raps-release` skill to bump version and tag.

---

## Testing Checklist

After all fixes:

```bash
# Fix 1: BIM 360 hub routes to admin API
RAPS_NO_COLOR=1 raps project list b.01fb1602-2ec0-4b05-bf6e-39dc70b3ae05

# Fix 1: ACC hub still uses DM API
RAPS_NO_COLOR=1 raps project list a.YnVzaW5lc3M6Z21haWw2MDUzMTAz

# Fix 1: Interactive mode prompts hub selection
RAPS_NO_COLOR=1 raps project list

# Fix 2: New names work
RAPS_NO_COLOR=1 raps folder permissions --help
RAPS_NO_COLOR=1 raps admin folder set-permissions --help

# Fix 2: Old names still work (aliases)
RAPS_NO_COLOR=1 raps folder rights --help
RAPS_NO_COLOR=1 raps admin folder rights --help

# Fix 3: Bulk tag visible
RAPS_NO_COLOR=1 raps admin user --help | grep -E "\[bulk\]|add-to-project"

# Fix 4: Auth note visible
RAPS_NO_COLOR=1 raps admin --help | grep -i "administrator"
```

# Design: `admin user add-to-all-projects`

## Purpose

Add a user by email as Project Admin to all active projects in a hub/account. Single command, simple direct implementation.

## Command Signature

```
raps admin user add-to-all-projects <email> [--account <id>] [--concurrency <n>] [--dry-run]
```

- `email` — positional, the user's email to add
- `--account` — account/hub ID (defaults to `APS_ACCOUNT_ID` env var, normalized via `normalize_account_id`)
- `--concurrency` — parallel requests (defaults to global `--concurrency`, capped at 50)
- `--dry-run` — list projects that would be affected without making changes

Role is hardcoded to Project Admin (no `--role` flag).

## Flow

1. Resolve account ID via existing `get_account_id()` helper
2. Create `AccountAdminClient` + `ProjectUsersClient` (existing clients)
3. Call `admin_client.list_all_projects(&account_id)` to get all projects
4. Filter to `status == "active"` only
5. Concurrently call `users_client.add_user()` for each project using `futures::stream` + `buffer_unordered(concurrency)`
6. Track success/skip/fail counts, print progress inline
7. Print summary at end

## Output

- Table mode: per-project status line (project name, success/already exists/error), then summary
- JSON/YAML: structured output with project results array

## Code Changes

- `raps-cli/src/commands/admin/mod.rs` — add `AddToAllProjects` variant to `UserCommands` enum
- `raps-cli/src/commands/admin/user.rs` — add handler implementation

No API client crate changes needed — all required methods exist in `raps-acc`.

## API Calls Used

- `AccountAdminClient::list_all_projects(account_id)` — fetch all projects
- `ProjectUsersClient::add_user(project_id, AddProjectUserRequest)` — add user per project

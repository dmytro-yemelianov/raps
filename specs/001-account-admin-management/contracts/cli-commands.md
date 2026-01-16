# CLI Command Contracts: Account Admin Bulk Management

**Date**: 2026-01-16
**Feature**: 001-account-admin-management

## Overview

This document specifies the CLI commands for the account admin bulk management feature. Commands follow the existing RAPS CLI patterns.

---

## Command Hierarchy

```
raps admin
├── user
│   ├── add       # Bulk add user to projects
│   ├── remove    # Bulk remove user from projects
│   └── update    # Bulk update user roles
├── folder
│   └── rights    # Bulk update folder permissions
├── project
│   └── list      # List projects with filtering
└── operation
    ├── status    # Check operation status
    ├── resume    # Resume interrupted operation
    └── cancel    # Cancel in-progress operation
```

---

## User Commands

### `raps admin user add`

Add a user to multiple projects.

```
USAGE:
    raps admin user add [OPTIONS] <EMAIL>

ARGS:
    <EMAIL>    Email address of the user to add

OPTIONS:
    -a, --account <ID>         Account ID (defaults to current profile account)
    -r, --role <NAME>          Role to assign (e.g., "Project Admin", "Document Manager")
    -f, --filter <EXPR>        Project filter expression (see filtering section)
        --project-ids <FILE>   File containing project IDs (one per line)
        --concurrency <N>      Parallel requests (default: 10, max: 50)
        --dry-run              Preview changes without executing
    -o, --output <FORMAT>      Output format: table, json, yaml, csv
    -y, --yes                  Skip confirmation prompt
    -h, --help                 Print help

EXAMPLES:
    # Add user to all active projects
    raps admin user add newuser@example.com --role "Project Admin"

    # Add user to filtered projects
    raps admin user add newuser@example.com --filter "name:*Building*" --role "Document Manager"

    # Preview operation without executing
    raps admin user add newuser@example.com --dry-run

    # Use project list from file
    raps admin user add newuser@example.com --project-ids projects.txt

EXIT CODES:
    0    Success - all projects processed
    1    Partial success - some projects failed (see output)
    2    Failure - operation could not start (auth, validation)
    3    Cancelled - user cancelled operation
```

---

### `raps admin user remove`

Remove a user from multiple projects.

```
USAGE:
    raps admin user remove [OPTIONS] <EMAIL>

ARGS:
    <EMAIL>    Email address of the user to remove

OPTIONS:
    -a, --account <ID>         Account ID
    -f, --filter <EXPR>        Project filter expression
        --project-ids <FILE>   File containing project IDs
        --concurrency <N>      Parallel requests (default: 10)
        --dry-run              Preview changes without executing
    -o, --output <FORMAT>      Output format: table, json, yaml, csv
    -y, --yes                  Skip confirmation prompt
    -h, --help                 Print help

EXAMPLES:
    # Remove user from all projects
    raps admin user remove exuser@example.com

    # Remove from specific projects
    raps admin user remove exuser@example.com --filter "platform:ACC"

EXIT CODES:
    0    Success
    1    Partial success
    2    Failure
    3    Cancelled
```

---

### `raps admin user update`

Update user roles across multiple projects.

```
USAGE:
    raps admin user update [OPTIONS] <EMAIL> --role <NEW_ROLE>

ARGS:
    <EMAIL>    Email address of the user to update

OPTIONS:
    -a, --account <ID>         Account ID
    -r, --role <NAME>          New role to assign (required)
        --from-role <NAME>     Only update users with this current role
    -f, --filter <EXPR>        Project filter expression
        --project-ids <FILE>   File containing project IDs
        --concurrency <N>      Parallel requests (default: 10)
        --dry-run              Preview changes without executing
    -o, --output <FORMAT>      Output format
    -y, --yes                  Skip confirmation prompt
    -h, --help                 Print help

EXAMPLES:
    # Update user role across all projects
    raps admin user update user@example.com --role "Project Admin"

    # Change from one role to another
    raps admin user update user@example.com --from-role "Viewer" --role "Editor"

EXIT CODES:
    0    Success
    1    Partial success
    2    Failure
    3    Cancelled
```

---

## Folder Commands

### `raps admin folder rights`

Update folder permissions for a user across projects.

```
USAGE:
    raps admin folder rights [OPTIONS] <EMAIL> --level <PERMISSION>

ARGS:
    <EMAIL>    Email address of the user

OPTIONS:
    -a, --account <ID>         Account ID
    -l, --level <PERMISSION>   Permission level (required):
                               view-only, view-download, upload-only,
                               view-download-upload, view-download-upload-edit,
                               folder-control
        --folder <TYPE>        Folder type: project-files, plans, or custom path
    -f, --filter <EXPR>        Project filter expression
        --project-ids <FILE>   File containing project IDs
        --concurrency <N>      Parallel requests (default: 10)
        --dry-run              Preview changes without executing
    -o, --output <FORMAT>      Output format
    -y, --yes                  Skip confirmation prompt
    -h, --help                 Print help

EXAMPLES:
    # Grant view-download to Project Files folder
    raps admin folder rights user@example.com --level view-download --folder project-files

    # Grant full control to Plans folder
    raps admin folder rights user@example.com --level folder-control --folder plans

EXIT CODES:
    0    Success
    1    Partial success
    2    Failure
    3    Cancelled
```

---

## Project Commands

### `raps admin project list`

List projects with filtering (useful for preview).

```
USAGE:
    raps admin project list [OPTIONS]

OPTIONS:
    -a, --account <ID>         Account ID
    -f, --filter <EXPR>        Filter expression
        --status <STATUS>      Project status: active, inactive, archived
        --platform <PLATFORM>  Platform: acc, bim360, all (default: all)
        --limit <N>            Maximum projects to return
    -o, --output <FORMAT>      Output format: table, json, yaml, csv
    -h, --help                 Print help

EXAMPLES:
    # List all active projects
    raps admin project list --status active

    # List ACC projects matching name pattern
    raps admin project list --platform acc --filter "name:*Hospital*"

    # Export project list to CSV
    raps admin project list --output csv > projects.csv

EXIT CODES:
    0    Success
    2    Failure
```

---

## Operation Commands

### `raps admin operation status`

Check status of a bulk operation.

```
USAGE:
    raps admin operation status [OPERATION_ID]

ARGS:
    [OPERATION_ID]    Operation ID (defaults to most recent)

OPTIONS:
    -o, --output <FORMAT>      Output format
    -h, --help                 Print help

OUTPUT (table format):
    Operation: 550e8400-e29b-41d4-a716-446655440000
    Type:      add_user
    Status:    in_progress
    Progress:  1500/3000 (50%)
    Completed: 1495
    Skipped:   5
    Failed:    0
    Duration:  5m 32s
```

---

### `raps admin operation resume`

Resume an interrupted operation.

```
USAGE:
    raps admin operation resume [OPERATION_ID]

ARGS:
    [OPERATION_ID]    Operation ID to resume (defaults to most recent incomplete)

OPTIONS:
        --concurrency <N>      Override concurrency setting
    -o, --output <FORMAT>      Output format
    -h, --help                 Print help

EXAMPLES:
    # Resume most recent interrupted operation
    raps admin operation resume

    # Resume specific operation with higher concurrency
    raps admin operation resume 550e8400-... --concurrency 20
```

---

### `raps admin operation cancel`

Cancel an in-progress operation.

```
USAGE:
    raps admin operation cancel [OPERATION_ID]

ARGS:
    [OPERATION_ID]    Operation ID to cancel

OPTIONS:
    -y, --yes                  Skip confirmation prompt
    -h, --help                 Print help
```

---

## Filter Expression Syntax

Filter expressions use a simple key:value syntax.

```
SYNTAX:
    key:value[,key:value...]

KEYS:
    name        Project name (supports * wildcard)
    status      Project status (active, inactive, archived)
    platform    Platform type (acc, bim360)
    created     Date filter (>YYYY-MM-DD, <YYYY-MM-DD, YYYY-MM-DD..YYYY-MM-DD)
    region      Region (us, emea)

EXAMPLES:
    name:*Hospital*
    status:active,platform:acc
    created:>2024-01-01
    name:Building*,platform:bim360
```

---

## Output Formats

### Table (default)

Human-readable format with progress bars and colors.

```
Processing: [████████████████████░░░░░░░░░░] 67% (2000/3000)
  ✓ Completed: 1995
  ○ Skipped:   5
  ✗ Failed:    0
```

### JSON

Machine-readable format for scripting.

```json
{
  "operation_id": "550e8400-...",
  "status": "completed",
  "progress": {
    "total": 3000,
    "completed": 2995,
    "skipped": 5,
    "failed": 0
  },
  "results": [...]
}
```

### CSV

Tabular format for spreadsheet import.

```csv
project_id,project_name,status,message
b.proj-001,Hospital A,success,
b.proj-002,Hospital B,skipped,already_exists
```

### YAML

Alternative structured format.

```yaml
operation_id: 550e8400-...
status: completed
progress:
  total: 3000
  completed: 2995
```

---

## Global Options

These options apply to all commands:

```
GLOBAL OPTIONS:
    -p, --profile <NAME>       Use named profile from config
    -v, --verbose              Enable verbose output
        --debug                Enable debug output (includes HTTP logs)
    -h, --help                 Print help
    -V, --version              Print version
```

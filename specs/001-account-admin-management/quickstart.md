# Quickstart: Account Admin Bulk Management

**Date**: 2026-01-16
**Feature**: 001-account-admin-management

## Prerequisites

1. **RAPS CLI installed** (v3.12.0+ with admin feature)
2. **Autodesk account credentials** configured via `raps config`
3. **Account admin privileges** on your Autodesk account

## Setup

### 1. Authenticate with Account Admin Scope

```bash
# Interactive 3-legged OAuth (recommended)
raps auth login --scopes account:read account:write data:read data:write

# Verify authentication
raps auth status
```

### 2. Verify Account Access

```bash
# List accessible accounts/hubs
raps hub list

# Note your account ID (format: b.xxx-xxx)
```

---

## Common Operations

### Add a New User to All Active Projects

```bash
# Preview what will happen (dry run)
raps admin user add newuser@company.com --role "Document Manager" --dry-run

# Execute the operation
raps admin user add newuser@company.com --role "Document Manager"

# Output:
# Processing: [████████████████████████████] 100% (3000/3000)
#   ✓ Completed: 2995
#   ○ Skipped:   5 (already existed)
#   ✗ Failed:    0
# Operation complete in 12m 45s
```

### Add User to Filtered Projects

```bash
# Only projects matching name pattern
raps admin user add newuser@company.com --filter "name:*Hospital*" --role "Viewer"

# Only ACC projects (not BIM 360)
raps admin user add newuser@company.com --filter "platform:acc" --role "Project Admin"

# Projects created after a date
raps admin user add newuser@company.com --filter "created:>2024-01-01" --role "Editor"
```

### Update User Role Across Projects

```bash
# Change role for a user
raps admin user update user@company.com --role "Project Admin"

# Change from specific role only
raps admin user update user@company.com --from-role "Viewer" --role "Editor"
```

### Remove User from Projects

```bash
# Remove from all projects
raps admin user remove exemployee@company.com

# Remove from specific platform
raps admin user remove exemployee@company.com --filter "platform:bim360"
```

### Update Folder Permissions

```bash
# Grant view-download to Project Files
raps admin folder rights user@company.com --level view-download --folder project-files

# Grant full control to Plans folder
raps admin folder rights user@company.com --level folder-control --folder plans
```

---

## Working with Large Operations

### Using Project Lists

```bash
# First, export project list
raps admin project list --output csv > target-projects.csv

# Edit CSV to select specific projects
# Then use the ID column:
cut -d',' -f1 target-projects.csv | tail -n +2 > project-ids.txt

# Use project list file
raps admin user add newuser@company.com --project-ids project-ids.txt --role "Viewer"
```

### Adjusting Concurrency

```bash
# Increase for faster processing (if rate limits allow)
raps admin user add newuser@company.com --concurrency 20 --role "Viewer"

# Decrease if hitting rate limits
raps admin user add newuser@company.com --concurrency 5 --role "Viewer"
```

### Resuming Interrupted Operations

```bash
# Check for incomplete operations
raps admin operation status

# Resume most recent
raps admin operation resume

# Resume specific operation
raps admin operation resume 550e8400-e29b-41d4-a716-446655440000
```

---

## Output Formats

### Machine-Readable Output

```bash
# JSON output for scripting
raps admin user add user@company.com --role "Viewer" --output json > result.json

# YAML output
raps admin user add user@company.com --role "Viewer" --output yaml

# CSV output for spreadsheet
raps admin user add user@company.com --role "Viewer" --output csv > result.csv
```

### Example JSON Output

```json
{
  "operation_id": "550e8400-e29b-41d4-a716-446655440000",
  "operation_type": "add_user",
  "status": "completed",
  "progress": {
    "total": 3000,
    "completed": 2995,
    "skipped": 5,
    "failed": 0
  },
  "duration_seconds": 765,
  "results": [
    {"project_id": "b.proj-001", "project_name": "Hospital A", "status": "success"},
    {"project_id": "b.proj-002", "project_name": "Hospital B", "status": "skipped", "reason": "already_exists"}
  ]
}
```

---

## Non-Interactive Mode (CI/CD)

```bash
# Skip confirmation prompts
raps admin user add user@company.com --role "Viewer" --yes

# With JSON output for parsing
raps admin user add user@company.com --role "Viewer" --yes --output json

# Check exit code
echo $?
# 0 = success, 1 = partial success, 2 = failure, 3 = cancelled
```

---

## Troubleshooting

### Rate Limit Errors

```
Error: Rate limit exceeded, retry after 30 seconds
```

**Solution**: Reduce concurrency or wait for rate limit reset.

```bash
raps admin user add user@company.com --concurrency 5 --role "Viewer"
```

### User Not Found

```
Error: User not found in account: user@example.com
```

**Solution**: Verify email and ensure user is invited to the account.

```bash
# Search for user in account
raps admin user search user@example.com
```

### Permission Denied

```
Error: Insufficient permissions for project b.proj-001
```

**Solution**: Ensure you have Account Admin privileges. Some projects may have restricted access.

```bash
# Skip problematic projects by filtering
raps admin user add user@company.com --filter "status:active" --role "Viewer"
```

---

## Best Practices

1. **Always dry-run first** for large operations
2. **Use filters** to target specific projects instead of "all"
3. **Export results** to CSV for record-keeping
4. **Monitor progress** with verbose output for long operations
5. **Check operation status** if interrupted to resume

---

## Getting Help

```bash
# General help
raps admin --help

# Command-specific help
raps admin user add --help

# Verbose output for debugging
raps admin user add user@company.com --role "Viewer" --verbose

# Debug HTTP requests
raps admin user add user@company.com --role "Viewer" --debug
```

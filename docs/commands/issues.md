---
layout: default
title: Issues Commands
---

# Issues Commands

Manage ACC (Autodesk Construction Cloud) and BIM 360 issues, RFIs, and other project issues.

## Prerequisites

Issues commands require:
- 3-legged OAuth authentication (`raps auth login`)
- Access to an ACC or BIM 360 project

## Important: Project ID Format

**For Issues API, use project ID WITHOUT the "b." prefix:**

- ✅ **Correct**: `project123` (from Data Management API, remove "b." prefix)
- ❌ **Incorrect**: `b.project123` (this is the Data Management format)

To get the correct project ID:
1. Use `raps project list <hub-id>` to see projects
2. The project ID shown is `b.project123`
3. For Issues API, use `project123` (remove "b." prefix)

## Commands

### `raps issue list`

List issues in a project.

**Usage:**
```bash
raps issue list <project-id> [--status STATUS]
```

**Arguments:**
- `project-id`: Project ID (without "b." prefix)

**Options:**
- `--status, -s`: Filter by status (e.g., `open`, `closed`, `answered`)

**Example:**
```bash
$ raps issue list project123
Fetching issues...

Issues:
──────────────────────────────────────────────────────────────────────────────────────────
ID       Status       Title                                    Assigned To
──────────────────────────────────────────────────────────────────────────────────────────
#123     open         Missing window in Room 101               john.doe@example.com
#124     answered     HVAC system not working                   jane.smith@example.com
#125     closed       Door installation complete                 -
──────────────────────────────────────────────────────────────────────────────────────────
```

**Example with status filter:**
```bash
$ raps issue list project123 --status open
Fetching issues...

Issues:
──────────────────────────────────────────────────────────────────────────────────────────
ID       Status       Title                                    Assigned To
──────────────────────────────────────────────────────────────────────────────────────────
#123     open         Missing window in Room 101               john.doe@example.com
──────────────────────────────────────────────────────────────────────────────────────────
```

**Requirements:**
- 3-legged OAuth authentication
- Project access permissions

### `raps issue create`

Create a new issue.

**Usage:**
```bash
raps issue create <project-id> [--title TITLE] [--description DESCRIPTION]
```

**Arguments:**
- `project-id`: Project ID (without "b." prefix)

**Options:**
- `--title, -t`: Issue title
- `--description, -d`: Issue description

**Example:**
```bash
$ raps issue create project123 --title "Missing window" --description "Window missing in Room 101"
Creating issue...
✓ Issue created!
  ID: abc123xyz
  Title: Missing window
  Status: open
```

**Interactive Example:**
```bash
$ raps issue create project123
Enter issue title: Missing window
Enter description (optional): Window missing in Room 101
Creating issue...
✓ Issue created!
```

**Requirements:**
- 3-legged OAuth authentication
- Project write permissions

### `raps issue update`

Update an existing issue.

**Usage:**
```bash
raps issue update <project-id> <issue-id> [--status STATUS] [--title TITLE]
```

**Arguments:**
- `project-id`: Project ID (without "b." prefix)
- `issue-id`: Issue ID to update

**Options:**
- `--status, -s`: New status (`open`, `answered`, `closed`)
- `--title, -t`: New title

**Example:**
```bash
$ raps issue update project123 abc123xyz --status closed
Updating issue...
✓ Issue updated!
  Title: Missing window
  Status: open → closed
```

**Interactive Example:**
```bash
$ raps issue update project123 abc123xyz
Select new status:
  > open
    answered
    closed
Updating issue...
✓ Issue updated!
```

**Requirements:**
- 3-legged OAuth authentication
- Project write permissions

### `raps issue types`

List issue types (categories) and subtypes for a project.

**Usage:**
```bash
raps issue types <project-id>
```

**Arguments:**
- `project-id`: Project ID (without "b." prefix)

**Example:**
```bash
$ raps issue types project123
Fetching issue types...

Issue Types (Categories):
────────────────────────────────────────────────────────────
  • Quality Control
    ID: abc123xyz
    └ Safety
    └ Defect
    └ Non-Conformance

  • Request for Information
    ID: def456uvw
    └ General
    └ Design
    └ Construction

  • Safety
    ID: ghi789rst
────────────────────────────────────────────────────────────
```

**Requirements:**
- 3-legged OAuth authentication
- Project access permissions

## Issue Statuses

Common issue statuses:
- `open` - Issue is open and needs attention
- `answered` - Issue has been answered
- `closed` - Issue is resolved and closed

## Common Workflows

### Create and Track an Issue

```bash
# 1. List issue types to understand categories
raps issue types project123

# 2. Create an issue
raps issue create project123 --title "Missing window" --description "Room 101"

# 3. List issues to see it
raps issue list project123

# 4. Update status when resolved
raps issue update project123 abc123xyz --status closed
```

### Filter Issues by Status

```bash
# List only open issues
raps issue list project123 --status open

# List closed issues
raps issue list project123 --status closed
```

## API Version

The Issues commands use the **Construction Issues API v1**:
- Endpoint: `/construction/issues/v1`
- Different from Data Management API
- Requires project ID without "b." prefix

## Related Commands

- [Authentication]({{ '/commands/auth' | relative_url }}) - Set up 3-legged OAuth
- [Data Management]({{ '/commands/data-management' | relative_url }}) - Browse projects and folders
- [Hubs]({{ '/commands/data-management' | relative_url }}#hub-commands) - List hubs and projects


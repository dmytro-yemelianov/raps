---
layout: default
title: RFIs
---

# RFIs (Requests for Information)

Manage Requests for Information (RFIs) in Autodesk Construction Cloud (ACC) and BIM 360 projects.

## List RFIs

List all RFIs in a project, optionally filtering by status.

```bash
raps rfi list <project-id> [--status <status>]
```

**Options:**
- `--status`: Filter by status (`open`, `answered`, `closed`, `void`)

## Get RFI Details

Get detailed information about a specific RFI.

```bash
raps rfi get <project-id> <rfi-id>
```

## Create RFI

Create a new RFI.

```bash
raps rfi create <project-id> --title "Clarification on details" --question "Please clarify..." [options]
```

**Options:**
- `--title`: RFI title (required)
- `--question`: The question or description (required)
- `--priority`: Priority (`low`, `normal`, `high`, `critical`) (default: `normal`)
- `--due-date`: Due date (YYYY-MM-DD)
- `--assigned-to`: User ID to assign to
- `--location`: Location reference
- `--discipline`: Discipline

## Update RFI

Update an existing RFI, including answering or changing status.

```bash
raps rfi update <project-id> <rfi-id> [options]
```

**Options:**
- `--answer`: Set the answer text
- `--status`: Change status (`open`, `answered`, `closed`, `void`)
- `--assigned-to`: Reassign to user
- `--due-date`: Change due date

> [!NOTE]
> Project ID should **not** include the `b.` prefix used in Data Management API.

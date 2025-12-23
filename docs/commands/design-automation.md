---
layout: default
title: Design Automation Commands
---

# Design Automation Commands

Manage Design Automation engines, app bundles, activities, and work items for CAD processing automation.

## Prerequisites

Design Automation requires:
- `APS_DA_NICKNAME` environment variable set
- 2-legged OAuth authentication

## Commands

### `raps da engines`

List all available Design Automation engines.

**Usage:**
```bash
raps da engines
```

**Example:**
```bash
$ raps da engines
Fetching engines...

Available Engines:
────────────────────────────────────────────────────────────────────────────────

AutoCAD:
  • Autodesk.AutoCAD+24
  • Autodesk.AutoCAD+25

Revit:
  • Autodesk.Revit+2024
  • Autodesk.Revit+2025

Inventor:
  • Autodesk.Inventor+2024

3ds Max:
  • Autodesk.3dsMax+2024
────────────────────────────────────────────────────────────────────────────────
```

**Requirements:**
- 2-legged OAuth authentication
- `APS_DA_NICKNAME` environment variable

### `raps da appbundles`

List all app bundles.

**Usage:**
```bash
raps da appbundles
```

**Example:**
```bash
$ raps da appbundles
Fetching app bundles...

App Bundles:
────────────────────────────────────────────────────────────────────────────
  • MyAppBundle+1.0.0
  • MyAppBundle+1.0.1
  • AnotherBundle+2.0.0
────────────────────────────────────────────────────────────────────────────
```

**Requirements:**
- 2-legged OAuth authentication
- `APS_DA_NICKNAME` environment variable

### `raps da appbundle-create`

Create a new app bundle.

**Usage:**
```bash
raps da appbundle-create [--id ID] [--engine ENGINE] [--description DESCRIPTION]
```

**Options:**
- `--id, -i`: App bundle ID
- `--engine, -e`: Engine ID (e.g., `Autodesk.AutoCAD+24`)
- `--description, -d`: Description

**Example:**
```bash
$ raps da appbundle-create --id MyAppBundle --engine Autodesk.AutoCAD+24 --description "My custom app"
Creating app bundle...
✓ App bundle created!
  ID: MyAppBundle+1.0.0
  Engine: Autodesk.AutoCAD+24
  Version: 1.0.0

Upload your bundle ZIP to:
  https://developer.api.autodesk.com/oss/v2/buckets/wip.dm.prod/objects/...
```

**Interactive Example:**
```bash
$ raps da appbundle-create
Fetching engines...
Select engine:
  > Autodesk.AutoCAD+24
    Autodesk.AutoCAD+25
    Autodesk.Revit+2024
    ...
Enter app bundle ID: MyAppBundle
Creating app bundle...
✓ App bundle created!
```

**Requirements:**
- 2-legged OAuth authentication
- `APS_DA_NICKNAME` environment variable

### `raps da appbundle-delete`

Delete an app bundle.

**Usage:**
```bash
raps da appbundle-delete <id>
```

**Arguments:**
- `id`: App bundle ID to delete

**Example:**
```bash
$ raps da appbundle-delete MyAppBundle
Deleting app bundle...
✓ App bundle 'MyAppBundle' deleted!
```

**Requirements:**
- 2-legged OAuth authentication
- `APS_DA_NICKNAME` environment variable

### `raps da activities`

List all activities.

**Usage:**
```bash
raps da activities
```

**Example:**
```bash
$ raps da activities
Fetching activities...

Activities:
────────────────────────────────────────────────────────────────────────────
  • MyActivity+1.0.0
  • ProcessDrawing+2.0.0
────────────────────────────────────────────────────────────────────────────
```

**Requirements:**
- 2-legged OAuth authentication
- `APS_DA_NICKNAME` environment variable

### `raps da activity-delete`

Delete an activity.

**Usage:**
```bash
raps da activity-delete <id>
```

**Arguments:**
- `id`: Activity ID to delete

**Example:**
```bash
$ raps da activity-delete MyActivity
Deleting activity...
✓ Activity 'MyActivity' deleted!
```

**Requirements:**
- 2-legged OAuth authentication
- `APS_DA_NICKNAME` environment variable

### `raps da status`

Check the status of a work item.

**Usage:**
```bash
raps da status <workitem-id> [--wait]
```

**Arguments:**
- `workitem-id`: Work item ID to check

**Options:**
- `--wait, -w`: Wait for completion (polls every 5 seconds)

**Example:**
```bash
$ raps da status abc123xyz
✓ success
  Progress: 100%
  Report: https://developer.api.autodesk.com/...
```

**Example with --wait:**
```bash
$ raps da status abc123xyz --wait
⋯ Status: inprogress, Progress: 45%
⋯ Status: inprogress, Progress: 78%
✓ Work item completed successfully!
  Report: https://developer.api.autodesk.com/...
```

**Status Values:**
- `pending` - Work item queued
- `inprogress` - Work item processing
- `success` - Work item completed successfully
- `failed` - Work item failed
- `cancelled` - Work item cancelled
- `failedLimitDataSize` - Failed due to data size limit
- `failedLimitProcessingTime` - Failed due to processing time limit

**Requirements:**
- 2-legged OAuth authentication
- `APS_DA_NICKNAME` environment variable

## Design Automation Concepts

### Engines

Engines are the CAD applications available for automation:
- **AutoCAD** - 2D/3D CAD design
- **Revit** - BIM modeling
- **Inventor** - 3D mechanical design
- **3ds Max** - 3D modeling and animation

### App Bundles

App bundles contain your custom code/plugins that run on engines:
- Created with `raps da appbundle-create`
- Uploaded as ZIP files
- Versioned automatically (e.g., `MyApp+1.0.0`, `MyApp+1.0.1`)

### Activities

Activities define what your app bundle does:
- Reference an app bundle
- Define input/output parameters
- Specify which engine to use

### Work Items

Work items execute activities:
- Submit work items to process files
- Monitor status with `raps da status`
- Download results when complete

## Common Workflows

### Create and Deploy an App Bundle

```bash
# 1. List available engines
raps da engines

# 2. Create an app bundle
raps da appbundle-create --id MyApp --engine Autodesk.AutoCAD+24

# 3. Upload your bundle ZIP to the URL provided
# (Use curl, Postman, or another tool)

# 4. Verify app bundle was created
raps da appbundles
```

### Monitor Work Item Progress

```bash
# Check status once
raps da status abc123xyz

# Wait for completion
raps da status abc123xyz --wait
```

## Configuration

Set the Design Automation nickname:

```bash
# Windows PowerShell
$env:APS_DA_NICKNAME = "your_nickname"

# macOS/Linux
export APS_DA_NICKNAME="your_nickname"
```

Or add to `.env` file:
```env
APS_DA_NICKNAME=your_nickname
```

**Note:** The nickname must be unique across all APS applications and cannot be changed after first use.

## Related Commands

- [Authentication]({{ '/commands/auth' | relative_url }}) - Set up authentication
- [Buckets]({{ '/commands/buckets' | relative_url }}) - Manage OSS buckets for file storage
- [Objects]({{ '/commands/objects' | relative_url }}) - Upload/download files for processing


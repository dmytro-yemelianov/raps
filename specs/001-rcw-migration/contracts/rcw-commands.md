# CLI Command Contracts: RCW Migration

**Feature**: 001-rcw-migration
**Date**: 2026-01-23

## Command Group: `raps da rcw`

All RCW migration commands are grouped under the existing `da` (Design Automation) command.

---

## `raps da rcw configure`

Configure the RCW migration automation environment (AppBundle and Activity).

### Synopsis

```
raps da rcw configure [OPTIONS]
```

### Options

| Option | Short | Type | Required | Default | Description |
|--------|-------|------|----------|---------|-------------|
| `--engine` | `-e` | enum | No | 2026 | Revit engine version (2025, 2026) |
| `--alias` | `-a` | string | No | dev | Activity alias |
| `--force` | `-f` | flag | No | false | Recreate even if exists |

### Output (Table)

```
✓ RCW Migration environment configured!
  AppBundle: nickname.RCWMigratorAppBundle+dev (v1)
  Activity:  nickname.RCWMigratorActivity+dev
  Engine:    Autodesk.Revit+2026
```

### Output (JSON)

```json
{
  "success": true,
  "appbundle": {
    "id": "nickname.RCWMigratorAppBundle+dev",
    "version": 1
  },
  "activity": {
    "id": "nickname.RCWMigratorActivity+dev"
  },
  "engine": "Autodesk.Revit+2026"
}
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Authentication failure |
| 2 | AppBundle creation failed |
| 3 | Activity creation failed |

---

## `raps da rcw list`

List RCW models in a BIM 360/ACC folder.

### Synopsis

```
raps da rcw list <FOLDER_URL> [OPTIONS]
```

### Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `FOLDER_URL` | string | Yes | Folder URL (e.g., `projects/b.xxx/folders/urn:adsk...`) |

### Options

| Option | Short | Type | Required | Default | Description |
|--------|-------|------|----------|---------|-------------|
| `--recursive` | `-r` | flag | No | false | Include nested folders |
| `--limit` | `-l` | int | No | 100 | Max items to list |

### Output (Table)

```
RCW Models in "Project Files/Revit":
--------------------------------------------------------------------------------
  Name                          Size        Modified
--------------------------------------------------------------------------------
  Building-A.rvt               45.2 MB     2026-01-15 14:30
  Building-B.rvt               38.7 MB     2026-01-18 09:15
  MEP-Coordination.rvt         52.1 MB     2026-01-20 16:45
--------------------------------------------------------------------------------
Total: 3 RCW models
```

### Output (JSON)

```json
{
  "folder": "urn:adsk.wipprod:fs.folder:co.xxxxx",
  "models": [
    {
      "item_id": "urn:adsk.wipprod:dm.lineage:abc123",
      "name": "Building-A.rvt",
      "size": 47420416,
      "last_modified": "2026-01-15T14:30:00Z"
    }
  ],
  "total": 3
}
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Authentication failure |
| 2 | Folder not found |
| 3 | No access to folder |

---

## `raps da rcw migrate`

Migrate a single RCW model to ACC Docs.

### Synopsis

```
raps da rcw migrate <SOURCE_ITEM_URL> <DEST_FOLDER_URL> [OPTIONS]
```

### Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `SOURCE_ITEM_URL` | string | Yes | Source item URL (e.g., `projects/b.xxx/items/urn:adsk...`) |
| `DEST_FOLDER_URL` | string | Yes | Destination folder URL |

### Options

| Option | Short | Type | Required | Default | Description |
|--------|-------|------|----------|---------|-------------|
| `--engine` | `-e` | enum | No | 2026 | Revit engine version |
| `--name` | `-n` | string | No | original | Output filename |
| `--wait` | `-w` | flag | No | false | Wait for completion |

### Output (Table - Submitted)

```
✓ Migration job submitted!
  Workitem ID: abc123-def456
  Source:      Building-A.rvt
  Destination: Project Files/ACC/
  Status:      pending

Use 'raps da rcw status abc123-def456 --wait' to monitor progress.
```

### Output (Table - Completed with --wait)

```
✓ Migration completed!
  Workitem ID: abc123-def456
  Source:      Building-A.rvt
  Destination: Project Files/ACC/Building-A.rvt
  Duration:    2m 34s
```

### Output (JSON)

```json
{
  "workitem_id": "abc123-def456",
  "source": {
    "item_id": "urn:adsk.wipprod:dm.lineage:xxx",
    "name": "Building-A.rvt"
  },
  "destination_folder": "urn:adsk.wipprod:fs.folder:co.yyy",
  "status": "pending"
}
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (job submitted or completed) |
| 1 | Authentication failure |
| 2 | Source item not found |
| 3 | Not an RCW model |
| 4 | Destination folder not found |
| 5 | No write access to destination |
| 6 | Migration failed (with --wait) |

---

## `raps da rcw batch`

Migrate all RCW models from a source folder.

### Synopsis

```
raps da rcw batch <SOURCE_FOLDER_URL> <DEST_FOLDER_URL> [OPTIONS]
```

### Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `SOURCE_FOLDER_URL` | string | Yes | Source folder containing RCW models |
| `DEST_FOLDER_URL` | string | Yes | Destination folder in ACC |

### Options

| Option | Short | Type | Required | Default | Description |
|--------|-------|------|----------|---------|-------------|
| `--engine` | `-e` | enum | No | 2026 | Revit engine version |
| `--limit` | `-l` | int | No | 50 | Max models to migrate |
| `--wait` | `-w` | flag | No | false | Wait for all completions |
| `--dry-run` | | flag | No | false | List files without migrating |

### Output (Table)

```
✓ Batch migration started!
  Batch ID: batch-uuid-123
  Source:   Project Files/Revit/
  Dest:     Project Files/ACC/

  Jobs:
  -----------------------------------------------------------------------
  Workitem ID          File                    Status
  -----------------------------------------------------------------------
  abc123               Building-A.rvt          pending
  def456               Building-B.rvt          pending
  ghi789               MEP-Coordination.rvt    pending
  -----------------------------------------------------------------------
  Total: 3 migrations queued

Use 'raps da rcw status --batch batch-uuid-123 --wait' to monitor.
```

### Output (JSON)

```json
{
  "batch_id": "batch-uuid-123",
  "source_folder": "urn:adsk.wipprod:fs.folder:co.src",
  "destination_folder": "urn:adsk.wipprod:fs.folder:co.dst",
  "jobs": [
    {
      "workitem_id": "abc123",
      "source_name": "Building-A.rvt",
      "status": "pending"
    }
  ],
  "summary": {
    "total": 3,
    "pending": 3,
    "in_progress": 0,
    "completed": 0,
    "failed": 0
  }
}
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All jobs completed successfully |
| 1 | Authentication failure |
| 2 | Source folder not found |
| 3 | No RCW models found |
| 4 | Destination folder not found |
| 5 | Partial failure (some jobs failed) |

---

## `raps da rcw status`

Check migration job status.

### Synopsis

```
raps da rcw status <WORKITEM_ID> [OPTIONS]
```

### Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `WORKITEM_ID` | string | Yes | Workitem ID or batch ID |

### Options

| Option | Short | Type | Required | Default | Description |
|--------|-------|------|----------|---------|-------------|
| `--wait` | `-w` | flag | No | false | Wait for completion |
| `--batch` | `-b` | flag | No | false | Treat ID as batch ID |

### Output (Table - Single)

```
Migration Status: abc123-def456
--------------------------------------------------------------------------------
  Source:      Building-A.rvt
  Destination: Project Files/ACC/
  Status:      inprogress
  Progress:    45%
  Started:     2026-01-23 10:30:15
--------------------------------------------------------------------------------
```

### Output (Table - Completed)

```
✓ Migration Status: abc123-def456
--------------------------------------------------------------------------------
  Source:      Building-A.rvt
  Destination: Project Files/ACC/Building-A.rvt
  Status:      completed
  Duration:    2m 34s
  Report:      https://developer.api.autodesk.com/...
--------------------------------------------------------------------------------
```

### Output (JSON)

```json
{
  "workitem_id": "abc123-def456",
  "status": "inprogress",
  "progress": 45,
  "source_name": "Building-A.rvt",
  "destination_folder": "urn:adsk.wipprod:fs.folder:co.xxx",
  "created_at": "2026-01-23T10:30:15Z",
  "report_url": null
}
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (or completed if --wait) |
| 1 | Workitem not found |
| 2 | Job failed (with --wait) |

---

## `raps da rcw cancel`

Cancel a pending or in-progress migration.

### Synopsis

```
raps da rcw cancel <WORKITEM_ID> [OPTIONS]
```

### Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `WORKITEM_ID` | string | Yes | Workitem ID to cancel |

### Options

| Option | Short | Type | Required | Default | Description |
|--------|-------|------|----------|---------|-------------|
| `--batch` | `-b` | flag | No | false | Cancel all jobs in batch |

### Output (Table)

```
✓ Migration cancelled: abc123-def456
```

### Output (JSON)

```json
{
  "workitem_id": "abc123-def456",
  "status": "cancelled"
}
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Successfully cancelled |
| 1 | Workitem not found |
| 2 | Cannot cancel (already completed) |

---

## Common Behaviors

### URL Format

All folder/item URLs accept either:
- Full URL: `https://developer.api.autodesk.com/data/v1/projects/b.xxx/folders/urn:adsk...`
- Short format: `projects/b.xxx/folders/urn:adsk...`
- URN only: `urn:adsk.wipprod:fs.folder:co.xxxxx` (requires `--project` option)

### Output Formats

All commands support `--format` option inherited from global flags:
- `table` (default): Human-readable output
- `json`: Machine-readable JSON
- `yaml`: YAML output
- `csv`: CSV output (where applicable)

### Authentication

All commands require:
- 2-legged auth: Set via `APS_CLIENT_ID`, `APS_CLIENT_SECRET`
- 3-legged auth: Obtained via `raps auth login`

If 3-legged token is expired, commands will prompt for re-authentication.

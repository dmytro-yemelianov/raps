# Quickstart: RCW Migration with RAPS

**Feature**: 001-rcw-migration
**Date**: 2026-01-23

## Prerequisites

1. **RAPS CLI installed** (v4.4.0+)
2. **APS Application** with Design Automation enabled
3. **BIM 360/ACC Access** to source and destination projects

## Setup

### 1. Configure Authentication

Set your APS credentials:

```bash
# Set environment variables
export APS_CLIENT_ID="your-client-id"
export APS_CLIENT_SECRET="your-client-secret"

# Or use a profile
raps config set --client-id "your-client-id" --client-secret "your-client-secret"
```

Login for 3-legged access (required for project access):

```bash
raps auth login
```

### 2. Configure Migration Environment

One-time setup of the Design Automation AppBundle and Activity:

```bash
# Configure for Revit 2026 (default)
raps da rcw configure

# Or specify Revit 2025
raps da rcw configure --engine 2025
```

You should see:
```
✓ RCW Migration environment configured!
  AppBundle: your-nickname.RCWMigratorAppBundle+dev
  Activity:  your-nickname.RCWMigratorActivity+dev
  Engine:    Autodesk.Revit+2026
```

## Usage Examples

### List RCW Models

Find RCW models in a BIM 360 folder:

```bash
# List models in a specific folder
raps da rcw list "projects/b.abc123/folders/urn:adsk.wipprod:fs.folder:co.xyz"

# Include nested folders
raps da rcw list "projects/b.abc123/folders/urn:adsk.wipprod:fs.folder:co.xyz" --recursive

# Output as JSON for scripting
raps da rcw list "projects/b.abc123/folders/urn:adsk.wipprod:fs.folder:co.xyz" --format json
```

### Migrate a Single Model

Migrate one RCW model from BIM 360 to ACC:

```bash
# Submit migration (returns immediately)
raps da rcw migrate \
  "projects/b.abc123/items/urn:adsk.wipprod:dm.lineage:source" \
  "projects/b.xyz789/folders/urn:adsk.wipprod:fs.folder:co.dest"

# Submit and wait for completion
raps da rcw migrate \
  "projects/b.abc123/items/urn:adsk.wipprod:dm.lineage:source" \
  "projects/b.xyz789/folders/urn:adsk.wipprod:fs.folder:co.dest" \
  --wait
```

### Batch Migration

Migrate all RCW models from a folder:

```bash
# Dry run - see what would be migrated
raps da rcw batch \
  "projects/b.abc123/folders/urn:adsk.wipprod:fs.folder:co.source" \
  "projects/b.xyz789/folders/urn:adsk.wipprod:fs.folder:co.dest" \
  --dry-run

# Migrate up to 10 models
raps da rcw batch \
  "projects/b.abc123/folders/urn:adsk.wipprod:fs.folder:co.source" \
  "projects/b.xyz789/folders/urn:adsk.wipprod:fs.folder:co.dest" \
  --limit 10

# Migrate and wait for all to complete
raps da rcw batch \
  "projects/b.abc123/folders/urn:adsk.wipprod:fs.folder:co.source" \
  "projects/b.xyz789/folders/urn:adsk.wipprod:fs.folder:co.dest" \
  --wait
```

### Monitor Migration Status

```bash
# Check status of a single job
raps da rcw status abc123-def456

# Wait for completion
raps da rcw status abc123-def456 --wait

# Check batch status
raps da rcw status batch-uuid-123 --batch
```

### Cancel a Migration

```bash
# Cancel a pending/in-progress job
raps da rcw cancel abc123-def456

# Cancel all jobs in a batch
raps da rcw cancel batch-uuid-123 --batch
```

## Scripting Examples

### Migrate All Projects

```bash
#!/bin/bash
# migrate-all.sh - Migrate RCW models from multiple projects

PROJECTS=(
  "b.project1:urn:folder1:urn:dest1"
  "b.project2:urn:folder2:urn:dest2"
)

for entry in "${PROJECTS[@]}"; do
  IFS=':' read -r project source dest <<< "$entry"
  echo "Migrating from $project..."
  raps da rcw batch \
    "projects/$project/folders/$source" \
    "projects/$project/folders/$dest" \
    --format json >> migration-log.json
done
```

### Wait for All Migrations

```bash
#!/bin/bash
# wait-for-migrations.sh - Wait for all migrations to complete

WORKITEMS=$(raps da rcw batch "$SOURCE" "$DEST" --format json | jq -r '.jobs[].workitem_id')

for id in $WORKITEMS; do
  echo "Waiting for $id..."
  raps da rcw status "$id" --wait
done

echo "All migrations complete!"
```

## Troubleshooting

### "Not an RCW model" Error

Only Cloud Worksharing models (C4RModel type) can be migrated. Standard .rvt files uploaded to BIM 360 are not eligible.

**Solution**: Check that your model was published as a cloud worksharing model from Revit.

### "No access to folder" Error

The authenticated user needs:
- Read access to source project/folder
- Write access to destination project/folder

**Solution**: Verify permissions in BIM 360/ACC Admin Console.

### "Activity not found" Error

The migration environment hasn't been configured.

**Solution**: Run `raps da rcw configure` first.

### Migration Timeout

Very large models (>1GB) may exceed the Design Automation time limit.

**Solution**:
- Try migrating linked models separately
- Contact Autodesk support for increased limits

## Reference

- [Full CLI Command Reference](./contracts/rcw-commands.md)
- [Data Model](./data-model.md)
- [Research Notes](./research.md)

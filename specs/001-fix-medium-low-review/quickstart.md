# Quickstart: Fix MEDIUM and LOW Severity Review Findings

**Date**: 2026-02-24
**Feature**: 001-fix-medium-low-review

## Integration Scenarios

### Scenario 1: Model Derivative Metadata Workflow

```bash
# Translate a model
raps translate start --urn "$URN" --format svf2 --wait

# List available views/viewables
raps translate metadata "$URN" --output json

# Get object tree for a specific view
raps translate tree "$URN" "$GUID" --output json

# Get all properties for a view
raps translate properties "$URN" "$GUID" --output table

# Query specific properties by object ID
raps translate query-properties "$URN" "$GUID" --filter "1,2,3" --output json

# EMEA region support
raps translate metadata "$URN" --region emea --output json
```

### Scenario 2: OSS Batch Operations

```bash
# Copy all objects from one bucket to another
raps object batch-copy my-source-bucket my-dest-bucket

# Copy only objects matching a prefix
raps object batch-copy my-source-bucket my-dest-bucket --prefix "models/"

# Copy specific objects
raps object batch-copy my-source-bucket my-dest-bucket --keys "file1.rvt,file2.rvt"

# Batch rename with pattern replacement
raps object batch-rename my-bucket --from "old-prefix/" --to "new-prefix/"
```

### Scenario 3: Parallel User Import

```bash
# Import users from CSV (now uses concurrent requests with semaphore=10)
raps admin user import --project "b.project-123" --from-csv users.csv

# Expected output shows concurrent progress:
# ⠋ Importing 50 users concurrently...
# ✓ 47 imported, ✗ 3 failed
# Failed: user@example.com (already exists), ...
```

### Scenario 4: DA App Bundle Upload

```bash
# Create app bundle and upload archive (creates new version + uploads)
raps da appbundle-upload MyBundle --file ./my-bundle.zip --engine "Autodesk.Revit+2024"

# Expected output:
# → Creating new version of app bundle 'MyBundle'...
# ✓ Version 2 created. Uploading archive...
# ✓ Archive './my-bundle.zip' uploaded to app bundle 'MyBundle' (version 2)
```

### Scenario 5: Polling with Timeout Safety

```bash
# Translation with wait — now has 2-hour timeout
raps translate start --urn "$URN" --format svf2 --wait
# If timeout: "⏱ Timed out after 2 hours. Check status: raps translate status <URN>"

# Reality capture with wait — has 4-hour timeout
raps reality process <PHOTOSCENE_ID> --wait
# If timeout: "⏱ Timed out after 4 hours. Use 'raps reality status <ID>' to check later."
```

### Scenario 6: Webhook Event Validation

```bash
# Valid event — proceeds normally
raps webhook create --event dm.version.added --url https://example.com/hook

# Invalid event — rejected immediately with helpful message
raps webhook create --event invalid.event --url https://example.com/hook
# Error: Unknown webhook event 'invalid.event'. Valid events: dm.version.added, dm.version.modified, ...
```

### Scenario 7: Human-Readable File Sizes

```bash
# View photoscene result — file size now human-readable
raps reality result <PHOTOSCENE_ID>
# Photoscene Result:
#   ID: scene-123
#   Progress: 100%
#   File Size: 52.43 MB    # was: "54935241"
```

## Verification Commands

```bash
# Build check
cargo check --workspace

# All tests pass
cargo test --workspace

# Linting clean
cargo clippy --workspace -- -D warnings

# Formatting clean
cargo fmt -- --check

# Specific crate tests
cargo test -p raps-derivative
cargo test -p raps-oss
cargo test -p raps-acc
cargo test -p raps-da
cargo test -p raps-webhooks
cargo test -p raps-reality
cargo test -p raps-cli
```

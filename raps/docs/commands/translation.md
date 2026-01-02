---
layout: default
title: Translation Commands
---

# Translation Commands

Translate CAD files using the Model Derivative API to various formats for viewing and processing.

## Commands

### `raps translate start`

Start a translation job for a file.

**Usage:**
```bash
raps translate start [urn] [--format FORMAT] [--root-filename FILENAME]
```

**Arguments:**
- `urn`: Base64-encoded URN of the source file (optional, will prompt if not provided)

**Options:**
- `--format, -f`: Output format (svf2, svf, obj, stl, step, iges, ifc)
- `--root-filename, -r`: Root filename (for ZIP files with multiple design files)

**Example:**
```bash
$ raps translate start dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6bXktYnVja2V0L21vZGVsLmR3Zw== --format svf2
Starting translation...
✓ Translation job started!
  URN: dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6bXktYnVja2V0L21vZGVsLmR3Zw==
  Format: svf2
  Job ID: abc123xyz
```

**Supported Formats:**
- `svf2` - SVF2 format for Forge Viewer (recommended)
- `svf` - Legacy SVF format
- `obj` - Wavefront OBJ format
- `stl` - STL format for 3D printing
- `step` - STEP format
- `iges` - IGES format
- `ifc` - IFC format for BIM

**Getting a URN:**
After uploading a file with `raps object upload`, the URN is displayed. You can also get it from the object ID:
```
urn:adsk.objects:os.object:bucket-key/object-key
```

**Requirements:**
- 2-legged OAuth authentication
- File must be uploaded to OSS first

### `raps translate status`

Check the status of a translation job.

**Usage:**
```bash
raps translate status <urn> [--wait]
```

**Arguments:**
- `urn`: Base64-encoded URN of the source file

**Options:**
- `--wait, -w`: Wait for translation to complete (polls every 5 seconds)

**Example:**
```bash
$ raps translate status dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6bXktYnVja2V0L21vZGVsLmR3Zw==
Status: success
Progress: 100%
```

**Example with --wait:**
```bash
$ raps translate status dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6bXktYnVja2V0L21vZGVsLmR3Zw== --wait
⋯ Checking status...
⋯ Status: inprogress, Progress: 45%
⋯ Status: inprogress, Progress: 78%
✓ Status: success, Progress: 100%
```

**Status Values:**
- `pending` - Translation queued
- `inprogress` - Translation in progress
- `success` - Translation completed successfully
- `failed` - Translation failed
- `timeout` - Translation timed out

**Requirements:**
- 2-legged OAuth authentication

### `raps translate manifest`

View the translation manifest (available derivatives).

**Usage:**
```bash
raps translate manifest <urn>
```

**Arguments:**
- `urn`: Base64-encoded URN of the source file

**Example:**
```bash
$ raps translate manifest dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6bXktYnVja2V0L21vZGVsLmR3Zw==
Fetching manifest...

Manifest:
────────────────────────────────────────────────────────────
  URN: dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6bXktYnVja2V0L21vZGVsLmR3Zw==
  Status: success
  Progress: 100%

  Derivatives:
    • SVF2 (viewable)
    • Geometry (obj)
    • Metadata (json)
────────────────────────────────────────────────────────────
```

**Requirements:**
- 2-legged OAuth authentication
- Translation must be completed (status: success)

### `raps translate download`

Download translated derivative outputs.

**Usage:**
```bash
raps translate download <urn> [--format FORMAT] [--guid GUID] [--output-dir DIR]
```

**Arguments:**
- `urn`: Base64-encoded URN of the source file

**Options:**
- `--format, -f`: Derivative format to download (svf2, obj, stl, step, iges)
- `--guid, -g`: Specific derivative GUID (from manifest)
- `--output-dir, -o`: Output directory (default: current directory)

**Example:**
```bash
$ raps translate download <urn> --format obj --output-dir ./exports
Fetching manifest...
Downloading OBJ derivative...
  ████████████████████████████ 100%
✓ Download complete!
  Saved to: ./exports/model.obj (2.3 MB)
```

**Download with GUID:**
```bash
$ raps translate download <urn> --guid "abc123-def456"
Downloading derivative...
✓ Download complete!
```

**Requirements:**
- 2-legged OAuth authentication
- Translation must be completed

### `raps translate preset`

Manage translation presets for common translation configurations.

**Usage:**
```bash
raps translate preset <subcommand>
```

**Subcommands:**
- `list` - List all available presets
- `create` - Create a new preset
- `delete` - Delete a preset
- `use` - Start a translation using a preset

**List Presets:**
```bash
$ raps translate preset list
Translation Presets:
────────────────────────────────────────────────────────────
Name              Format    Description
────────────────────────────────────────────────────────────
viewer            svf2      Optimized for Forge Viewer
3d-print          stl       Ready for 3D printing
cad-exchange      step      CAD data exchange format
bim               ifc       BIM interoperability format
────────────────────────────────────────────────────────────
```

**Create a Preset:**
```bash
$ raps translate preset create --name "my-preset" --format obj --description "Custom OBJ export"
✓ Preset 'my-preset' created!
```

**Use a Preset:**
```bash
$ raps translate preset use <urn> --preset viewer
→ Using preset: viewer (svf2)
✓ Translation started with preset 'viewer'!
  Format: svf2
  URN: dXJuOmFkc2sub2...
```

**Delete a Preset:**
```bash
$ raps translate preset delete my-preset
✓ Preset 'my-preset' deleted!
```

**Default Presets:**
- `viewer` - SVF2 format optimized for Forge Viewer
- `3d-print` - STL format for 3D printing
- `cad-exchange` - STEP format for CAD data exchange
- `bim` - IFC format for BIM interoperability

## Common Workflows

### Complete Translation Workflow

```bash
# 1. Upload a file
raps object upload my-bucket model.dwg
# Note the URN from the output

# 2. Start translation
raps translate start <urn> --format svf2

# 3. Wait for completion
raps translate status <urn> --wait

# 4. View manifest
raps translate manifest <urn>
```

### Translate Multiple Formats

```bash
# Translate to SVF2 for viewing
raps translate start <urn> --format svf2

# Translate to OBJ for export
raps translate start <urn> --format obj

# Translate to STL for 3D printing
raps translate start <urn> --format stl
```

### Translate ZIP Files

For ZIP files containing multiple design files, specify the root filename:

```bash
raps translate start <urn> --format svf2 --root-filename model.dwg
```

## Supported File Types

### Input Formats
- **CAD Files**: DWG, DXF, DWF, DWFX
- **3D Models**: FBX, OBJ, STL, STEP, IGES
- **BIM Files**: RVT, NWD, NWC, IFC
- **Archives**: ZIP (with design files inside)

### Output Formats
- **Viewing**: SVF2, SVF
- **Export**: OBJ, STL, STEP, IGES, IFC

## Translation Tips

1. **Use SVF2 format** for web viewing (recommended)
2. **Check status regularly** or use `--wait` flag
3. **Large files take longer** - be patient
4. **ZIP files** may require specifying root filename
5. **Failed translations** - check file format compatibility

## Troubleshooting

### Translation Failed

1. Verify file format is supported
2. Check file isn't corrupted
3. Ensure file is fully uploaded to OSS
4. Try a different output format

### Translation Stuck

1. Check status: `raps translate status <urn>`
2. Wait longer (large files can take 10+ minutes)
3. Cancel and restart if necessary

## Related Commands

- [Objects](objects.md) - Upload files to OSS
- [Buckets](buckets.md) - Manage buckets
- [Authentication](auth.md) - Set up authentication


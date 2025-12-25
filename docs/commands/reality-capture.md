---
layout: default
title: Reality Capture Commands
---

# Reality Capture Commands

Create photoscenes, upload photos, and process photogrammetry projects using the Reality Capture API.

## Commands

### `raps reality create`

Create a new photoscene for photogrammetry processing.

**Usage:**
```bash
raps reality create [--name NAME] [--scene-type TYPE] [--format FORMAT]
```

**Options:**
- `--name, -n`: Photoscene name
- `--scene-type, -t`: Scene type (`aerial` or `object`)
- `--format, -f`: Output format (`rcm`, `rcs`, `obj`, `fbx`, `ortho`)

**Example:**
```bash
$ raps reality create --name "Building Scan" --scene-type object --format obj
Creating photoscene...
✓ Photoscene created!
  ID: abc123xyz
  Name: Building Scan
  Scene Type: object
  Format: obj
```

**Interactive Example:**
```bash
$ raps reality create
Enter photoscene name: Building Scan
Select scene type:
  > aerial
    object
Select output format:
  > rcm
    rcs
    obj
    fbx
    ortho
Creating photoscene...
✓ Photoscene created!
```

**Scene Types:**
- `aerial` - Aerial/drone photography
- `object` - Object photography

**Output Formats:**
- `rcm` - Reality Capture Mesh
- `rcs` - Reality Capture Scene
- `obj` - Wavefront OBJ format
- `fbx` - FBX format
- `ortho` - Orthophoto

**Requirements:**
- 2-legged OAuth authentication

### `raps reality upload`

Upload photos to a photoscene.

**Usage:**
```bash
raps reality upload <photoscene-id> <photo1> [photo2] [photo3] ...
```

**Arguments:**
- `photoscene-id`: Photoscene ID
- `photo1`, `photo2`, etc.: Paths to photo files

**Example:**
```bash
$ raps reality upload abc123xyz photo1.jpg photo2.jpg photo3.jpg
Uploading photos...
  photo1.jpg... ✓
  photo2.jpg... ✓
  photo3.jpg... ✓
✓ All photos uploaded successfully!
```

**Requirements:**
- 2-legged OAuth authentication
- Photo files must exist and be readable

### `raps reality process`

Start processing a photoscene.

**Usage:**
```bash
raps reality process <photoscene-id>
```

**Arguments:**
- `photoscene-id`: Photoscene ID to process

**Example:**
```bash
$ raps reality process abc123xyz
Starting processing...
✓ Processing started!
  Photoscene ID: abc123xyz
  Status: processing
```

**Requirements:**
- 2-legged OAuth authentication
- Photoscene must have photos uploaded

### `raps reality status`

Check the status of a photoscene.

**Usage:**
```bash
raps reality status <photoscene-id> [--wait]
```

**Arguments:**
- `photoscene-id`: Photoscene ID to check

**Options:**
- `--wait, -w`: Wait for processing to complete (polls every 10 seconds)

**Example:**
```bash
$ raps reality status abc123xyz
Status: processing
Progress: 45%
```

**Example with --wait:**
```bash
$ raps reality status abc123xyz --wait
⋯ Status: processing, Progress: 45%
⋯ Status: processing, Progress: 78%
✓ Status: success, Progress: 100%
```

**Status Values:**
- `pending` - Photoscene created, waiting for photos
- `processing` - Processing in progress
- `success` - Processing completed successfully
- `failed` - Processing failed

**Requirements:**
- 2-legged OAuth authentication

### `raps reality result`

Get download link for processed results.

**Usage:**
```bash
raps reality result <photoscene-id> [--format FORMAT]
```

**Arguments:**
- `photoscene-id`: Photoscene ID

**Options:**
- `--format, -f`: Output format (default: `obj`)

**Example:**
```bash
$ raps reality result abc123xyz --format obj
Fetching result...

Result:
────────────────────────────────────────────────────────────
  Format: obj
  Download URL: https://developer.api.autodesk.com/.../result.obj
  Size: 45.2 MB
────────────────────────────────────────────────────────────
```

**Requirements:**
- 2-legged OAuth authentication
- Photoscene must be processed successfully

### `raps reality formats`

List available output formats.

**Usage:**
```bash
raps reality formats
```

**Example:**
```bash
$ raps reality formats

Available Formats:
────────────────────────────────────────────────────────────
  • rcm - Reality Capture Mesh
  • rcs - Reality Capture Scene
  • obj - Wavefront OBJ format
  • fbx - FBX format
  • ortho - Orthophoto
────────────────────────────────────────────────────────────
```

### `raps reality delete`

Delete a photoscene.

**Usage:**
```bash
raps reality delete <photoscene-id>
```

**Arguments:**
- `photoscene-id`: Photoscene ID to delete

**Example:**
```bash
$ raps reality delete abc123xyz
Deleting photoscene...
✓ Photoscene deleted successfully!
```

**Requirements:**
- 2-legged OAuth authentication

## Complete Workflow

### From Photos to 3D Model

```bash
# 1. Create a photoscene
raps reality create --name "Building Scan" --scene-type object --format obj

# 2. Upload photos
raps reality upload abc123xyz photo1.jpg photo2.jpg photo3.jpg photo4.jpg

# 3. Start processing
raps reality process abc123xyz

# 4. Wait for completion
raps reality status abc123xyz --wait

# 5. Get download link
raps reality result abc123xyz --format obj
```

## Photo Requirements

For best results:
- **Minimum photos**: 10-20 photos recommended
- **Overlap**: Photos should have 60-80% overlap
- **Coverage**: Cover the object/scene from multiple angles
- **Quality**: Use high-resolution photos (2MP+)
- **Format**: JPEG or PNG

### Aerial Photography Tips
- Fly in a grid pattern
- Maintain consistent altitude
- Ensure good overlap between passes

### Object Photography Tips
- Move around the object in a circle
- Take photos at multiple heights
- Include close-up and wide shots

## Processing Time

Processing time depends on:
- Number of photos (more photos = longer processing)
- Photo resolution (higher resolution = longer processing)
- Scene complexity
- Typically 10-60 minutes for small to medium projects

## Related Commands

- [Authentication](commands/auth.md) - Set up authentication
- [Objects](commands/objects.md) - Download processed results
- [Buckets](commands/buckets.md) - Store processed models


---
layout: default
title: Object Commands
---

# Object Commands

Manage objects (files) in OSS buckets.

## Commands

### `raps object upload`

Upload a file to a bucket.

**Usage:**
```bash
raps object upload [bucket] <file> [--key KEY]
```

**Arguments:**
- `bucket`: Bucket key (optional, will prompt if not provided)
- `file`: Path to the file to upload

**Options:**
- `--key, -k`: Object key (defaults to filename)

**Example:**
```bash
$ raps object upload my-bucket model.dwg
Uploading model.dwg to my-bucket/model.dwg
✓ Upload complete!
  Object ID: urn:adsk.objects:os.object:my-bucket/model.dwg
  Size: 2.45 MB
  SHA1: abc123def456...

  URN (for translation): dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6bXktYnVja2V0L21vZGVsLmR3Zw==
```

**Example with custom object key:**
```bash
$ raps object upload my-bucket model.dwg --key models/v1/model.dwg
Uploading model.dwg to my-bucket/models/v1/model.dwg
✓ Upload complete!
```

**Requirements:**
- 2-legged OAuth authentication
- Bucket must exist

### `raps object download`

Download an object from a bucket.

**Usage:**
```bash
raps object download [bucket] [object] [--output OUTPUT]
```

**Arguments:**
- `bucket`: Bucket key (optional, will prompt if not provided)
- `object`: Object key to download (optional, will prompt if not provided)

**Options:**
- `--output, -o`: Output file path (defaults to object key)

**Example:**
```bash
$ raps object download my-bucket model.dwg
Downloading my-bucket/model.dwg to model.dwg
✓ Download complete!
  Saved to: model.dwg
```

**Example with custom output path:**
```bash
$ raps object download my-bucket model.dwg --output ./downloads/model.dwg
Downloading my-bucket/model.dwg to ./downloads/model.dwg
✓ Download complete!
  Saved to: ./downloads/model.dwg
```

**Requirements:**
- 2-legged OAuth authentication

### `raps object list`

List all objects in a bucket.

**Usage:**
```bash
raps object list [bucket]
```

**Arguments:**
- `bucket`: Bucket key (optional, will prompt if not provided)

**Example:**
```bash
$ raps object list my-bucket
Fetching objects from 'my-bucket'...

Objects in my-bucket
────────────────────────────────────────────────────────────────────────────────
Object Key                                          Size           SHA1
────────────────────────────────────────────────────────────────────────────────
model.dwg                                           2.45 MB        abc123de
models/v1/model.dwg                                  1.23 MB        def456gh
textures/texture.jpg                                 512.45 KB      ghi789jk
────────────────────────────────────────────────────────────────────────────────
```

**Requirements:**
- 2-legged OAuth authentication

### `raps object delete`

Delete an object from a bucket.

**Usage:**
```bash
raps object delete [bucket] [object] [--yes]
```

**Arguments:**
- `bucket`: Bucket key (optional, will prompt if not provided)
- `object`: Object key to delete (optional, will prompt if not provided)

**Options:**
- `--yes, -y`: Skip confirmation prompt

**Example:**
```bash
$ raps object delete my-bucket model.dwg
Are you sure you want to delete 'my-bucket/model.dwg'? [y/N]: y
Deleting object...
✓ Object 'my-bucket/model.dwg' deleted successfully!
```

**Non-interactive Example:**
```bash
$ raps object delete my-bucket model.dwg --yes
Deleting object...
✓ Object 'my-bucket/model.dwg' deleted successfully!
```

**Requirements:**
- 2-legged OAuth authentication

### `raps object signed-url`

Generate a signed S3 URL for direct download (bypasses OSS servers).

**Usage:**
```bash
raps object signed-url <bucket> <object> [--minutes MINUTES]
```

**Arguments:**
- `bucket`: Bucket key
- `object`: Object key

**Options:**
- `--minutes, -m`: Expiration time in minutes (1-60, default: 2)

**Example:**
```bash
$ raps object signed-url my-bucket model.dwg --minutes 10
Generating signed S3 download URL...

✓ Signed URL generated!

Download URL (single part):
https://developer.api.autodesk.com/oss/v2/buckets/my-bucket/objects/model.dwg?...

  Size: 2.45 MB
  SHA1: abc123def456...
  Status: complete

Note: URL expires in 10 minutes
```

**Use Cases:**
- Share download links with clients or team members
- Direct downloads without going through OSS servers
- Integration with external systems

**Requirements:**
- 2-legged OAuth authentication

## Common Workflows

### Upload and Get URN for Translation

```bash
# Upload a file
raps object upload my-bucket model.dwg

# The URN is displayed after upload
# Use it for translation:
raps translate start dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6bXktYnVja2V0L21vZGVsLmR3Zw== --format svf2
```

### Download All Objects from a Bucket

```bash
# List objects first
raps object list my-bucket

# Download each object
raps object download my-bucket model.dwg --output ./downloads/model.dwg
raps object download my-bucket texture.jpg --output ./downloads/texture.jpg
```

### Share a File via Signed URL

```bash
# Generate a signed URL valid for 1 hour
raps object signed-url my-bucket model.dwg --minutes 60

# Share the URL with others (expires in 60 minutes)
```

## File Size Limits

- Maximum file size: 5 GB per object
- For larger files, consider chunked uploads (not currently supported by RAPS CLI)

## Object Keys

Object keys can include:
- Forward slashes (`/`) for organizing files in folders
- Letters, numbers, hyphens, underscores, dots
- Examples:
  - `model.dwg`
  - `models/v1/model.dwg`
  - `textures/diffuse/texture.jpg`

## Related Commands

- [Buckets](buckets.md) - Manage buckets
- [Translation](translation.md) - Translate uploaded files
- [Authentication](auth.md) - Set up authentication


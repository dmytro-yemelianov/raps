---
layout: default
title: Demo Commands
---

# Demo Commands

Run complete demo scenarios to learn RAPS CLI workflows and test APS functionality.

## Commands

### `raps demo bucket-lifecycle`

Complete bucket lifecycle demonstration: create, upload files, list, and cleanup.

**Usage:**
```bash
raps demo bucket-lifecycle [--prefix PREFIX] [--skip-cleanup]
```

**Options:**
- `--prefix`: Prefix for bucket names (default: auto-generated timestamp)
- `--skip-cleanup`: Skip cleanup at the end

**Example:**
```bash
$ raps demo bucket-lifecycle
Creating demo bucket...
✓ Bucket created: demo-1234567890

Uploading test files...
✓ File uploaded: test1.dwg
✓ File uploaded: test2.pdf

Listing objects...
[Shows list of objects]

Cleaning up...
✓ Bucket deleted
```

**What it demonstrates:**
- Creating buckets
- Uploading objects
- Listing objects
- Deleting buckets

### `raps demo model-pipeline`

End-to-end model processing pipeline: upload, translate, and check status.

**Usage:**
```bash
raps demo model-pipeline [--file FILE] [--bucket BUCKET] [--format FORMAT] [--keep-bucket]
```

**Options:**
- `--file, -f`: Path to model file (optional, generates synthetic if not provided)
- `--bucket, -b`: Bucket key (auto-generated if not provided)
- `--format`: Output format (default: `svf2`)
- `--keep-bucket`: Keep bucket after completion

**Example:**
```bash
$ raps demo model-pipeline --file model.dwg
Creating bucket...
✓ Bucket created

Uploading model...
✓ Model uploaded
  URN: dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6...

Starting translation...
✓ Translation started

Waiting for completion...
✓ Translation completed successfully!

Cleaning up...
✓ Bucket deleted
```

**What it demonstrates:**
- Creating buckets
- Uploading models
- Starting translations
- Monitoring translation status
- Viewing manifests

### `raps demo data-management`

Explore BIM 360/ACC hubs, projects, and folders interactively.

**Usage:**
```bash
raps demo data-management [--non-interactive] [--export EXPORT]
```

**Options:**
- `--non-interactive`: Run without prompts
- `--export`: Export data to JSON file

**Example:**
```bash
$ raps demo data-management
Listing hubs...
Select a hub:
  > My Company Hub
    ACC Project Hub

Listing projects...
Select a project:
  > Office Building Project
    Warehouse Renovation

Browsing folders...
[Shows folder structure]
```

**What it demonstrates:**
- Listing hubs
- Browsing projects
- Exploring folder structures
- Viewing items

**Requirements:**
- 3-legged OAuth authentication (`raps auth login`)

### `raps demo batch-processing`

Batch translation of multiple model files.

**Usage:**
```bash
raps demo batch-processing [--files FILE1] [FILE2] [FILE3] ... [--format FORMAT]
```

**Options:**
- `--files`: Model files to process
- `--format`: Output format (default: `svf2`)

**Example:**
```bash
$ raps demo batch-processing model1.dwg model2.dwg model3.dwg
Processing 3 files...

[1/3] Processing model1.dwg...
✓ Translation completed

[2/3] Processing model2.dwg...
✓ Translation completed

[3/3] Processing model3.dwg...
✓ Translation completed

All files processed successfully!
```

**What it demonstrates:**
- Batch file processing
- Parallel translation jobs
- Progress tracking
- Error handling

## Use Cases

### Learning RAPS CLI

Run demo scenarios to understand how RAPS CLI works:
```bash
# Start with bucket lifecycle
raps demo bucket-lifecycle

# Then try model pipeline
raps demo model-pipeline

# Explore data management
raps demo data-management
```

### Testing APS Setup

Use demos to verify your APS configuration:
```bash
# Test 2-legged OAuth
raps demo bucket-lifecycle

# Test 3-legged OAuth
raps demo data-management
```

### Demonstrating APS Features

Use demos to show APS capabilities to others:
```bash
# Show complete workflow
raps demo model-pipeline --file presentation-model.dwg

# Show batch processing
raps demo batch-processing model1.dwg model2.dwg model3.dwg
```

## Demo Data

Some demos can generate synthetic test data if files aren't provided:
- Model files (DWG, OBJ, STL, etc.)
- Metadata files (JSON)
- Sample project structures

## Related Commands

- [Buckets]({{ '/commands/buckets' | relative_url }}) - Bucket operations used in demos
- [Objects]({{ '/commands/objects' | relative_url }}) - Object operations used in demos
- [Translation]({{ '/commands/translation' | relative_url }}) - Translation operations used in demos
- [Data Management]({{ '/commands/data-management' | relative_url }}) - Data management operations used in demos


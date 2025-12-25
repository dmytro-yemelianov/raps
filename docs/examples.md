---
layout: default
title: Examples
---

# Examples

Common use cases and workflows with RAPS CLI.

## Quick Start Examples

### Upload and Translate a Model

Complete workflow from file upload to translated model:

```bash
# 1. Create a bucket
raps bucket create --key my-models --policy persistent --region US

# 2. Upload a CAD file
raps object upload my-models building.dwg
# Note the URN from output

# 3. Translate to SVF2 for viewing
raps translate start <urn> --format svf2

# 4. Wait for translation to complete
raps translate status <urn> --wait

# 5. View manifest
raps translate manifest <urn>
```

### Browse BIM 360/ACC Projects

Explore your BIM 360 or ACC projects:

```bash
# 1. Login with 3-legged OAuth
raps auth login

# 2. List hubs
raps hub list

# 3. List projects in a hub
raps project list b.abc123xyz

# 4. Browse folder contents
raps folder list b.project123 urn:adsk.wiprod:fs.folder:co.abc123xyz

# 5. View item versions
raps item versions b.project123 urn:adsk.wiprod:fs.file:co.def456uvw
```

## Advanced Examples

### Batch Process Multiple Files

Process multiple CAD files in sequence:

```bash
# Create a bucket
raps bucket create --key batch-processing --policy temporary --region US

# Upload multiple files
raps object upload batch-processing model1.dwg
raps object upload batch-processing model2.dwg
raps object upload batch-processing model3.dwg

# Translate each file
for urn in $(raps object list batch-processing | grep "urn:"); do
  raps translate start $urn --format svf2
done

# Or use the batch processing demo
raps demo batch-processing model1.dwg model2.dwg model3.dwg
```

### Set Up Webhook for File Uploads

Receive notifications when files are uploaded:

```bash
# 1. Create a webhook for version added events
raps webhook create \
  --url https://your-server.com/webhook \
  --event dm.version.added

# 2. List webhooks to verify
raps webhook list

# 3. Test by uploading a file to BIM 360/ACC
# Your webhook endpoint will receive a POST request
```

### Create and Manage Issues

Manage construction issues in ACC/BIM 360:

```bash
# 1. Login (required for issues)
raps auth login

# 2. List issue types
raps issue types project123

# 3. Create an issue
raps issue create project123 \
  --title "Missing window in Room 101" \
  --description "Window frame installed but glass missing"

# 4. List open issues
raps issue list project123 --status open

# 5. Update issue status
raps issue update project123 abc123xyz --status closed
```

### Photogrammetry Workflow

Create a 3D model from photos:

```bash
# 1. Create a photoscene
raps reality create \
  --name "Building Exterior" \
  --scene-type object \
  --format obj

# 2. Upload photos
raps reality upload abc123xyz \
  photo1.jpg photo2.jpg photo3.jpg \
  photo4.jpg photo5.jpg photo6.jpg

# 3. Start processing
raps reality process abc123xyz

# 4. Wait for completion
raps reality status abc123xyz --wait

# 5. Get download link
raps reality result abc123xyz --format obj
```

### Design Automation Workflow

Set up and use Design Automation:

```bash
# 1. Set Design Automation nickname
export APS_DA_NICKNAME="my-nickname"

# 2. List available engines
raps da engines

# 3. Create an app bundle
raps da appbundle-create \
  --id MyApp \
  --engine Autodesk.AutoCAD+24 \
  --description "My custom AutoCAD app"

# 4. Upload your bundle ZIP to the URL provided
# (Use curl or another tool)

# 5. List app bundles to verify
raps da appbundles

# 6. Check work item status
raps da status workitem123 --wait
```

## Integration Examples

### PowerShell Script

Automate tasks with PowerShell:

```powershell
# Set credentials
$env:APS_CLIENT_ID = "your_client_id"
$env:APS_CLIENT_SECRET = "your_client_secret"

# Create bucket and upload
raps bucket create --key my-bucket --policy persistent --region US
raps object upload my-bucket model.dwg

# Get URN and translate
$output = raps object upload my-bucket model.dwg 2>&1
$urn = ($output | Select-String "URN").ToString().Split(":")[-1]
raps translate start $urn --format svf2

# Wait for completion
raps translate status $urn --wait
```

### Bash Script

Automate tasks with Bash:

```bash
#!/bin/bash

# Set credentials
export APS_CLIENT_ID="your_client_id"
export APS_CLIENT_SECRET="your_client_secret"

# Create bucket
raps bucket create --key my-bucket --policy persistent --region US

# Upload file
raps object upload my-bucket model.dwg | tee upload.log

# Extract URN from log
URN=$(grep "URN" upload.log | awk '{print $NF}')

# Translate
raps translate start "$URN" --format svf2

# Wait for completion
raps translate status "$URN" --wait
```

### CI/CD Integration

Use RAPS CLI in CI/CD pipelines:

```yaml
# GitHub Actions example
name: Translate Models

on:
  push:
    paths:
      - 'models/**'

jobs:
  translate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install RAPS
        run: |
          wget https://github.com/dmytro-yemelianov/raps/releases/latest/download/raps-linux-x64.tar.gz
          tar -xzf raps-linux-x64.tar.gz
          sudo mv raps /usr/local/bin/
      
      - name: Translate models
        env:
          APS_CLIENT_ID: ${{ secrets.APS_CLIENT_ID }}
          APS_CLIENT_SECRET: ${{ secrets.APS_CLIENT_SECRET }}
        run: |
          raps bucket create --key ci-models --policy temporary --region US
          for file in models/*.dwg; do
            raps object upload ci-models "$file"
            # Translate and wait...
          done
```

## Best Practices

### Organize Files with Folders

Use object keys with paths to organize files:

```bash
# Upload to organized structure
raps object upload my-bucket model.dwg --key models/2024/january/model.dwg
raps object upload my-bucket texture.jpg --key textures/diffuse/texture.jpg
```

### Use Appropriate Retention Policies

Choose retention policies based on use case:

```bash
# Temporary test files
raps bucket create --key test --policy transient --region US

# Short-term processing
raps bucket create --key processing --policy temporary --region US

# Production data
raps bucket create --key production --policy persistent --region US
```

### Monitor Translation Status

Always check translation status:

```bash
# Start translation
raps translate start <urn> --format svf2

# Check status with wait
raps translate status <urn> --wait

# Or check periodically
while true; do
  raps translate status <urn>
  sleep 30
done
```

## Related Documentation

- [Getting Started](getting-started.md) - Overview and prerequisites
- [Command Reference](commands/buckets.md) - Complete command documentation
- [Troubleshooting](troubleshooting.md) - Common issues and solutions


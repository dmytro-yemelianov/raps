# Quickstart: RAPS Ecosystem Improvements

**Feature**: 001-raps-ecosystem-improvements  
**Date**: 2025-12-29

This guide demonstrates the improved RAPS ecosystem capabilities after implementation.

---

## Prerequisites

- APS account with credentials from [APS Developer Portal](https://aps.autodesk.com/myapps)
- Rust 1.88+ (for building from source)
- Docker (optional, for container usage)

---

## Installation Options

### Option 1: Cargo (Recommended)

```bash
cargo install raps
```

### Option 2: Pre-built Binary

```bash
# macOS/Linux
curl -fsSL https://rapscli.xyz/install.sh | sh

# Windows (PowerShell)
irm https://rapscli.xyz/install.ps1 | iex
```

### Option 3: Docker

```bash
docker pull dmytroyemelianov/raps:latest
```

### Option 4: GitHub Action

```yaml
- uses: dmytro-yemelianov/raps-action@v1
  with:
    command: bucket list
    client-id: ${{ secrets.APS_CLIENT_ID }}
    client-secret: ${{ secrets.APS_CLIENT_SECRET }}
```

---

## Configuration

```bash
# Set credentials (environment variables)
export APS_CLIENT_ID="your_client_id"
export APS_CLIENT_SECRET="your_client_secret"

# Or use profiles
raps config profile create production
raps config set client_id "your_client_id"
raps config set client_secret "your_client_secret"

# Verify authentication
raps auth test
```

---

## Improved Features

### 1. High-Performance Parallel Uploads

Upload large files 6x faster with parallel chunk uploads:

```bash
# Upload with parallel chunks (default: 5 concurrent)
raps object upload my-bucket large-model.dwg --parallel

# Custom concurrency for faster uploads on high-bandwidth connections
raps object upload my-bucket huge-file.rvt --parallel --concurrency 10

# Resume interrupted upload
raps object upload my-bucket huge-file.rvt --resume

# Batch upload with progress
raps object upload my-bucket ./models/ --batch --parallel
```

**Performance comparison (100MB file):**
| Mode | Time |
|------|------|
| Sequential (old) | ~120s |
| Parallel (new, 5 concurrent) | ~20s |
| Parallel (10 concurrent) | ~12s |

### 2. Consistent Output Schemas

All commands now output consistent JSON schemas:

```bash
# Get JSON output with documented schema
raps bucket list --output json

# Output follows documented schema
{
  "data": {
    "buckets": [...],
    "total": 42
  },
  "meta": {
    "next_marker": "abc123"
  }
}

# View schema documentation
raps schema bucket-list
```

### 3. Robust CI/CD Mode

Commands work reliably in non-interactive environments:

```bash
# Non-interactive mode fails fast on missing input
raps bucket create --non-interactive --key my-bucket --policy transient

# Auto-confirm destructive operations
raps bucket delete my-bucket --yes

# Structured exit codes for scripting
raps auth test && echo "Auth OK" || echo "Auth failed (exit: $?)"
```

**Exit codes:**
- `0` - Success
- `2` - Invalid arguments
- `3` - Authentication failure
- `4` - Resource not found
- `5` - API/network error
- `6` - Internal error

### 4. MCP Server for AI Assistants

Integrate RAPS with Claude, Cursor, or other MCP clients:

```bash
# Start MCP server
raps serve
```

**Claude Desktop configuration** (`claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "raps": {
      "command": "raps",
      "args": ["serve"],
      "env": {
        "APS_CLIENT_ID": "your_client_id",
        "APS_CLIENT_SECRET": "your_client_secret"
      }
    }
  }
}
```

**Available MCP tools:**
- `auth_test`, `auth_status`
- `bucket_list`, `bucket_create`, `bucket_get`, `bucket_delete`
- `object_list`, `object_upload`, `object_delete`, `object_signed_url`, `object_urn`
- `translate_start`, `translate_status`, `translate_download`
- `hub_list`, `project_list`, `folder_list`
- `issue_list`, `webhook_list`

**Example AI conversation:**
> "List my OSS buckets and create a new one called 'ai-test-bucket'"

### 5. GitHub Actions Integration

```yaml
name: APS Automation

on: [push]

jobs:
  deploy-model:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Upload CAD file
        uses: dmytro-yemelianov/raps-action@v1
        with:
          command: object upload my-bucket model.dwg --parallel
          client-id: ${{ secrets.APS_CLIENT_ID }}
          client-secret: ${{ secrets.APS_CLIENT_SECRET }}
      
      - name: Start translation
        id: translate
        uses: dmytro-yemelianov/raps-action@v1
        with:
          command: translate start ${{ steps.upload.outputs.urn }} --format svf2 --output json
          client-id: ${{ secrets.APS_CLIENT_ID }}
          client-secret: ${{ secrets.APS_CLIENT_SECRET }}
      
      - name: Check result
        run: echo "Translation started for ${{ fromJson(steps.translate.outputs.result).data.urn }}"
```

**Windows runner support:**
```yaml
jobs:
  windows-job:
    runs-on: windows-latest
    steps:
      - uses: dmytro-yemelianov/raps-action@v1
        with:
          command: bucket list
          client-id: ${{ secrets.APS_CLIENT_ID }}
          client-secret: ${{ secrets.APS_CLIENT_SECRET }}
```

### 6. Docker Container

```bash
# Run commands via Docker
docker run --rm \
  -e APS_CLIENT_ID="$APS_CLIENT_ID" \
  -e APS_CLIENT_SECRET="$APS_CLIENT_SECRET" \
  dmytroyemelianov/raps bucket list

# Upload files with volume mount
docker run --rm \
  -v $(pwd)/models:/data \
  -e APS_CLIENT_ID="$APS_CLIENT_ID" \
  -e APS_CLIENT_SECRET="$APS_CLIENT_SECRET" \
  dmytroyemelianov/raps object upload my-bucket /data/model.dwg

# Docker Compose
```

**docker-compose.yml:**
```yaml
services:
  raps:
    image: dmytroyemelianov/raps:latest
    environment:
      - APS_CLIENT_ID
      - APS_CLIENT_SECRET
    volumes:
      - ./data:/data
    healthcheck:
      test: ["CMD", "raps", "auth", "test", "--timeout", "5"]
      interval: 30s
      timeout: 5s
      retries: 3
```

### 7. TUI (Terminal User Interface)

Interactive exploration of APS resources:

```bash
# Launch TUI
aps-tui
```

**Keyboard shortcuts:**
- `Tab` / `Shift+Tab` - Navigate panels
- `j` / `k` or `↓` / `↑` - Move up/down
- `Enter` - Select/drill down
- `/` - Search/filter
- `u` - Upload file (in Objects panel)
- `d` - Download file
- `Delete` - Delete resource
- `t` - Start translation
- `?` - Help
- `q` - Quit

---

## Common Workflows

### Workflow 1: Model Upload and Translation

```bash
# 1. Create bucket
raps bucket create --key my-project-bucket --policy persistent

# 2. Upload model (parallel for speed)
raps object upload my-project-bucket building.rvt --parallel

# 3. Get URN
URN=$(raps object urn my-project-bucket building.rvt)

# 4. Start translation
raps translate start "$URN" --format svf2

# 5. Wait for completion
raps translate status "$URN" --wait

# 6. Download derivatives
raps translate download "$URN" --output ./derivatives/
```

### Workflow 2: Batch Processing Pipeline

```yaml
# pipeline.yaml
name: Model Processing Pipeline
steps:
  - name: Upload models
    command: object upload my-bucket ./models/ --batch --parallel
    
  - name: Translate all
    command: translate start --batch --format svf2
    
  - name: Wait for translations
    command: translate status --wait --all
```

```bash
raps pipeline run pipeline.yaml --continue-on-error
```

### Workflow 3: ACC Project Automation

```bash
# Login with 3-legged OAuth
raps auth login

# List hubs
raps hub list

# Browse projects
raps project list <hub-id>

# Create folder
raps folder create <project-id> <parent-folder-id> "My Folder"

# Bind uploaded object
raps item bind <project-id> <folder-id> <urn>

# Create issue
raps issue create <project-id> --title "Review model" --due-date 2025-01-15
```

---

## Troubleshooting

### Authentication Issues

```bash
# Debug mode shows request details
raps auth test --debug

# Inspect token
raps auth inspect-token

# Clear cached tokens
raps auth logout
```

### Upload Failures

```bash
# Check resume state
raps upload-state list

# Clear stuck session
raps upload-state clear <session-id>

# Retry with detailed logging
raps object upload my-bucket file.dwg --verbose --parallel
```

### Rate Limiting

```bash
# Reduce concurrency
raps object upload my-bucket ./files/ --batch --concurrency 2

# Custom retry settings
raps config set max_retries 5
raps config set retry_base_delay 2
```

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `APS_CLIENT_ID` | OAuth client ID | Required |
| `APS_CLIENT_SECRET` | OAuth client secret | Required |
| `APS_CALLBACK_URL` | 3-legged OAuth callback | `http://localhost:8080/callback` |
| `APS_REGION` | Default region (US/EMEA) | `US` |
| `RAPS_USE_KEYCHAIN` | Use OS keychain for tokens | `false` |
| `RAPS_CONCURRENCY` | Default parallel operations | `5` |
| `RAPS_TIMEOUT` | HTTP timeout (seconds) | `120` |
| `NO_COLOR` | Disable colored output | Not set |

---

## Getting Help

```bash
# Command help
raps --help
raps bucket --help
raps object upload --help

# Version info
raps --version

# Generate shell completions
raps completions bash >> ~/.bashrc
raps completions powershell >> $PROFILE
```

**Resources:**
- [Documentation](https://rapscli.xyz/docs)
- [API Reference](https://rapscli.xyz/api)
- [GitHub Issues](https://github.com/dmytro-yemelianov/raps/issues)
- [Changelog](https://rapscli.xyz/changelog)

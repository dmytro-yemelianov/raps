## 🌼 RAPS v3.0.0 - MCP Server Integration

### Summary
This is a **major version release** that introduces Model Context Protocol (MCP) server support, enabling AI assistants like Claude Desktop and Cursor to interact directly with Autodesk Platform Services.

### New Command
- **`raps serve`** - Starts the MCP server for AI assistant integration

### 14 MCP Tools Implemented
| Tool | Description |
|------|-------------|
| `auth_test` | Test 2-legged OAuth credentials |
| `auth_status` | Check authentication status (2-legged and 3-legged) |
| `bucket_list` | List OSS buckets with optional region filter |
| `bucket_create` | Create new OSS bucket with retention policy |
| `bucket_get` | Get bucket details |
| `bucket_delete` | Delete empty bucket |
| `object_list` | List objects in bucket |
| `object_delete` | Delete object from bucket |
| `object_signed_url` | Generate pre-signed S3 download URL |
| `object_urn` | Get Base64-encoded URN for translation |
| `translate_start` | Start CAD file translation |
| `translate_status` | Check translation job status |
| `hub_list` | List accessible BIM 360/ACC hubs |
| `project_list` | List projects in a hub |

### Architecture
The MCP server **reuses the existing RAPS codebase** - no duplicate API code:
- Uses existing `AuthClient`, `OssClient`, `DerivativeClient`, `DataManagementClient`
- Uses existing `Config` and `HttpClientConfig`
- MCP is a new interface layer, like CLI but for AI assistants

### Dependencies Added
- `rmcp` v0.12 - Official Rust SDK for Model Context Protocol
- `schemars` v0.8 - JSON Schema generation
- `tracing` / `tracing-subscriber` - Logging

### Usage

```bash
raps serve
```

**Claude Desktop** (`claude_desktop_config.json`):

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

**Cursor** (`.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "raps": {
      "command": "raps",
      "args": ["serve"]
    }
  }
}
```

### Breaking Changes
- Version bumped to 3.0.0 (major version for new capability paradigm)
- No breaking changes to existing CLI commands

### Checklist
- [x] New `raps serve` command added
- [x] 14 MCP tools implemented
- [x] Reuses existing API clients
- [x] CHANGELOG.md updated
- [x] README.md updated with MCP documentation
- [x] Compiles without warnings


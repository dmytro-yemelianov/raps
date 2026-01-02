---
layout: default
title: MCP Server
---

# MCP Server (v3.0.0+)

The MCP (Model Context Protocol) server enables AI assistants like Claude Desktop and Cursor to interact directly with Autodesk Platform Services.

## Overview

MCP is an open protocol that allows AI assistants to access external tools and data sources. RAPS implements an MCP server that exposes APS APIs as tools, enabling natural language interaction with your Autodesk cloud resources.

## Starting the Server

```bash
raps serve
```

The server communicates via stdio (standard input/output), making it compatible with all MCP clients.

## Configuration

### Claude Desktop

Add to your Claude Desktop configuration file:

**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`
**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`

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

### Cursor IDE

Add to `.cursor/mcp.json` in your project or global config:

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

For global configuration, create at:
- **Windows:** `%USERPROFILE%\.cursor\mcp.json`
- **macOS/Linux:** `~/.cursor/mcp.json`

### Environment Variables

The MCP server uses the same environment variables as the CLI:

```bash
# Required for 2-legged operations
APS_CLIENT_ID=your_client_id
APS_CLIENT_SECRET=your_client_secret

# Optional - for 3-legged operations, login via CLI first
# raps auth login
```

## Available Tools

The MCP server exposes 14 tools organized by functionality:

### Authentication Tools

| Tool | Description |
|------|-------------|
| `auth_test` | Test 2-legged OAuth credentials |
| `auth_status` | Check authentication status (2-legged and 3-legged) |

### Bucket Tools

| Tool | Description |
|------|-------------|
| `bucket_list` | List OSS buckets with optional region filter (US/EMEA) |
| `bucket_create` | Create new bucket with retention policy |
| `bucket_get` | Get bucket details (owner, policy, creation date) |
| `bucket_delete` | Delete an empty bucket |

### Object Tools

| Tool | Description |
|------|-------------|
| `object_list` | List objects in a bucket |
| `object_delete` | Delete an object from a bucket |
| `object_signed_url` | Generate pre-signed S3 download URL (2-60 min) |
| `object_urn` | Get Base64-encoded URN for translation |

### Translation Tools

| Tool | Description |
|------|-------------|
| `translate_start` | Start CAD file translation (svf2, obj, stl, step, iges, ifc) |
| `translate_status` | Check translation job status |

### Data Management Tools

| Tool | Description |
|------|-------------|
| `hub_list` | List accessible BIM 360/ACC hubs (requires 3-legged auth) |
| `project_list` | List projects in a hub (requires 3-legged auth) |

## Example Conversations

Once configured, you can interact with APS using natural language:

### List Buckets
> "Show me all my APS buckets"

The AI will call `bucket_list` and display the results.

### Create a Bucket
> "Create a new bucket called 'my-project-files' with persistent retention in US region"

The AI will call `bucket_create` with the appropriate parameters.

### Upload and Translate Workflow
> "What objects are in my 'design-files' bucket?"

> "Get me a download URL for 'model.rvt' that expires in 30 minutes"

> "What's the URN for 'model.rvt' in the 'design-files' bucket?"

> "Start translating that URN to SVF2 format"

> "Check the translation status"

### BIM 360/ACC Navigation
> "List all my BIM 360 hubs"

> "Show me the projects in hub ID 'b.abc123'"

## Architecture

The MCP server reuses the existing RAPS codebase:

```
┌─────────────────────────────────────────────────────┐
│                  AI Assistant                        │
│            (Claude, Cursor, etc.)                    │
└─────────────────┬───────────────────────────────────┘
                  │ MCP Protocol (stdio)
                  ▼
┌─────────────────────────────────────────────────────┐
│               RAPS MCP Server                        │
│                 raps serve                           │
├─────────────────────────────────────────────────────┤
│            Shared API Clients                        │
│  • AuthClient    • OssClient                        │
│  • DerivativeClient  • DataManagementClient         │
└─────────────────┬───────────────────────────────────┘
                  │ HTTPS
                  ▼
┌─────────────────────────────────────────────────────┐
│           Autodesk Platform Services                 │
│  OSS • Model Derivative • Data Management • ...      │
└─────────────────────────────────────────────────────┘
```

## Debugging

Enable verbose logging by setting the `RUST_LOG` environment variable:

```bash
RUST_LOG=debug raps serve
```

Logs are written to stderr, so they won't interfere with the MCP protocol on stdout.

## Requirements

- APS credentials (`APS_CLIENT_ID`, `APS_CLIENT_SECRET`)
- For 3-legged operations: Login via `raps auth login` before starting the server
- MCP-compatible AI client (Claude Desktop, Cursor, etc.)

## Limitations

- **No file uploads** - The MCP server cannot accept file uploads directly. Upload files using the CLI (`raps object upload`) first.
- **No interactive prompts** - All parameters must be provided by the AI assistant.
- **Token storage** - 3-legged tokens are read from the same storage as the CLI. Login with `raps auth login` before starting the server.

## Related

- [Authentication](auth.md) - Setup credentials and login
- [Buckets](buckets.md) - Bucket operations via CLI
- [Objects](objects.md) - Object operations via CLI
- [Translation](translation.md) - Model Derivative operations via CLI


---
layout: default
title: Hosted MCP Server
---

# Hosted MCP Server

The hosted RAPS MCP server at `mcp.rapscli.xyz` lets AI assistants connect to Autodesk Platform Services without installing or running anything locally. Point your MCP client at the URL, authenticate with your license key, and all 107+ APS tools are available immediately.

## Prerequisites

1. **Pro License** — an active RAPS Pro subscription (any plugin tier). Purchase at [rapscli.xyz](https://rapscli.xyz).
2. **APS Credentials** — a Client ID and Client Secret from the [APS Developer Portal](https://aps.autodesk.com/myapps).

## Setup

### 1. Store Your License Key

```bash
raps marketplace license <your-license-key>
```

### 2. Store Your APS Credentials

Your APS credentials are encrypted and stored server-side so the hosted MCP server can authenticate with Autodesk APIs on your behalf.

```bash
raps marketplace credentials set --client-id <YOUR_CLIENT_ID> --client-secret <YOUR_CLIENT_SECRET>
```

### 3. Verify Stored Credentials

```bash
raps marketplace credentials list
```

This shows your stored credential labels and Client IDs (secrets are never displayed).

## Client Configuration

All MCP clients that support Streamable HTTP can connect using the same URL and authorization header:

- **URL:** `https://mcp.rapscli.xyz/mcp`
- **Authorization:** `Bearer <your-license-key>`

### Claude Desktop

Add to your Claude Desktop configuration file:

**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`
**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "raps": {
      "url": "https://mcp.rapscli.xyz/mcp",
      "headers": {
        "Authorization": "Bearer <your-license-key>"
      }
    }
  }
}
```

Restart Claude Desktop after saving.

### Cursor

Add to `.cursor/mcp.json` in your project or global config:

**Global:** `~/.cursor/mcp.json` (macOS/Linux) or `%USERPROFILE%\.cursor\mcp.json` (Windows)

```json
{
  "mcpServers": {
    "raps": {
      "url": "https://mcp.rapscli.xyz/mcp",
      "headers": {
        "Authorization": "Bearer <your-license-key>"
      }
    }
  }
}
```

### Claude Code

Add to `.mcp.json` in your project root:

```json
{
  "mcpServers": {
    "raps": {
      "url": "https://mcp.rapscli.xyz/mcp",
      "headers": {
        "Authorization": "Bearer <your-license-key>"
      }
    }
  }
}
```

### Generic (Any Streamable HTTP Client)

Any MCP client that supports Streamable HTTP transport (MCP spec 2025-03-26) can connect with:

| Parameter | Value |
|-----------|-------|
| URL | `https://mcp.rapscli.xyz/mcp` |
| Method | `POST` (client-to-server), `GET` (SSE stream) |
| Authorization Header | `Bearer <your-license-key>` |
| Content-Type | `application/json` |
| Accept | `application/json, text/event-stream` |

## How It Works

```
Client (Claude Desktop, Cursor, etc.)
│
│  Streamable HTTP (POST/GET + SSE)
│  Authorization: Bearer <license-key>
▼
┌──────────────────────────────────────┐
│  Cloudflare Worker Gateway           │
│  mcp.rapscli.xyz                     │
│                                      │
│  1. Validates license key            │
│  2. Resolves your APS credentials    │
│  3. Routes to your container         │
│  4. Enforces rate limits             │
└──────────────┬───────────────────────┘
               │
               ▼
┌──────────────────────────────────────┐
│  Isolated Container (per session)    │
│                                      │
│  Runs the same raps binary as local  │
│  APS credentials injected via env    │
│  Auto-sleeps after 5 min idle        │
│  Auto-wakes on next request          │
└──────────────┬───────────────────────┘
               │
               ▼
        Autodesk Platform Services
```

Each license key gets its own isolated container. The container runs the same `raps` binary used locally, so every tool behaves identically. Containers auto-sleep after 5 minutes of inactivity (no cost while sleeping) and auto-wake on the next request.

## Available Tools

The hosted server exposes the same 107+ tools as the local `raps mcp` command, covering:

- **Authentication** — test credentials, check status
- **Buckets** — create, list, delete OSS buckets
- **Objects** — list, delete, generate signed URLs, get URNs
- **Translation** — start and monitor Model Derivative jobs
- **Data Management** — browse hubs, projects, folders, items
- **ACC** — issues, RFIs, submittals, checklists, assets
- **Design Automation** — engines, app bundles, work items
- **Webhooks** — create, list, update, delete
- **Admin** — bulk user management across projects

Full tool reference: [rapscli.xyz/docs/mcp](https://rapscli.xyz/docs/mcp)

## Troubleshooting

### 401 Unauthorized

**Cause:** Invalid or missing license key.

**Fix:** Verify your license key is correct and included in the `Authorization` header. Re-store it if needed:

```bash
raps marketplace license <your-license-key>
```

### 403 Forbidden

**Cause:** Your Pro subscription has expired.

**Fix:** Renew your subscription at [rapscli.xyz](https://rapscli.xyz).

### 429 Too Many Requests

**Cause:** Rate limit exceeded (100 requests per minute per license).

**Fix:** Wait a moment and retry. If you consistently hit the limit, reduce request frequency or batch operations.

### Connection Timeout on First Request

**Cause:** Container cold start. The first request after a period of inactivity takes ~3-5 seconds while the container wakes up.

**Fix:** This is expected. Subsequent requests within the same session are fast. No action needed.

### "No APS Credentials" Error

**Cause:** You have not stored your APS credentials for the hosted server.

**Fix:** Store them with the CLI:

```bash
raps marketplace credentials set --client-id <YOUR_CLIENT_ID> --client-secret <YOUR_CLIENT_SECRET>
```

Then verify they are stored:

```bash
raps marketplace credentials list
```

## Comparison: Local vs Hosted MCP

| | Local (`raps mcp`) | Hosted (`mcp.rapscli.xyz`) |
|---|---|---|
| Installation | Download and install `raps` binary | None |
| Transport | stdio | Streamable HTTP |
| APS Credentials | Environment variables or `.env` file | Stored via `raps marketplace credentials set` |
| License | Free (open source) | Pro subscription required |
| 3-Legged Auth | `raps auth login` (opens browser) | Not yet supported |
| Tools | 107+ | 107+ (identical) |
| Performance | Instant (local process) | ~3-5s cold start, then fast |

## Related

- [MCP Server (Local)](commands/mcp.md) — local stdio-based MCP server
- [Configuration](configuration.md) — APS credential setup for local use
- [Getting Started](getting-started.md) — initial RAPS setup
- [Installation](installation.md) — installing the RAPS binary

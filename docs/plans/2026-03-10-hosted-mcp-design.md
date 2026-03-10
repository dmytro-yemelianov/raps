# Hosted RAPS MCP Server — Design Document

**Date:** 2026-03-10
**Status:** Draft
**Author:** Dmytro Yemelianov + Claude

## Problem

The RAPS MCP server (`raps mcp`) currently only runs locally via stdio transport. Users must install the CLI, configure APS credentials, and run the binary on their machine. This limits adoption — especially for users who just want to connect Claude Desktop or Cursor to APS APIs without local setup.

## Goal

Host the RAPS MCP server at `mcp.rapscli.xyz` so any Pro subscriber can connect by URL with their license key. The server must:

- Run the existing Rust binary without rewriting tool handlers
- Support multiple concurrent users with isolated APS credentials
- Auto-scale horizontally (more users → more instances)
- Scale to zero when idle (no cost when unused)
- Use Streamable HTTP transport (MCP spec 2025-03-26)

## Architecture

```
Client (Claude Desktop, Cursor, etc.)
│
│  Streamable HTTP (POST/GET + SSE)
│  Authorization: Bearer <license-key>
▼
┌──────────────────────────────────────────┐
│  Cloudflare Worker                       │
│  mcp.rapscli.xyz                         │
│                                          │
│  1. Validate license key (D1)            │
│  2. Resolve APS credentials (D1)         │
│  3. Route to Container by session ID     │
│  4. Rate limit (KV)                      │
└──────────┬───────────────────────────────┘
           │  container.fetch(request)
           ▼
┌──────────────────────────────────────────┐
│  Cloudflare Container (per session)      │
│                                          │
│  Docker image: raps-mcp                  │
│  - raps mcp --transport http --port 8080 │
│  - APS_CLIENT_ID / APS_CLIENT_SECRET     │
│    injected via envVars at creation      │
│  - sleepAfter: 5m (auto-sleep on idle)   │
│  - Auto-wakes on next request            │
└──────────────────────────────────────────┘
           │
           │  HTTPS (APS OAuth)
           ▼
     Autodesk Platform Services
```

### Why Cloudflare Containers

- **No rewrite** — the compiled Rust binary runs as-is inside a Docker container.
- **Per-session isolation** — each session gets its own container with separate environment variables. A crash in one session cannot affect another.
- **Auto-scale** — Cloudflare creates containers on demand and sleeps them after inactivity. No capacity planning.
- **Same stack** — D1, KV, R2, and the marketplace API Worker are already on Cloudflare.
- **Pay-per-use** — billed per 10ms of active runtime. Sleeping containers cost nothing.

## Components

### 1. Rust MCP Server (transport changes)

**Current state:** `raps mcp` starts a stdio-only MCP server using `rmcp` 1.1 with `transport-io` feature.

**Changes:**
- Add rmcp features: `transport-streamable-http-server`, `transport-streamable-http-server-session`
- Add `--transport` flag to `raps mcp`: `stdio` (default) or `http`
- Add `--port` flag (default 8080, only used with `--transport http`)
- When `--transport http`, create an axum router mounting `StreamableHttpService` at `/mcp`
- `RapsServer` factory function receives APS credentials from environment variables (already the case)
- Per-session isolation provided by rmcp's `StreamableHttpService` which creates a new `RapsServer` per session

**Key code pattern (from rmcp docs):**
```rust
let service = StreamableHttpService::new(
    || Ok(RapsServer::new()?),           // factory: new server per session
    LocalSessionManager::default().into(),
    Default::default(),
);
let app = axum::Router::new().nest_service("/mcp", service);
axum::serve(listener, app).await?;
```

**No changes to tool handlers.** All 107+ tools work as-is.

### 2. Docker Image

Minimal image for the `raps` binary:

```dockerfile
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY raps /usr/local/bin/raps
EXPOSE 8080
CMD ["raps", "mcp", "--transport", "http", "--port", "8080"]
```

Built for `linux/amd64`. Cross-compiled from the existing CI release workflow or built locally with `cargo build --release --target x86_64-unknown-linux-gnu`.

Image pushed to Cloudflare Container Registry or referenced as a local Dockerfile in wrangler config.

### 3. Cloudflare Worker (Gateway)

**Endpoint:** `mcp.rapscli.xyz`

**Responsibilities:**
1. **License authentication** — extract `Authorization: Bearer <key>` header, hash key, validate against `licenses` table in D1 (same database as marketplace API)
2. **Credential resolution** — look up the customer's stored APS credentials from D1 (new `customer_credentials` table)
3. **Session routing** — derive a session ID from the license key + `Mcp-Session-Id` header, route to a named container instance
4. **Rate limiting** — per-license rate limit using KV (e.g., 100 requests/min)
5. **Proxy** — forward the Streamable HTTP request to the container, stream the response back

**Worker pseudo-code:**
```typescript
export default {
  async fetch(request: Request, env: Env) {
    // 1. Auth
    const licenseKey = extractBearer(request)
    const license = await validateLicense(env.DB, licenseKey)
    if (!license) return new Response('Unauthorized', { status: 401 })

    // 2. Rate limit
    if (await isRateLimited(env.RATE_LIMIT, license.id)) {
      return new Response('Too Many Requests', { status: 429 })
    }

    // 3. Get APS credentials
    const creds = await getCredentials(env.DB, license.customerId)

    // 4. Route to container
    const sessionId = deriveSessionId(license.id, request)
    const container = getContainer(env.MCP_CONTAINER, sessionId)

    // 5. Inject creds as env vars on first start
    container.env = {
      APS_CLIENT_ID: creds.clientId,
      APS_CLIENT_SECRET: creds.clientSecret,
    }

    // 6. Proxy
    return container.fetch(request)
  }
}
```

### 4. Database Schema Additions

New table in the existing `raps-marketplace` D1 database:

```sql
CREATE TABLE IF NOT EXISTS customer_credentials (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id INTEGER NOT NULL REFERENCES customers(id),
  label TEXT NOT NULL DEFAULT 'default',
  aps_client_id TEXT NOT NULL,
  aps_client_secret_encrypted TEXT NOT NULL,
  aps_callback_url TEXT DEFAULT 'http://localhost:8080/callback',
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(customer_id, label)
);
```

- `aps_client_secret_encrypted` — encrypted with a Worker secret (`CREDENTIAL_ENCRYPTION_KEY`), decrypted at runtime before injecting into the container.
- Customers can store multiple credential sets (labeled), defaulting to `default`.

### 5. APS Credential Management

New `raps marketplace credentials` CLI commands:

| Command | Description |
|---------|-------------|
| `raps marketplace credentials set` | Store APS credentials (encrypted, sent to API) |
| `raps marketplace credentials list` | List stored credential labels |
| `raps marketplace credentials delete <label>` | Remove stored credentials |

Alternatively, clients can pass credentials at session start using the existing `auth_configure` tool or custom headers.

### 6. Wrangler Configuration

```jsonc
// wrangler.json (raps-mcp-gateway worker)
{
  "name": "raps-mcp-gateway",
  "main": "src/index.ts",
  "containers": [{
    "class_name": "McpContainer",
    "image": "./Dockerfile",
    "max_instances": 100
  }],
  "durable_objects": {
    "bindings": [{
      "name": "MCP_CONTAINER",
      "class_name": "McpContainer"
    }]
  },
  "d1_databases": [{
    "binding": "DB",
    "database_name": "raps-marketplace",
    "database_id": "bcfa7aac-79b2-4090-bb1e-623feba2bf8d"
  }],
  "kv_namespaces": [{
    "binding": "RATE_LIMIT",
    "id": "..."
  }],
  "routes": [{
    "pattern": "mcp.rapscli.xyz/*",
    "zone_name": "rapscli.xyz"
  }]
}
```

## Session Lifecycle

1. **Connect** — client sends `POST mcp.rapscli.xyz/mcp` with `InitializeRequest` and `Authorization: Bearer <key>`
2. **Auth** — Worker validates license, resolves APS credentials
3. **Container start** — Worker routes to a named container (keyed by license ID). If sleeping, Cloudflare wakes it (~seconds). If new, starts fresh with APS env vars injected.
4. **Session established** — container returns `InitializeResult` with `Mcp-Session-Id`. Client includes this header on all subsequent requests.
5. **Tool calls** — client POSTs tool requests. Worker proxies to the same container. Container responds with JSON or SSE stream.
6. **Idle** — after 5min with no requests, container auto-sleeps. No cost while sleeping.
7. **Resume** — next request auto-wakes the container. Session state is preserved (in-memory caches for auth tokens, etc.).
8. **Disconnect** — client sends `DELETE /mcp` with session ID, or session times out.

## Transport Details (MCP Spec 2025-03-26)

### Streamable HTTP

- **Endpoint:** `POST /mcp` for all client→server messages, `GET /mcp` for server→client SSE stream
- **Session:** Server returns `Mcp-Session-Id` header on `InitializeResult`. Client includes it on all subsequent requests.
- **Response modes:** Server can respond with `application/json` (simple) or `text/event-stream` (SSE, for streaming/progress)
- **Resumability:** Optional — SSE events can include `id` fields for reconnection via `Last-Event-ID`

### Backward Compatibility (Legacy SSE)

For clients that only support the old HTTP+SSE transport (2024-11-05):
- The gateway can detect this (GET request to establish SSE first, then POST to a separate endpoint)
- Not required for initial launch — Streamable HTTP is sufficient for Claude Desktop and modern clients

## Security

| Concern | Mitigation |
|---------|-----------|
| License key in transit | HTTPS only (Cloudflare edge TLS) |
| APS secrets at rest | Encrypted in D1 with `CREDENTIAL_ENCRYPTION_KEY` Worker secret |
| APS secrets to container | Injected via `envVars` — never exposed to client |
| Cross-session access | Each session is a separate container process |
| DNS rebinding | Worker validates `Origin` header |
| Rate abuse | Per-license rate limiting via KV |
| Container escape | Cloudflare's gVisor-based isolation |

## Cost Estimate

| Component | Monthly cost |
|-----------|-------------|
| Worker requests (gateway) | $0 (included in $5/mo Workers Paid) |
| D1 reads (license + credential lookups) | $0 (included in $5/mo) |
| KV reads/writes (rate limiting) | $0 (included) |
| Containers — memory (256MB × active hours) | ~$0.50-2 (auto-sleep) |
| Containers — CPU | ~$0.50-2 (pay per 10ms active) |
| **Total incremental** | **~$1-4/month** (on top of existing $5/mo Workers Paid) |

Scales linearly: 10× more users ≈ 10× container cost, but auto-sleep keeps idle sessions free.

## Milestones

1. **M1: HTTP transport in raps binary** — add `--transport http` flag, axum + rmcp StreamableHttpService, session factory
2. **M2: Docker image + CI** — Dockerfile, cross-compile, push to registry
3. **M3: Gateway Worker** — license auth, credential lookup, container routing, rate limiting
4. **M4: Database + credential management** — `customer_credentials` table, encryption, CLI commands
5. **M5: Deploy + DNS** — wrangler deploy, `mcp.rapscli.xyz` route, end-to-end test
6. **M6: Documentation** — connection guide, credential setup, client configuration examples

## Client Configuration Examples

### Claude Desktop (`claude_desktop_config.json`)
```json
{
  "mcpServers": {
    "raps": {
      "url": "https://mcp.rapscli.xyz/mcp",
      "headers": {
        "Authorization": "Bearer <license-key>"
      }
    }
  }
}
```

### Cursor
```json
{
  "mcpServers": {
    "raps": {
      "url": "https://mcp.rapscli.xyz/mcp",
      "headers": {
        "Authorization": "Bearer <license-key>"
      }
    }
  }
}
```

## Open Questions

1. **3-legged OAuth** — tools that need user-context auth (Data Management, ACC) currently require `raps auth login` which opens a browser. For hosted MCP, we need an alternative flow (e.g., client provides a refresh token, or we implement device code flow).
2. **Container cold start** — how long does it take for a Cloudflare Container to wake from sleep? If >3s, we may want keep-alive pings for active subscribers.
3. **Container image size** — the `raps` release binary is ~43MB. With a slim Debian base, the image should be ~60-80MB. Verify this is within Cloudflare Container limits.
4. **Credential encryption key rotation** — design a key rotation strategy for `CREDENTIAL_ENCRYPTION_KEY`.

# Hosted MCP Server Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Host the RAPS MCP server at `mcp.rapscli.xyz` using Cloudflare Worker gateway + Cloudflare Containers, with per-session isolation and auto-scaling.

**Architecture:** CF Worker authenticates license keys and routes requests to per-session CF Containers running the `raps mcp --transport http` binary. Each container gets its own APS credentials via environment variables. Containers auto-sleep after 5min idle.

**Tech Stack:** Rust (rmcp 1.1 with `transport-streamable-http-server`), axum, Docker, Cloudflare Workers (TypeScript/Hono), Cloudflare Containers, D1, KV.

**Design doc:** `docs/plans/2026-03-10-hosted-mcp-design.md`

---

## Task 1: Add Streamable HTTP transport to `raps mcp`

**Files:**
- Modify: `Cargo.toml:116` (workspace rmcp features)
- Modify: `raps-cli/Cargo.toml:150-158` (add `mcp-http` feature)
- Modify: `raps-cli/src/mcp/server.rs:16,530-539` (imports + `run_server`)
- Modify: `raps-cli/src/main.rs:359-360,807-812` (Mcp command args + dispatch)

**Step 1: Add rmcp HTTP transport features to workspace Cargo.toml**

In `Cargo.toml:116`, update rmcp dependency:

```toml
rmcp = { version = "1.1", features = ["server", "transport-io", "schemars"] }
```
→
```toml
rmcp = { version = "1.1", features = ["server", "transport-io", "transport-streamable-http-server", "transport-streamable-http-server-session", "schemars"] }
```

**Step 2: Add `mcp-http` feature flag to raps-cli**

In `raps-cli/Cargo.toml`, add axum as a non-optional dep (it's already in workspace) and a feature:

```toml
[features]
default = []
dashboard = ["ratatui", "crossterm"]
mcp-http = ["dep:axum"]
redis = ["raps-kernel/redis", "dep:hostname", "dep:redis", "dep:deadpool-redis"]
kubernetes = ["raps-kernel/kubernetes", "redis", "dep:axum"]
h3 = ["raps-kernel/h3"]
```

Keep `axum = { workspace = true, optional = true }` as-is — it's already there.

**Step 3: Add transport/port args to the Mcp command**

In `raps-cli/src/main.rs`, change:

```rust
    /// Start MCP (Model Context Protocol) server for AI assistant integration
    Mcp,
```
→
```rust
    /// Start MCP (Model Context Protocol) server for AI assistant integration
    Mcp {
        /// Transport: stdio (default) or http
        #[arg(long, default_value = "stdio")]
        transport: String,

        /// Port for HTTP transport (ignored for stdio)
        #[arg(long, default_value = "8080")]
        port: u16,
    },
```

Update the match in main.rs (~line 807):

```rust
    if let Commands::Mcp { transport, port } = &command {
        mcp::server::run_server(transport, *port)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        return Ok(());
    }
```

Also update the command name match (~line 1388):

```rust
        Commands::Mcp { .. } => "mcp",
```

**Step 4: Implement HTTP transport in server.rs**

In `raps-cli/src/mcp/server.rs`, update imports (line 16):

```rust
use rmcp::{ServerHandler, ServiceExt, model::*, transport::stdio};
```
→
```rust
use rmcp::{ServerHandler, ServiceExt, model::*, transport::stdio};
#[cfg(feature = "mcp-http")]
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
```

Replace `run_server()` (lines 530-539):

```rust
/// Run the MCP server.
///
/// - `transport = "stdio"`: classic stdin/stdout (default, for local CLI use)
/// - `transport = "http"`:  Streamable HTTP on the given port (for hosted/remote use)
pub async fn run_server(transport: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    match transport {
        "stdio" => {
            let server = RapsServer::new()?;
            let service = server.serve(stdio()).await?;
            service.waiting().await?;
        }
        #[cfg(feature = "mcp-http")]
        "http" => {
            let config = rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default();
            let service = StreamableHttpService::new(
                || Ok(RapsServer::new()?),
                LocalSessionManager::default().into(),
                config,
            );
            let app = axum::Router::new().nest_service("/mcp", service);
            let addr = format!("0.0.0.0:{}", port);
            tracing::info!("MCP HTTP server listening on {}", addr);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, app).await?;
        }
        #[cfg(not(feature = "mcp-http"))]
        "http" => {
            return Err("HTTP transport requires --features mcp-http".into());
        }
        other => {
            return Err(format!("Unknown transport: '{}'. Use 'stdio' or 'http'.", other).into());
        }
    }
    Ok(())
}
```

**Step 5: Verify stdio still works**

Run: `cargo build -p raps-cli 2>&1 | tail -3`
Expected: Compiles with zero errors.

Run: `echo '{}' | timeout 2 ./target/debug/raps mcp 2>&1; echo "exit: $?"`
Expected: Starts and exits cleanly (timeout).

**Step 6: Verify HTTP transport compiles**

Run: `cargo build -p raps-cli --features mcp-http 2>&1 | tail -5`
Expected: Compiles with zero errors.

**Step 7: Test HTTP transport locally**

Run in background: `cargo run -p raps-cli --features mcp-http -- mcp --transport http --port 9999 &`

Test endpoint exists:
```bash
curl -s -X POST http://localhost:9999/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}' 2>&1
```
Expected: JSON response with server info and `Mcp-Session-Id` header.

Kill background process.

**Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock raps-cli/Cargo.toml raps-cli/src/main.rs raps-cli/src/mcp/server.rs
git commit -m "feat: add Streamable HTTP transport to MCP server (--transport http)"
```

---

## Task 2: Docker image for MCP HTTP server

**Files:**
- Create: `deploy/docker/Dockerfile.mcp`
- Modify: `.github/workflows/docker.yml:26` (add mcp to matrix)

**Step 1: Create Dockerfile**

```dockerfile
# deploy/docker/Dockerfile.mcp
FROM rust:1.88-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release -p raps-cli --features mcp-http \
    && strip target/release/raps

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/raps /usr/local/bin/raps
EXPOSE 8080
ENV RUST_LOG=info
CMD ["raps", "mcp", "--transport", "http", "--port", "8080"]
```

**Step 2: Build and test locally**

```bash
docker build -f deploy/docker/Dockerfile.mcp -t raps-mcp:local .
docker run --rm -p 9999:8080 -e APS_CLIENT_ID=test -e APS_CLIENT_SECRET=test raps-mcp:local &
sleep 3
curl -s -X POST http://localhost:9999/mcp \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}'
docker stop $(docker ps -q --filter ancestor=raps-mcp:local)
```

Expected: JSON initialize response.

**Step 3: Add to CI docker workflow**

In `.github/workflows/docker.yml`, add to matrix:

```yaml
          - name: mcp
            dockerfile: deploy/docker/Dockerfile.mcp
```

**Step 4: Commit**

```bash
git add deploy/docker/Dockerfile.mcp .github/workflows/docker.yml
git commit -m "feat: add MCP HTTP server Docker image"
```

---

## Task 3: Customer credentials table in D1

**Files:**
- Create: `raps-marketplace-api/migrations/0002_customer_credentials.sql`
- Modify: `raps-marketplace-api/src/db/schema.sql` (add table)

**Step 1: Create migration**

```sql
-- migrations/0002_customer_credentials.sql
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

**Step 2: Run migration on remote D1**

```bash
cd raps-marketplace-api
npx wrangler d1 execute raps-marketplace --remote --file migrations/0002_customer_credentials.sql
```

**Step 3: Update schema.sql to include the new table**

Append the `CREATE TABLE` statement to `src/db/schema.sql`.

**Step 4: Commit**

```bash
cd raps-marketplace-api
git add migrations/0002_customer_credentials.sql src/db/schema.sql
git commit -m "feat: add customer_credentials table for hosted MCP"
```

---

## Task 4: Credential management API endpoints

**Files:**
- Create: `raps-marketplace-api/src/routes/credentials.ts`
- Create: `raps-marketplace-api/src/lib/encryption.ts`
- Modify: `raps-marketplace-api/src/index.ts` (mount routes)

**Step 1: Create encryption helper**

`src/lib/encryption.ts`:

```typescript
const ALGO = { name: 'AES-GCM', length: 256 }

export async function encrypt(plaintext: string, keyHex: string): Promise<string> {
  const key = await crypto.subtle.importKey(
    'raw', hexToBytes(keyHex), ALGO, false, ['encrypt']
  )
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const ct = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    key,
    new TextEncoder().encode(plaintext)
  )
  // Format: hex(iv) + ":" + hex(ciphertext)
  return bytesToHex(iv) + ':' + bytesToHex(new Uint8Array(ct))
}

export async function decrypt(encoded: string, keyHex: string): Promise<string> {
  const [ivHex, ctHex] = encoded.split(':')
  const key = await crypto.subtle.importKey(
    'raw', hexToBytes(keyHex), ALGO, false, ['decrypt']
  )
  const pt = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: hexToBytes(ivHex) },
    key,
    hexToBytes(ctHex)
  )
  return new TextDecoder().decode(pt)
}

function hexToBytes(hex: string): Uint8Array {
  const arr = new Uint8Array(hex.length / 2)
  for (let i = 0; i < hex.length; i += 2)
    arr[i / 2] = parseInt(hex.slice(i, i + 2), 16)
  return arr
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('')
}
```

**Step 2: Create credential routes**

`src/routes/credentials.ts`:

```typescript
import { Hono } from 'hono'
import { Env } from '../index'
import { LicenseVars, licenseAuth } from '../middleware/licenseAuth'
import { encrypt, decrypt } from '../lib/encryption'

export const credentialRoutes = new Hono<{ Bindings: Env; Variables: LicenseVars }>()

// Store APS credentials
credentialRoutes.post('/', licenseAuth, async (c) => {
  const subId = c.get('subscriptionId')
  const { client_id, client_secret, label, callback_url } = await c.req.json()

  if (!client_id || !client_secret) {
    return c.json({ error: 'client_id and client_secret are required' }, 400)
  }

  // Get customer_id from subscription
  const sub = await c.env.DB.prepare(
    'SELECT customer_id FROM subscriptions WHERE id = ?'
  ).bind(subId).first<{ customer_id: number }>()
  if (!sub) return c.json({ error: 'Subscription not found' }, 404)

  const encrypted = await encrypt(client_secret, c.env.CREDENTIAL_ENCRYPTION_KEY)

  await c.env.DB.prepare(`
    INSERT INTO customer_credentials (customer_id, label, aps_client_id, aps_client_secret_encrypted, aps_callback_url)
    VALUES (?, ?, ?, ?, ?)
    ON CONFLICT(customer_id, label) DO UPDATE SET
      aps_client_id = excluded.aps_client_id,
      aps_client_secret_encrypted = excluded.aps_client_secret_encrypted,
      aps_callback_url = excluded.aps_callback_url
  `).bind(sub.customer_id, label || 'default', client_id, encrypted, callback_url || 'http://localhost:8080/callback').run()

  return c.json({ stored: true, label: label || 'default' })
})

// List credential labels (not secrets)
credentialRoutes.get('/', licenseAuth, async (c) => {
  const subId = c.get('subscriptionId')
  const sub = await c.env.DB.prepare(
    'SELECT customer_id FROM subscriptions WHERE id = ?'
  ).bind(subId).first<{ customer_id: number }>()
  if (!sub) return c.json({ error: 'Subscription not found' }, 404)

  const { results } = await c.env.DB.prepare(
    'SELECT label, aps_client_id, aps_callback_url, created_at FROM customer_credentials WHERE customer_id = ?'
  ).bind(sub.customer_id).all()

  return c.json({ credentials: results })
})

// Delete credentials
credentialRoutes.delete('/:label', licenseAuth, async (c) => {
  const label = c.req.param('label')
  const subId = c.get('subscriptionId')
  const sub = await c.env.DB.prepare(
    'SELECT customer_id FROM subscriptions WHERE id = ?'
  ).bind(subId).first<{ customer_id: number }>()
  if (!sub) return c.json({ error: 'Subscription not found' }, 404)

  await c.env.DB.prepare(
    'DELETE FROM customer_credentials WHERE customer_id = ? AND label = ?'
  ).bind(sub.customer_id, label).run()

  return c.json({ deleted: true })
})
```

**Step 3: Mount routes in index.ts**

Add to the existing Hono app:

```typescript
import { credentialRoutes } from './routes/credentials'
// ...
app.route('/credentials', credentialRoutes)
```

**Step 4: Add CREDENTIAL_ENCRYPTION_KEY secret**

```bash
openssl rand -hex 32  # generate key
npx wrangler secret put CREDENTIAL_ENCRYPTION_KEY
```

**Step 5: Deploy and test**

```bash
npx wrangler deploy
curl -s -X POST https://api.rapscli.xyz/credentials \
  -H "Authorization: Bearer <test-license-key>" \
  -H "Content-Type: application/json" \
  -d '{"client_id":"test123","client_secret":"secret456"}'
```

Expected: `{"stored":true,"label":"default"}`

**Step 6: Commit**

```bash
git add src/routes/credentials.ts src/lib/encryption.ts src/index.ts migrations/
git commit -m "feat: add credential storage API for hosted MCP"
```

---

## Task 5: MCP Gateway Worker + Container config

**Files:**
- Create: `raps-mcp-gateway/` (new Worker project)
  - `raps-mcp-gateway/src/index.ts`
  - `raps-mcp-gateway/wrangler.json`
  - `raps-mcp-gateway/Dockerfile`
  - `raps-mcp-gateway/package.json`
  - `raps-mcp-gateway/tsconfig.json`

**Step 1: Scaffold the project**

```bash
mkdir -p ~/github/raps/raps-mcp-gateway/src
cd ~/github/raps/raps-mcp-gateway
npm init -y
npm install hono
npm install -D wrangler typescript @cloudflare/workers-types
```

**Step 2: Create wrangler.json**

```json
{
  "name": "raps-mcp-gateway",
  "main": "src/index.ts",
  "compatibility_date": "2026-03-01",
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
    "id": "01b40d7a02094803a0df475e529787e6"
  }],
  "routes": [{
    "pattern": "mcp.rapscli.xyz/*",
    "zone_name": "rapscli.xyz"
  }]
}
```

**Step 3: Copy Dockerfile (reference to raps binary image)**

```dockerfile
FROM ghcr.io/dmytro-yemelianov/raps-mcp:latest
EXPOSE 8080
CMD ["raps", "mcp", "--transport", "http", "--port", "8080"]
```

Or build locally from the raps repo Dockerfile.mcp if GHCR image isn't ready yet.

**Step 4: Create gateway Worker**

`src/index.ts`:

```typescript
import { Container, getContainer } from 'cloudflare:container'
import { hashLicenseKey } from './lib'

interface Env {
  MCP_CONTAINER: DurableObjectNamespace
  DB: D1Database
  RATE_LIMIT: KVNamespace
  CREDENTIAL_ENCRYPTION_KEY: string
}

export class McpContainer extends Container {
  defaultPort = 8080
  sleepAfter = '5m'

  override onStart(): void {
    console.log('MCP container started')
  }
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url)

    // Health check
    if (url.pathname === '/health') {
      return new Response('ok')
    }

    // Only handle /mcp path
    if (!url.pathname.startsWith('/mcp')) {
      return new Response('Not Found', { status: 404 })
    }

    // 1. Extract and validate license key
    const auth = request.headers.get('Authorization')
    const licenseKey = auth?.startsWith('Bearer ') ? auth.slice(7).trim() : null
    if (!licenseKey) {
      return new Response(JSON.stringify({ error: 'Missing Authorization: Bearer <license-key>' }), {
        status: 401,
        headers: { 'Content-Type': 'application/json' },
      })
    }

    const keyHash = await hashLicenseKey(licenseKey)
    const license = await env.DB.prepare(`
      SELECT l.id, l.subscription_id, s.customer_id, s.status, s.current_period_end
      FROM licenses l
      JOIN subscriptions s ON s.id = l.subscription_id
      WHERE l.key_hash = ? AND l.revoked = 0
    `).bind(keyHash).first<{
      id: number
      subscription_id: number
      customer_id: number
      status: string
      current_period_end: string
    }>()

    if (!license) {
      return new Response(JSON.stringify({ error: 'Invalid license key' }), {
        status: 401,
        headers: { 'Content-Type': 'application/json' },
      })
    }

    if (license.status !== 'active' || new Date(license.current_period_end) < new Date()) {
      return new Response(JSON.stringify({ error: 'Subscription expired' }), {
        status: 403,
        headers: { 'Content-Type': 'application/json' },
      })
    }

    // 2. Rate limit (100 req/min per license)
    const rateKey = `mcp:${license.id}:${Math.floor(Date.now() / 60000)}`
    const count = parseInt(await env.RATE_LIMIT.get(rateKey) || '0')
    if (count >= 100) {
      return new Response(JSON.stringify({ error: 'Rate limit exceeded' }), {
        status: 429,
        headers: { 'Content-Type': 'application/json' },
      })
    }
    await env.RATE_LIMIT.put(rateKey, String(count + 1), { expirationTtl: 120 })

    // 3. Get APS credentials for this customer
    const creds = await env.DB.prepare(`
      SELECT aps_client_id, aps_client_secret_encrypted, aps_callback_url
      FROM customer_credentials
      WHERE customer_id = ? AND label = 'default'
    `).bind(license.customer_id).first<{
      aps_client_id: string
      aps_client_secret_encrypted: string
      aps_callback_url: string
    }>()

    // 4. Route to per-license container
    const sessionName = `license-${license.id}`
    const container = getContainer(env.MCP_CONTAINER, sessionName)

    // Inject APS credentials if available
    if (creds) {
      const secret = await decryptSecret(creds.aps_client_secret_encrypted, env.CREDENTIAL_ENCRYPTION_KEY)
      container.env = {
        APS_CLIENT_ID: creds.aps_client_id,
        APS_CLIENT_SECRET: secret,
        APS_CALLBACK_URL: creds.aps_callback_url,
      }
    }

    // 5. Proxy request to container (strip the Authorization header)
    const proxyHeaders = new Headers(request.headers)
    proxyHeaders.delete('Authorization')

    const proxyRequest = new Request(request.url, {
      method: request.method,
      headers: proxyHeaders,
      body: request.body,
    })

    return container.fetch(proxyRequest)
  }
}

async function hashLicenseKey(key: string): Promise<string> {
  const data = new TextEncoder().encode(key)
  const buf = await crypto.subtle.digest('SHA-256', data)
  return Array.from(new Uint8Array(buf)).map(b => b.toString(16).padStart(2, '0')).join('')
}

async function decryptSecret(encoded: string, keyHex: string): Promise<string> {
  const [ivHex, ctHex] = encoded.split(':')
  const keyBytes = new Uint8Array(keyHex.length / 2)
  for (let i = 0; i < keyHex.length; i += 2) keyBytes[i / 2] = parseInt(keyHex.slice(i, i + 2), 16)
  const key = await crypto.subtle.importKey('raw', keyBytes, { name: 'AES-GCM' }, false, ['decrypt'])
  const iv = new Uint8Array(ivHex.length / 2)
  for (let i = 0; i < ivHex.length; i += 2) iv[i / 2] = parseInt(ivHex.slice(i, i + 2), 16)
  const ct = new Uint8Array(ctHex.length / 2)
  for (let i = 0; i < ctHex.length; i += 2) ct[i / 2] = parseInt(ctHex.slice(i, i + 2), 16)
  const pt = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, ct)
  return new TextDecoder().decode(pt)
}
```

**Step 5: Deploy**

```bash
npx wrangler secret put CREDENTIAL_ENCRYPTION_KEY  # same key as marketplace API
npx wrangler deploy
```

**Step 6: Commit**

```bash
git init && git add -A
git commit -m "feat: MCP gateway Worker with CF Containers"
```

---

## Task 6: DNS + end-to-end test

**Files:**
- No code changes — infrastructure and manual testing

**Step 1: Verify DNS**

`mcp.rapscli.xyz` should already route to the Worker via the `routes` config in wrangler.json. If not, add a CNAME in Cloudflare DNS dashboard.

**Step 2: Store test credentials**

```bash
curl -s -X POST https://api.rapscli.xyz/credentials \
  -H "Authorization: Bearer <test-license-key>" \
  -H "Content-Type: application/json" \
  -d '{"client_id":"<APS_CLIENT_ID>","client_secret":"<APS_CLIENT_SECRET>"}'
```

**Step 3: End-to-end MCP test**

```bash
# Initialize session
curl -v -X POST https://mcp.rapscli.xyz/mcp \
  -H "Authorization: Bearer <test-license-key>" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"curl-test","version":"0.1"}}}'
```

Expected: JSON with server info, `Mcp-Session-Id` header in response.

```bash
# Call a tool (auth_test)
curl -s -X POST https://mcp.rapscli.xyz/mcp \
  -H "Authorization: Bearer <test-license-key>" \
  -H "Mcp-Session-Id: <session-id-from-above>" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"auth_test","arguments":{}}}'
```

Expected: APS auth test result.

**Step 4: Test from Claude Desktop**

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "raps": {
      "url": "https://mcp.rapscli.xyz/mcp",
      "headers": {
        "Authorization": "Bearer <test-license-key>"
      }
    }
  }
}
```

Restart Claude Desktop. Ask: "List my APS buckets."

---

## Task 7: `raps marketplace credentials` CLI commands

**Files:**
- Create: `raps-cli/src/commands/marketplace/credentials.rs`
- Modify: `raps-cli/src/commands/marketplace/mod.rs` (add subcommand + handlers)

**Step 1: Add CLI subcommands**

In `mod.rs`, add to `MarketplaceCommands` enum:

```rust
    /// Manage APS credentials for hosted MCP
    #[command(subcommand)]
    Credentials(CredentialCommands),
```

Add to the match in `execute()`:

```rust
    MarketplaceCommands::Credentials(cmd) => credentials::execute(cmd, output_format).await,
```

**Step 2: Create credentials.rs**

```rust
use anyhow::Result;
use clap::Subcommand;
use crate::marketplace::{MarketplaceAuth, client::MarketplaceClient};
use crate::output::OutputFormat;

#[derive(Debug, Subcommand)]
pub enum CredentialCommands {
    /// Store APS credentials for hosted MCP
    Set {
        /// APS Client ID
        #[arg(long)]
        client_id: String,
        /// APS Client Secret
        #[arg(long)]
        client_secret: String,
        /// Credential label (default: "default")
        #[arg(long, default_value = "default")]
        label: String,
    },
    /// List stored credential labels
    List,
    /// Delete stored credentials
    Delete {
        /// Label to delete
        label: String,
    },
}

pub async fn execute(cmd: CredentialCommands, output_format: OutputFormat) -> Result<()> {
    let key = MarketplaceAuth::get_license_key()?
        .ok_or_else(|| anyhow::anyhow!("No license key found. Run `raps marketplace license <key>` first."))?;

    let client = MarketplaceClient::new()?;
    match cmd {
        CredentialCommands::Set { client_id, client_secret, label } => {
            client.store_credentials(&key, &client_id, &client_secret, &label).await?;
            output_format.write(&serde_json::json!({ "stored": true, "label": label }))?;
        }
        CredentialCommands::List => {
            let creds = client.list_credentials(&key).await?;
            output_format.write(&creds)?;
        }
        CredentialCommands::Delete { label } => {
            client.delete_credentials(&key, &label).await?;
            output_format.write(&serde_json::json!({ "deleted": true, "label": label }))?;
        }
    }
    Ok(())
}
```

**Step 3: Add HTTP methods to MarketplaceClient**

In `raps-cli/src/marketplace/client.rs`, add:

```rust
    pub async fn store_credentials(&self, license_key: &str, client_id: &str, client_secret: &str, label: &str) -> Result<()> {
        let url = format!("{}/credentials", self.api_base);
        let resp = self.client.post(&url)
            .bearer_auth(license_key)
            .json(&serde_json::json!({
                "client_id": client_id,
                "client_secret": client_secret,
                "label": label,
            }))
            .send().await.context("Failed to store credentials")?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to store credentials (HTTP {})", resp.status());
        }
        Ok(())
    }

    pub async fn list_credentials(&self, license_key: &str) -> Result<serde_json::Value> {
        let url = format!("{}/credentials", self.api_base);
        let resp = self.client.get(&url)
            .bearer_auth(license_key)
            .send().await.context("Failed to list credentials")?;
        resp.json().await.context("Failed to parse credentials response")
    }

    pub async fn delete_credentials(&self, license_key: &str, label: &str) -> Result<()> {
        let url = format!("{}/credentials/{}", self.api_base, label);
        let resp = self.client.delete(&url)
            .bearer_auth(license_key)
            .send().await.context("Failed to delete credentials")?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to delete credentials (HTTP {})", resp.status());
        }
        Ok(())
    }
```

**Step 4: Test**

```bash
raps marketplace credentials set --client-id test123 --client-secret secret456
raps marketplace credentials list
raps marketplace credentials delete default
```

**Step 5: Commit**

```bash
git add raps-cli/src/commands/marketplace/ raps-cli/src/marketplace/client.rs
git commit -m "feat: add marketplace credentials CLI commands"
```

---

## Task 8: Documentation

**Files:**
- Create: `docs/hosted-mcp.md`

**Step 1: Write connection guide**

Cover:
- What is hosted MCP
- Prerequisites (Pro license, APS credentials)
- Storing credentials: `raps marketplace credentials set`
- Connecting Claude Desktop / Cursor / other clients
- Troubleshooting (401, 403, 429 errors)

**Step 2: Commit**

```bash
git add docs/hosted-mcp.md
git commit -m "docs: add hosted MCP connection guide"
```

---

## Execution Order

```
Task 1 ──→ Task 2 ──→ Task 5 ──→ Task 6
                ↗                    ↗
Task 3 ──→ Task 4 ──────────────────
                                    ↘
                              Task 7 ──→ Task 8
```

Tasks 1+2 (Rust transport + Docker) and Tasks 3+4 (DB + API) can run in parallel.
Task 5 (gateway) depends on both tracks.
Task 6 (E2E test) depends on everything.
Tasks 7+8 (CLI + docs) can start once Task 4 is done.

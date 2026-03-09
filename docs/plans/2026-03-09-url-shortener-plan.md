# URL Shortener Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a Cloudflare Worker at `go.rapscli.xyz` that redirects short codes to URLs, with a KV-backed API and inline admin UI.

**Architecture:** Single Hono-based Cloudflare Worker. KV namespace `URL_SHORTENER` stores dynamic links (`code → {url, created_at}`). Hardcoded links are a `const` map in source checked before KV. Auth middleware validates `Authorization: Bearer` on `/api/*` routes.

**Tech Stack:** TypeScript, Hono 4, Cloudflare Workers KV, `@cloudflare/vitest-pool-workers` for tests, wrangler for deploy.

---

### Task 1: Scaffold project

**Files:**
- Create: `workers/url-shortener/wrangler.toml`
- Create: `workers/url-shortener/package.json`
- Create: `workers/url-shortener/tsconfig.json`
- Create: `workers/url-shortener/vitest.config.ts`
- Create: `workers/url-shortener/src/index.ts`

**Step 1: Create wrangler.toml**

```toml
name = "raps-url-shortener"
main = "src/index.ts"
compatibility_date = "2024-12-01"
compatibility_flags = ["nodejs_compat"]

kv_namespaces = [
  { binding = "KV", id = "placeholder-replace-after-create" }
]

routes = [
  { pattern = "go.rapscli.xyz/*", zone_name = "rapscli.xyz" }
]

[vars]
BASE_URL = "https://go.rapscli.xyz"

# Secrets set via: wrangler secret put ADMIN_TOKEN
```

**Step 2: Create package.json**

```json
{
  "name": "raps-url-shortener",
  "private": true,
  "scripts": {
    "dev": "wrangler dev",
    "deploy": "wrangler deploy",
    "test": "vitest run"
  },
  "devDependencies": {
    "@cloudflare/vitest-pool-workers": "^0.12.20",
    "@cloudflare/workers-types": "^4.20251002.0",
    "typescript": "^5.6.3",
    "vitest": "^3.2.4",
    "wrangler": "^4.0.0"
  },
  "dependencies": {
    "hono": "^4.12.5"
  }
}
```

**Step 3: Create tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "moduleResolution": "bundler",
    "lib": ["ES2022"],
    "types": ["@cloudflare/workers-types"],
    "strict": true,
    "noEmit": true
  },
  "include": ["src", "test"]
}
```

**Step 4: Create vitest.config.ts**

```typescript
import { defineWorkersConfig } from '@cloudflare/vitest-pool-workers/config'

export default defineWorkersConfig({
  test: {
    poolOptions: {
      workers: {
        wrangler: { configPath: './wrangler.toml' },
        miniflare: {
          bindings: {
            ADMIN_TOKEN: 'test-token',
            BASE_URL: 'https://go.rapscli.xyz',
          },
          kvNamespaces: ['KV'],
        },
      },
    },
  },
})
```

**Step 5: Create src/index.ts stub**

```typescript
import { Hono } from 'hono'

export type Env = {
  KV: KVNamespace
  ADMIN_TOKEN: string
  BASE_URL: string
}

const app = new Hono<{ Bindings: Env }>()

app.get('/', (c) => c.text('go.rapscli.xyz'))

export default app
```

**Step 6: Install deps**

```bash
cd workers/url-shortener && npm install
```

**Step 7: Verify it type-checks**

```bash
cd workers/url-shortener && npx tsc --noEmit
```

Expected: no errors

**Step 8: Commit**

```bash
git add workers/url-shortener/
git commit -m "feat: scaffold url-shortener Worker project"
```

---

### Task 2: Auth middleware

**Files:**
- Create: `workers/url-shortener/src/auth.ts`
- Create: `workers/url-shortener/test/auth.test.ts`

**Step 1: Write failing test**

```typescript
// workers/url-shortener/test/auth.test.ts
import { describe, it, expect } from 'vitest'
import { env } from 'cloudflare:test'
import app from '../src/index'

describe('auth middleware', () => {
  it('rejects missing token on /api/*', async () => {
    const res = await app.fetch(new Request('http://localhost/api/links'), env)
    expect(res.status).toBe(401)
  })

  it('rejects wrong token on /api/*', async () => {
    const res = await app.fetch(
      new Request('http://localhost/api/links', {
        headers: { Authorization: 'Bearer wrong' },
      }),
      env,
    )
    expect(res.status).toBe(401)
  })

  it('allows correct token on /api/*', async () => {
    const res = await app.fetch(
      new Request('http://localhost/api/links', {
        headers: { Authorization: 'Bearer test-token' },
      }),
      env,
    )
    // 200 or 404 — just not 401
    expect(res.status).not.toBe(401)
  })
})
```

**Step 2: Run to verify it fails**

```bash
cd workers/url-shortener && npm test 2>&1 | grep -E "ok|FAIL|pass|fail" | head -10
```

Expected: tests fail (401 not returned yet)

**Step 3: Create src/auth.ts**

```typescript
import { createMiddleware } from 'hono/factory'
import type { Env } from './index'

export const adminAuth = createMiddleware<{ Bindings: Env }>(async (c, next) => {
  const header = c.req.header('Authorization') ?? ''
  const token = header.startsWith('Bearer ') ? header.slice(7) : ''
  if (!token || token !== c.env.ADMIN_TOKEN) {
    return c.json({ error: 'Unauthorized' }, 401)
  }
  return next()
})
```

**Step 4: Wire auth into index.ts**

```typescript
import { Hono } from 'hono'
import { adminAuth } from './auth'

export type Env = {
  KV: KVNamespace
  ADMIN_TOKEN: string
  BASE_URL: string
}

const app = new Hono<{ Bindings: Env }>()

app.get('/', (c) => c.text('go.rapscli.xyz'))

// Protected API routes (handlers added in later tasks)
app.use('/api/*', adminAuth)
app.get('/api/links', (c) => c.json([]))

export default app
```

**Step 5: Run tests**

```bash
cd workers/url-shortener && npm test 2>&1 | grep -E "✓|×|pass|fail"
```

Expected: all 3 auth tests pass

**Step 6: Commit**

```bash
git add workers/url-shortener/src/auth.ts workers/url-shortener/src/index.ts workers/url-shortener/test/auth.test.ts
git commit -m "feat: add auth middleware for /api/* routes"
```

---

### Task 3: Redirect handler

**Files:**
- Create: `workers/url-shortener/src/redirect.ts`
- Create: `workers/url-shortener/test/redirect.test.ts`

**Step 1: Write failing tests**

```typescript
// workers/url-shortener/test/redirect.test.ts
import { describe, it, expect, beforeEach } from 'vitest'
import { env } from 'cloudflare:test'
import app from '../src/index'

beforeEach(async () => {
  await env.KV.put('abc123', JSON.stringify({ url: 'https://example.com', created_at: '2026-03-09T00:00:00Z' }))
})

describe('redirect', () => {
  it('redirects known code to destination', async () => {
    const res = await app.fetch(new Request('http://localhost/abc123'), env)
    expect(res.status).toBe(301)
    expect(res.headers.get('Location')).toBe('https://example.com')
  })

  it('returns 404 for unknown code', async () => {
    const res = await app.fetch(new Request('http://localhost/unknown'), env)
    expect(res.status).toBe(404)
  })

  it('redirects hardcoded links', async () => {
    const res = await app.fetch(new Request('http://localhost/docs'), env)
    expect(res.status).toBe(301)
    expect(res.headers.get('Location')).toBe('https://rapscli.xyz/docs')
  })
})
```

**Step 2: Run to verify it fails**

```bash
cd workers/url-shortener && npm test 2>&1 | grep -E "redirect" | head -10
```

**Step 3: Create src/redirect.ts**

```typescript
import type { Context } from 'hono'
import type { Env } from './index'

// Hardcoded permanent links — add more here as needed
export const HARDCODED: Record<string, string> = {
  docs: 'https://rapscli.xyz/docs',
  marketplace: 'https://marketplace.rapscli.xyz',
  discord: 'https://discord.gg/placeholder',
}

export async function handleRedirect(c: Context<{ Bindings: Env }>) {
  const code = c.req.param('code')

  // Check hardcoded first
  if (HARDCODED[code]) {
    return c.redirect(HARDCODED[code], 301)
  }

  // Check KV
  const raw = await c.env.KV.get(code)
  if (!raw) {
    return c.html('<h1>404</h1><p>Link not found.</p>', 404)
  }

  const { url } = JSON.parse(raw) as { url: string }
  return c.redirect(url, 301)
}
```

**Step 4: Wire into index.ts**

Add to `src/index.ts`:

```typescript
import { handleRedirect } from './redirect'

// ... existing code ...

app.get('/:code', handleRedirect)
```

**Step 5: Run tests**

```bash
cd workers/url-shortener && npm test 2>&1 | grep -E "✓|×"
```

Expected: all redirect tests pass

**Step 6: Commit**

```bash
git add workers/url-shortener/src/redirect.ts workers/url-shortener/src/index.ts workers/url-shortener/test/redirect.test.ts
git commit -m "feat: add redirect handler with hardcoded + KV lookup"
```

---

### Task 4: API handlers

**Files:**
- Create: `workers/url-shortener/src/api.ts`
- Create: `workers/url-shortener/test/api.test.ts`

**Step 1: Write failing tests**

```typescript
// workers/url-shortener/test/api.test.ts
import { describe, it, expect, beforeEach } from 'vitest'
import { env } from 'cloudflare:test'
import app from '../src/index'

const authHeader = { Authorization: 'Bearer test-token' }

function apiPost(path: string, body: unknown) {
  return app.fetch(new Request(`http://localhost${path}`, {
    method: 'POST',
    headers: { ...authHeader, 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  }), env)
}

describe('POST /api/shorten', () => {
  it('creates a link with custom code', async () => {
    const res = await apiPost('/api/shorten', { url: 'https://example.com', code: 'mycode' })
    expect(res.status).toBe(201)
    const body = await res.json() as { code: string; short_url: string }
    expect(body.code).toBe('mycode')
    expect(body.short_url).toContain('mycode')
  })

  it('creates a link with auto-generated code', async () => {
    const res = await apiPost('/api/shorten', { url: 'https://example.com' })
    expect(res.status).toBe(201)
    const body = await res.json() as { code: string }
    expect(body.code).toMatch(/^[a-z0-9]{6}$/)
  })

  it('returns 409 if code already exists', async () => {
    await apiPost('/api/shorten', { url: 'https://a.com', code: 'taken' })
    const res = await apiPost('/api/shorten', { url: 'https://b.com', code: 'taken' })
    expect(res.status).toBe(409)
  })

  it('returns 400 for invalid URL', async () => {
    const res = await apiPost('/api/shorten', { url: 'not-a-url', code: 'bad' })
    expect(res.status).toBe(400)
  })
})

describe('GET /api/links', () => {
  beforeEach(async () => {
    await env.KV.put('testlink', JSON.stringify({ url: 'https://test.com', created_at: '2026-03-09T00:00:00Z' }))
  })

  it('returns list of links', async () => {
    const res = await app.fetch(
      new Request('http://localhost/api/links', { headers: authHeader }), env
    )
    expect(res.status).toBe(200)
    const body = await res.json() as unknown[]
    expect(Array.isArray(body)).toBe(true)
  })
})

describe('DELETE /api/links/:code', () => {
  beforeEach(async () => {
    await env.KV.put('todelete', JSON.stringify({ url: 'https://del.com', created_at: '2026-03-09T00:00:00Z' }))
  })

  it('deletes a dynamic link', async () => {
    const res = await app.fetch(
      new Request('http://localhost/api/links/todelete', { method: 'DELETE', headers: authHeader }), env
    )
    expect(res.status).toBe(200)
    expect(await env.KV.get('todelete')).toBeNull()
  })

  it('returns 404 for unknown code', async () => {
    const res = await app.fetch(
      new Request('http://localhost/api/links/nosuchcode', { method: 'DELETE', headers: authHeader }), env
    )
    expect(res.status).toBe(404)
  })
})
```

**Step 2: Run to verify it fails**

```bash
cd workers/url-shortener && npm test 2>&1 | grep -E "api" | head -10
```

**Step 3: Create src/api.ts**

```typescript
import { Hono } from 'hono'
import type { Env } from './index'
import { HARDCODED } from './redirect'

export const apiRoutes = new Hono<{ Bindings: Env }>()

function generateCode(): string {
  const chars = 'abcdefghijklmnopqrstuvwxyz0123456789'
  return Array.from(crypto.getRandomValues(new Uint8Array(6)))
    .map(b => chars[b % chars.length])
    .join('')
}

apiRoutes.post('/shorten', async (c) => {
  let body: { url?: string; code?: string }
  try { body = await c.req.json() } catch { return c.json({ error: 'Invalid JSON' }, 400) }

  const { url, code: requestedCode } = body
  if (!url) return c.json({ error: 'Missing url' }, 400)
  try { new URL(url) } catch { return c.json({ error: 'Invalid URL' }, 400) }

  let code = requestedCode?.trim()
  if (code) {
    if (HARDCODED[code]) return c.json({ error: 'Code already exists' }, 409)
    if (await c.env.KV.get(code)) return c.json({ error: 'Code already exists' }, 409)
  } else {
    for (let i = 0; i < 5; i++) {
      const candidate = generateCode()
      if (!HARDCODED[candidate] && !(await c.env.KV.get(candidate))) {
        code = candidate
        break
      }
    }
    if (!code) return c.json({ error: 'Could not generate unique code' }, 500)
  }

  const value = JSON.stringify({ url, created_at: new Date().toISOString() })
  await c.env.KV.put(code, value)

  return c.json({ code, short_url: `${c.env.BASE_URL}/${code}` }, 201)
})

apiRoutes.get('/links', async (c) => {
  const list = await c.env.KV.list()
  const links = await Promise.all(
    list.keys.map(async ({ name }) => {
      const raw = await c.env.KV.get(name)
      const data = raw ? JSON.parse(raw) as { url: string; created_at: string } : { url: '', created_at: '' }
      return { code: name, ...data }
    })
  )
  return c.json(links)
})

apiRoutes.delete('/links/:code', async (c) => {
  const code = c.req.param('code')
  if (!(await c.env.KV.get(code))) return c.json({ error: 'Not found' }, 404)
  await c.env.KV.delete(code)
  return c.json({ ok: true })
})
```

**Step 4: Wire into index.ts**

```typescript
import { Hono } from 'hono'
import { adminAuth } from './auth'
import { handleRedirect } from './redirect'
import { apiRoutes } from './api'

export type Env = {
  KV: KVNamespace
  ADMIN_TOKEN: string
  BASE_URL: string
}

const app = new Hono<{ Bindings: Env }>()

app.use('/api/*', adminAuth)
app.route('/api', apiRoutes)

app.get('/:code', handleRedirect)

export default app
```

**Step 5: Run tests**

```bash
cd workers/url-shortener && npm test 2>&1 | grep -E "✓|×|pass|fail"
```

Expected: all tests pass

**Step 6: Commit**

```bash
git add workers/url-shortener/src/api.ts workers/url-shortener/src/index.ts workers/url-shortener/test/api.test.ts
git commit -m "feat: add API handlers (shorten, list, delete)"
```

---

### Task 5: Admin UI

**Files:**
- Create: `workers/url-shortener/src/admin.ts`
- Modify: `workers/url-shortener/src/index.ts`

**Step 1: Write failing test**

```typescript
// add to workers/url-shortener/test/auth.test.ts
describe('admin UI', () => {
  it('serves HTML at /admin', async () => {
    const res = await app.fetch(new Request('http://localhost/admin'), env)
    expect(res.status).toBe(200)
    expect(res.headers.get('content-type')).toContain('text/html')
  })
})
```

**Step 2: Run to verify it fails**

```bash
cd workers/url-shortener && npm test 2>&1 | grep "admin UI" | head -5
```

**Step 3: Create src/admin.ts**

```typescript
export function adminHtml(): string {
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>go.rapscli.xyz — Admin</title>
  <style>
    body { font-family: system-ui, sans-serif; max-width: 700px; margin: 40px auto; padding: 0 16px; }
    h1 { font-size: 1.4rem; }
    input, button { padding: 6px 10px; font-size: 14px; }
    input { border: 1px solid #ccc; border-radius: 4px; }
    button { background: #0070f3; color: white; border: none; border-radius: 4px; cursor: pointer; }
    button.del { background: #e00; }
    table { width: 100%; border-collapse: collapse; margin-top: 16px; }
    th, td { text-align: left; padding: 6px 8px; border-bottom: 1px solid #eee; font-size: 14px; }
    .tag { font-size: 11px; background: #eee; padding: 1px 5px; border-radius: 3px; }
    #status { margin-top: 8px; font-size: 13px; color: #555; }
  </style>
</head>
<body>
  <h1>go.rapscli.xyz</h1>

  <div>
    <label>Token: <input id="token" type="password" placeholder="admin token" style="width:220px"></label>
    <button onclick="loadLinks()">Load</button>
  </div>

  <hr>

  <h2 style="font-size:1rem">Create link</h2>
  <div style="display:flex;gap:8px;flex-wrap:wrap">
    <input id="url" type="url" placeholder="https://example.com" style="width:280px">
    <input id="code" placeholder="custom code (optional)" style="width:160px">
    <button onclick="createLink()">Shorten</button>
  </div>
  <div id="status"></div>

  <table id="links-table">
    <thead><tr><th>Code</th><th>URL</th><th>Created</th><th></th></tr></thead>
    <tbody id="links-body"></tbody>
  </table>

  <script>
    function token() { return document.getElementById('token').value || localStorage.getItem('admin_token') || '' }
    function saveToken() { localStorage.setItem('admin_token', document.getElementById('token').value) }
    function status(msg) { document.getElementById('status').textContent = msg }

    window.onload = () => {
      const saved = localStorage.getItem('admin_token')
      if (saved) document.getElementById('token').value = saved
    }

    async function loadLinks() {
      saveToken()
      const res = await fetch('/api/links', { headers: { Authorization: 'Bearer ' + token() } })
      if (!res.ok) { status('Error: ' + res.status); return }
      const links = await res.json()
      const tbody = document.getElementById('links-body')
      tbody.innerHTML = links.map(l =>
        \`<tr>
          <td><a href="/\${l.code}">\${l.code}</a></td>
          <td style="max-width:280px;overflow:hidden;text-overflow:ellipsis">\${l.url}</td>
          <td>\${l.created_at ? l.created_at.slice(0,10) : ''}</td>
          <td><button class="del" onclick="deleteLink('\${l.code}')">×</button></td>
        </tr>\`
      ).join('')
      status(links.length + ' links loaded')
    }

    async function createLink() {
      saveToken()
      const url = document.getElementById('url').value
      const code = document.getElementById('code').value
      if (!url) { status('URL required'); return }
      const body = { url }
      if (code) body.code = code
      const res = await fetch('/api/shorten', {
        method: 'POST',
        headers: { Authorization: 'Bearer ' + token(), 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
      })
      const data = await res.json()
      if (!res.ok) { status('Error: ' + (data.error || res.status)); return }
      status('Created: ' + data.short_url)
      document.getElementById('url').value = ''
      document.getElementById('code').value = ''
      loadLinks()
    }

    async function deleteLink(code) {
      if (!confirm('Delete ' + code + '?')) return
      const res = await fetch('/api/links/' + code, {
        method: 'DELETE', headers: { Authorization: 'Bearer ' + token() }
      })
      if (!res.ok) { status('Error deleting ' + code); return }
      status('Deleted ' + code)
      loadLinks()
    }
  </script>
</body>
</html>`
}
```

**Step 4: Add /admin route to index.ts**

Add to `src/index.ts`:

```typescript
import { adminHtml } from './admin'

// before /:code route:
app.get('/admin', (c) => c.html(adminHtml()))
```

**Step 5: Run tests**

```bash
cd workers/url-shortener && npm test 2>&1 | grep -E "✓|×|pass|fail"
```

Expected: all tests pass

**Step 6: Commit**

```bash
git add workers/url-shortener/src/admin.ts workers/url-shortener/src/index.ts
git commit -m "feat: add inline admin UI at /admin"
```

---

### Task 6: CI workflow + deploy instructions

**Files:**
- Create: `.github/workflows/deploy-url-shortener.yml`

**Step 1: Create workflow**

```yaml
name: Deploy url-shortener

on:
  push:
    branches: [main]
    paths: ['workers/url-shortener/**']
  workflow_dispatch:

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: workers/url-shortener/package-lock.json
      - run: npm ci
        working-directory: workers/url-shortener
      - run: npm test
        working-directory: workers/url-shortener
      - run: npx wrangler deploy
        working-directory: workers/url-shortener
        env:
          CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
          CLOUDFLARE_ACCOUNT_ID: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
```

**Step 2: First-time setup instructions**

After implementation, run once locally to create the KV namespace and set the token:

```bash
cd workers/url-shortener

# Create KV namespace (copy the id into wrangler.toml)
npx wrangler kv namespace create URL_SHORTENER

# Set the admin token secret
npx wrangler secret put ADMIN_TOKEN

# Deploy
npx wrangler deploy
```

**Step 3: Run all tests one final time**

```bash
cd workers/url-shortener && npm test 2>&1 | grep -E "Test Files|Tests "
```

Expected: all test files pass, 0 failed

**Step 4: Commit**

```bash
git add .github/workflows/deploy-url-shortener.yml workers/url-shortener/
git commit -m "ci: add deploy workflow for url-shortener"
```

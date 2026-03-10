# RAPS Pro Plugin Marketplace — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the RAPS Pro Plugin Marketplace — a Cloudflare Worker API, two Cloudflare Pages frontends (storefront + admin), and the RAPS CLI integration that lets paying customers install signed pro plugin binaries via `raps marketplace install <name>`.

**Architecture:** Cloudflare Worker (Hono v4) backed by D1 (SQLite), R2 (plugin binaries), and KV (license token cache). Stripe handles payments. The CLI stores license keys in the system keyring and does a 7-day periodic validation against the Worker. Two separate Cloudflare Pages sites serve the public storefront (Astro) and admin dashboard (React + Vite).

**Tech Stack:** TypeScript, Hono v4, Stripe SDK, `jose` (JWT), Web Crypto API, Vitest + `@cloudflare/vitest-pool-workers`; Astro (storefront); React + Vite (admin); Rust + `reqwest` + `keyring` + `ed25519-dalek` (CLI).

---

## Context

The `raps` repo already contains stub marketplace command files at `raps-cli/src/commands/marketplace/` that reference two unimplemented modules:
- `crate::marketplace::*` (in `raps-cli`)
- `raps_kernel::marketplace::*` (in `raps-kernel`)

This plan wires those stubs to the real API. No new clap structs or command files are needed — only the implementation modules.

The Worker API lives at `marketplace.rapscli.xyz`. The storefront at `buy.rapscli.xyz`. The admin at `admin.rapscli.xyz`.

---

## PART 1: `raps-marketplace-api` (new private repo)

---

### Task 1: Scaffold the Worker project

**Files:**
- Create: `raps-marketplace-api/` (new repo root)
- Create: `raps-marketplace-api/package.json`
- Create: `raps-marketplace-api/wrangler.toml`
- Create: `raps-marketplace-api/tsconfig.json`
- Create: `raps-marketplace-api/src/index.ts`
- Create: `raps-marketplace-api/vitest.config.ts`

**Step 1: Init the project**

```bash
mkdir raps-marketplace-api && cd raps-marketplace-api
npm init -y
npm install hono @hono/zod-validator zod stripe jose
npm install -D wrangler typescript @cloudflare/workers-types \
  vitest @cloudflare/vitest-pool-workers
```

**Step 2: Write `wrangler.toml`**

```toml
name = "raps-marketplace-api"
main = "src/index.ts"
compatibility_date = "2024-12-01"
compatibility_flags = ["nodejs_compat"]

[[d1_databases]]
binding = "DB"
database_name = "raps-marketplace"
database_id = "REPLACE_AFTER_CREATE"

[[r2_buckets]]
binding = "PLUGINS"
bucket_name = "raps-plugins"

[[kv_namespaces]]
binding = "LICENSE_CACHE"
id = "REPLACE_AFTER_CREATE"

[vars]
MARKETPLACE_URL = "https://marketplace.rapscli.xyz"
STOREFRONT_URL = "https://buy.rapscli.xyz"

# Secrets (set via: wrangler secret put <NAME>)
# STRIPE_SECRET_KEY
# STRIPE_WEBHOOK_SECRET
# ADMIN_PASSWORD_HASH   (PBKDF2 hex of admin password)
# ADMIN_JWT_SECRET      (32-byte random hex)
# ED25519_PUBLIC_KEY    (hex-encoded, for verifying upload signatures)
```

**Step 3: Write `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022"],
    "module": "ES2022",
    "moduleResolution": "bundler",
    "types": ["@cloudflare/workers-types"],
    "strict": true,
    "noUnusedLocals": true,
    "noImplicitReturns": true
  },
  "include": ["src/**/*", "test/**/*"]
}
```

**Step 4: Write `src/index.ts` (router skeleton)**

```typescript
import { Hono } from 'hono'
import { cors } from 'hono/cors'
import { publicRoutes } from './routes/public'
import { licenseRoutes } from './routes/license'
import { adminRoutes } from './routes/admin'

export type Env = {
  DB: D1Database
  PLUGINS: R2Bucket
  LICENSE_CACHE: KVNamespace
  STRIPE_SECRET_KEY: string
  STRIPE_WEBHOOK_SECRET: string
  ADMIN_PASSWORD_HASH: string
  ADMIN_JWT_SECRET: string
  ED25519_PUBLIC_KEY: string
  MARKETPLACE_URL: string
  STOREFRONT_URL: string
}

const app = new Hono<{ Bindings: Env }>()

app.use('*', cors({
  origin: (origin) => origin, // tightened per-route in admin
  allowMethods: ['GET', 'POST', 'DELETE'],
  allowHeaders: ['Content-Type', 'Authorization'],
}))

app.route('/plugins', publicRoutes)
app.route('/license', licenseRoutes)
app.route('/admin', adminRoutes)
app.post('/webhooks/stripe', (c) => import('./routes/webhook').then(m => m.handleWebhook(c)))

export default app
```

**Step 5: Write `vitest.config.ts`**

```typescript
import { defineConfig } from 'vitest/config'
import { defineWorkersConfig } from '@cloudflare/vitest-pool-workers/config'

export default defineWorkersConfig({
  test: {
    poolOptions: {
      workers: {
        wrangler: { configPath: './wrangler.toml' },
      },
    },
  },
})
```

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: scaffold raps-marketplace-api Worker project"
```

---

### Task 2: D1 schema migrations

**Files:**
- Create: `src/db/schema.sql`
- Create: `src/db/migrate.ts`

**Step 1: Write `src/db/schema.sql`**

```sql
CREATE TABLE IF NOT EXISTS customers (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  email TEXT NOT NULL UNIQUE,
  stripe_customer_id TEXT UNIQUE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS plugins (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  slug TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  description TEXT NOT NULL,
  price_monthly_cents INTEGER NOT NULL,
  price_yearly_cents INTEGER NOT NULL,
  stripe_price_id_monthly TEXT NOT NULL,
  stripe_price_id_yearly TEXT NOT NULL,
  latest_version TEXT NOT NULL DEFAULT '0.0.0',
  published INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS subscriptions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  customer_id INTEGER NOT NULL REFERENCES customers(id),
  plugin_id INTEGER NOT NULL REFERENCES plugins(id),
  stripe_subscription_id TEXT NOT NULL UNIQUE,
  seat_count INTEGER NOT NULL DEFAULT 1,
  status TEXT NOT NULL DEFAULT 'active',
  current_period_end DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS licenses (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  subscription_id INTEGER NOT NULL REFERENCES subscriptions(id),
  key_hash TEXT NOT NULL UNIQUE,
  created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  last_validated_at DATETIME,
  revoked INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS plugin_releases (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  plugin_id INTEGER NOT NULL REFERENCES plugins(id),
  version TEXT NOT NULL,
  platform TEXT NOT NULL,
  r2_key TEXT NOT NULL UNIQUE,
  sha256 TEXT NOT NULL,
  ed25519_signature TEXT NOT NULL,
  published_at DATETIME DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(plugin_id, version, platform)
);
```

**Step 2: Run migration locally**

```bash
npx wrangler d1 create raps-marketplace
# Copy the database_id into wrangler.toml
npx wrangler d1 execute raps-marketplace --local --file=src/db/schema.sql
```

**Step 3: Write `src/db/migrate.ts` (runtime helper)**

```typescript
export async function runMigrations(db: D1Database): Promise<void> {
  // Used in tests only — production uses wrangler d1 execute
  await db.exec(`
    CREATE TABLE IF NOT EXISTS customers (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      email TEXT NOT NULL UNIQUE,
      stripe_customer_id TEXT UNIQUE,
      created_at DATETIME DEFAULT CURRENT_TIMESTAMP
    );
    -- (paste full schema here)
  `)
}
```

**Step 4: Commit**

```bash
git add src/db/
git commit -m "feat: add D1 schema migrations"
```

---

### Task 3: Shared helpers — crypto, license keys

**Files:**
- Create: `src/lib/crypto.ts`
- Create: `src/lib/license.ts`
- Create: `test/lib/crypto.test.ts`

**Step 1: Write `src/lib/crypto.ts`**

```typescript
/** SHA-256 hash a string, return hex */
export async function sha256Hex(input: string): Promise<string> {
  const data = new TextEncoder().encode(input)
  const buf = await crypto.subtle.digest('SHA-256', data)
  return Array.from(new Uint8Array(buf))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('')
}

/** PBKDF2-hash a password for admin storage (100k iterations, SHA-256) */
export async function hashPassword(password: string, salt: string): Promise<string> {
  const keyMaterial = await crypto.subtle.importKey(
    'raw', new TextEncoder().encode(password), 'PBKDF2', false, ['deriveBits']
  )
  const bits = await crypto.subtle.deriveBits(
    { name: 'PBKDF2', salt: hexToBytes(salt), iterations: 100_000, hash: 'SHA-256' },
    keyMaterial, 256
  )
  return Array.from(new Uint8Array(bits)).map(b => b.toString(16).padStart(2, '0')).join('')
}

export function hexToBytes(hex: string): Uint8Array {
  const arr = new Uint8Array(hex.length / 2)
  for (let i = 0; i < hex.length; i += 2) arr[i / 2] = parseInt(hex.slice(i, i + 2), 16)
  return arr
}

/** Verify Ed25519 signature of uploaded plugin binary */
export async function verifyEd25519(
  data: ArrayBuffer,
  signatureHex: string,
  publicKeyHex: string,
): Promise<boolean> {
  try {
    const key = await crypto.subtle.importKey(
      'raw', hexToBytes(publicKeyHex), { name: 'Ed25519' }, false, ['verify']
    )
    return await crypto.subtle.verify('Ed25519', key, hexToBytes(signatureHex), data)
  } catch {
    return false
  }
}

/** Constant-time string compare to prevent timing attacks */
export function safeCompare(a: string, b: string): boolean {
  if (a.length !== b.length) return false
  let diff = 0
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i)
  return diff === 0
}
```

**Step 2: Write `src/lib/license.ts`**

```typescript
import { sha256Hex } from './crypto'

/** Generate a cryptographically random 32-byte license key (64 hex chars) */
export function generateLicenseKey(): string {
  const bytes = new Uint8Array(32)
  crypto.getRandomValues(bytes)
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('')
}

/** Hash a license key for storage (keys are high-entropy, SHA-256 is sufficient) */
export async function hashLicenseKey(key: string): Promise<string> {
  return sha256Hex(key)
}
```

**Step 3: Write `test/lib/crypto.test.ts`**

```typescript
import { describe, it, expect } from 'vitest'
import { sha256Hex, safeCompare } from '../../src/lib/crypto'

describe('sha256Hex', () => {
  it('produces consistent output', async () => {
    const h = await sha256Hex('hello')
    expect(h).toHaveLength(64)
    expect(await sha256Hex('hello')).toBe(h)
  })
})

describe('safeCompare', () => {
  it('returns true for identical strings', () => {
    expect(safeCompare('abc', 'abc')).toBe(true)
  })
  it('returns false for different strings of same length', () => {
    expect(safeCompare('abc', 'xyz')).toBe(false)
  })
  it('returns false for different lengths', () => {
    expect(safeCompare('abc', 'abcd')).toBe(false)
  })
})
```

**Step 4: Run tests**

```bash
npx vitest run test/lib/crypto.test.ts
```
Expected: 4 tests pass.

**Step 5: Commit**

```bash
git add src/lib/ test/lib/
git commit -m "feat: add crypto and license key helpers"
```

---

### Task 4: Public routes — plugin listing + Stripe checkout

**Files:**
- Create: `src/routes/public.ts`
- Create: `test/routes/public.test.ts`

**Step 1: Write `src/routes/public.ts`**

```typescript
import { Hono } from 'hono'
import { Env } from '../index'
import { getStripe } from '../lib/stripe'

export const publicRoutes = new Hono<{ Bindings: Env }>()

publicRoutes.get('/', async (c) => {
  const { results } = await c.env.DB.prepare(
    'SELECT slug, name, description, price_monthly_cents, price_yearly_cents, latest_version FROM plugins WHERE published = 1'
  ).all()
  return c.json(results)
})

publicRoutes.get('/:slug', async (c) => {
  const slug = c.req.param('slug')
  const plugin = await c.env.DB.prepare(
    'SELECT * FROM plugins WHERE slug = ? AND published = 1'
  ).bind(slug).first()
  if (!plugin) return c.json({ error: 'Not found' }, 404)
  return c.json(plugin)
})

publicRoutes.post('/checkout', async (c) => {
  const { slug, billing } = await c.req.json<{ slug: string; billing: 'monthly' | 'yearly' }>()
  const plugin = await c.env.DB.prepare(
    'SELECT * FROM plugins WHERE slug = ? AND published = 1'
  ).bind(slug).first<{ id: number; stripe_price_id_monthly: string; stripe_price_id_yearly: string; name: string }>()

  if (!plugin) return c.json({ error: 'Plugin not found' }, 404)

  const stripe = getStripe(c.env.STRIPE_SECRET_KEY)
  const priceId = billing === 'yearly' ? plugin.stripe_price_id_yearly : plugin.stripe_price_id_monthly

  const session = await stripe.checkout.sessions.create({
    mode: 'subscription',
    payment_method_types: ['card'],
    line_items: [{ price: priceId, quantity: 1 }],
    success_url: `${c.env.STOREFRONT_URL}/success?session_id={CHECKOUT_SESSION_ID}`,
    cancel_url: `${c.env.STOREFRONT_URL}/plugins/${slug}`,
    metadata: { plugin_slug: slug },
  })

  return c.json({ url: session.url })
})
```

**Step 2: Write `src/lib/stripe.ts`**

```typescript
import Stripe from 'stripe'

let _stripe: Stripe | null = null

export function getStripe(secretKey: string): Stripe {
  if (!_stripe) {
    _stripe = new Stripe(secretKey, { apiVersion: '2024-12-18.acacia', httpClient: Stripe.createFetchHttpClient() })
  }
  return _stripe
}
```

**Step 3: Write minimal public route test**

```typescript
import { describe, it, expect } from 'vitest'
import { env } from 'cloudflare:test'
import app from '../../src/index'

describe('GET /plugins', () => {
  it('returns empty array when no plugins published', async () => {
    const res = await app.fetch(new Request('http://localhost/plugins'), env)
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(Array.isArray(body)).toBe(true)
  })
})
```

**Step 4: Run tests**

```bash
npx vitest run test/routes/public.test.ts
```

**Step 5: Commit**

```bash
git add src/routes/public.ts src/lib/stripe.ts test/routes/
git commit -m "feat: add public plugin listing and Stripe checkout endpoint"
```

---

### Task 5: Stripe webhook handler

**Files:**
- Create: `src/routes/webhook.ts`
- Create: `test/routes/webhook.test.ts`

**Step 1: Write `src/routes/webhook.ts`**

```typescript
import { Context } from 'hono'
import { Env } from '../index'
import { getStripe } from '../lib/stripe'
import { generateLicenseKey, hashLicenseKey } from '../lib/license'

export async function handleWebhook(c: Context<{ Bindings: Env }>) {
  const sig = c.req.header('stripe-signature')
  if (!sig) return c.json({ error: 'Missing signature' }, 400)

  const body = await c.req.text()
  const stripe = getStripe(c.env.STRIPE_SECRET_KEY)

  let event: import('stripe').Stripe.Event
  try {
    event = await stripe.webhooks.constructEventAsync(body, sig, c.env.STRIPE_WEBHOOK_SECRET)
  } catch {
    return c.json({ error: 'Invalid signature' }, 400)
  }

  switch (event.type) {
    case 'checkout.session.completed':
      await handleCheckoutCompleted(c.env.DB, event.data.object as import('stripe').Stripe.Checkout.Session)
      break
    case 'customer.subscription.updated':
    case 'customer.subscription.deleted':
      await handleSubscriptionChange(c.env.DB, event.data.object as import('stripe').Stripe.Subscription)
      break
  }

  return c.json({ received: true })
}

async function handleCheckoutCompleted(db: D1Database, session: import('stripe').Stripe.Checkout.Session) {
  const email = session.customer_details?.email
  const stripeCustomerId = session.customer as string
  const stripeSubId = session.subscription as string
  const pluginSlug = session.metadata?.plugin_slug

  if (!email || !pluginSlug) return

  // Upsert customer
  await db.prepare(
    'INSERT INTO customers (email, stripe_customer_id) VALUES (?, ?) ON CONFLICT(email) DO UPDATE SET stripe_customer_id = excluded.stripe_customer_id'
  ).bind(email, stripeCustomerId).run()

  const customer = await db.prepare('SELECT id FROM customers WHERE email = ?').bind(email).first<{ id: number }>()
  const plugin = await db.prepare('SELECT id FROM plugins WHERE slug = ?').bind(pluginSlug).first<{ id: number }>()

  if (!customer || !plugin) return

  // Insert subscription
  const periodEnd = new Date((session as any).current_period_end * 1000).toISOString()
  await db.prepare(
    'INSERT OR IGNORE INTO subscriptions (customer_id, plugin_id, stripe_subscription_id, status, current_period_end) VALUES (?, ?, ?, "active", ?)'
  ).bind(customer.id, plugin.id, stripeSubId, periodEnd).run()

  const sub = await db.prepare('SELECT id FROM subscriptions WHERE stripe_subscription_id = ?').bind(stripeSubId).first<{ id: number }>()
  if (!sub) return

  // Generate and store license key
  const key = generateLicenseKey()
  const keyHash = await hashLicenseKey(key)
  await db.prepare('INSERT INTO licenses (subscription_id, key_hash) VALUES (?, ?)').bind(sub.id, keyHash).run()

  // TODO: email the license key to `email` via Resend / Cloudflare Email Workers
  // The plaintext `key` should be emailed once and never stored
  console.log(`License key for ${email}: ${key}`) // remove after email integration
}

async function handleSubscriptionChange(db: D1Database, sub: import('stripe').Stripe.Subscription) {
  const status = sub.status === 'active' ? 'active' : sub.status === 'canceled' ? 'canceled' : 'past_due'
  const periodEnd = new Date(sub.current_period_end * 1000).toISOString()
  await db.prepare(
    'UPDATE subscriptions SET status = ?, current_period_end = ? WHERE stripe_subscription_id = ?'
  ).bind(status, periodEnd, sub.id).run()
}
```

**Step 2: Commit**

```bash
git add src/routes/webhook.ts
git commit -m "feat: add Stripe webhook handler (checkout, subscription lifecycle)"
```

---

### Task 6: License validation endpoint

**Files:**
- Create: `src/routes/license.ts`
- Create: `src/middleware/licenseAuth.ts`
- Create: `test/routes/license.test.ts`

**Step 1: Write `src/middleware/licenseAuth.ts`**

```typescript
import { createMiddleware } from 'hono/factory'
import { Env } from '../index'
import { hashLicenseKey } from '../lib/license'

export type LicenseVars = { licenseId: number; subscriptionId: number }

export const licenseAuth = createMiddleware<{ Bindings: Env; Variables: LicenseVars }>(async (c, next) => {
  const auth = c.req.header('Authorization')
  const key = auth?.startsWith('Bearer ') ? auth.slice(7) : null
  if (!key) return c.json({ error: 'Missing license key' }, 401)

  const keyHash = await hashLicenseKey(key)
  const license = await c.env.DB.prepare(`
    SELECT l.id, l.subscription_id, s.status, s.current_period_end
    FROM licenses l
    JOIN subscriptions s ON s.id = l.subscription_id
    WHERE l.key_hash = ? AND l.revoked = 0
  `).bind(keyHash).first<{ id: number; subscription_id: number; status: string; current_period_end: string }>()

  if (!license) return c.json({ error: 'Invalid license key' }, 401)
  if (license.status !== 'active') return c.json({ error: 'Subscription inactive' }, 403)
  if (new Date(license.current_period_end) < new Date()) return c.json({ error: 'Subscription expired' }, 403)

  c.set('licenseId', license.id)
  c.set('subscriptionId', license.subscription_id)
  await next()
})
```

**Step 2: Write `src/routes/license.ts`**

```typescript
import { Hono } from 'hono'
import { Env } from '../index'
import { licenseAuth, LicenseVars } from '../middleware/licenseAuth'

export const licenseRoutes = new Hono<{ Bindings: Env; Variables: LicenseVars }>()

licenseRoutes.post('/validate', licenseAuth, async (c) => {
  const licenseId = c.get('licenseId')
  const subId = c.get('subscriptionId')

  // Update last_validated_at
  await c.env.DB.prepare('UPDATE licenses SET last_validated_at = CURRENT_TIMESTAMP WHERE id = ?')
    .bind(licenseId).run()

  // Get entitlements (plugin slugs this subscription covers)
  const { results } = await c.env.DB.prepare(`
    SELECT p.slug FROM plugins p
    JOIN subscriptions s ON s.plugin_id = p.id
    WHERE s.id = ?
  `).bind(subId).all<{ slug: string }>()

  const validUntil = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString()

  return c.json({
    valid: true,
    plugins: results.map(r => r.slug),
    valid_until: validUntil,
  })
})
```

**Step 3: Write `test/routes/license.test.ts`**

```typescript
import { describe, it, expect, beforeAll } from 'vitest'
import { env } from 'cloudflare:test'
import app from '../../src/index'
import { runMigrations } from '../../src/db/migrate'
import { hashLicenseKey } from '../../src/lib/license'

beforeAll(async () => {
  await runMigrations(env.DB)
})

describe('POST /license/validate', () => {
  it('returns 401 for missing key', async () => {
    const res = await app.fetch(new Request('http://localhost/license/validate', { method: 'POST' }), env)
    expect(res.status).toBe(401)
  })

  it('returns 401 for unknown key', async () => {
    const res = await app.fetch(new Request('http://localhost/license/validate', {
      method: 'POST',
      headers: { Authorization: 'Bearer deadbeefdeadbeef' },
    }), env)
    expect(res.status).toBe(401)
  })
})
```

**Step 4: Run tests**

```bash
npx vitest run test/routes/license.test.ts
```

**Step 5: Commit**

```bash
git add src/routes/license.ts src/middleware/licenseAuth.ts test/routes/license.test.ts
git commit -m "feat: add license validation endpoint with 7-day valid_until"
```

---

### Task 7: Plugin download endpoint

**Files:**
- Modify: `src/routes/license.ts`

**Step 1: Add download route to `src/routes/license.ts`**

```typescript
import { detectPlatform } from '../lib/platform'

licenseRoutes.get('/plugins/:slug/download', licenseAuth, async (c) => {
  const slug = c.req.param('slug')
  const platform = c.req.query('platform') ?? detectPlatform(c.req.header('user-agent') ?? '')
  const subId = c.get('subscriptionId')

  // Check entitlement
  const entitled = await c.env.DB.prepare(`
    SELECT 1 FROM subscriptions s JOIN plugins p ON p.id = s.plugin_id
    WHERE s.id = ? AND p.slug = ?
  `).bind(subId, slug).first()

  if (!entitled) return c.json({ error: 'Not entitled to this plugin' }, 403)

  const release = await c.env.DB.prepare(`
    SELECT r2_key, sha256, ed25519_signature FROM plugin_releases pr
    JOIN plugins p ON p.id = pr.plugin_id
    WHERE p.slug = ? AND pr.platform = ?
    ORDER BY pr.published_at DESC LIMIT 1
  `).bind(slug, platform).first<{ r2_key: string; sha256: string; ed25519_signature: string }>()

  if (!release) return c.json({ error: `No release for platform ${platform}` }, 404)

  const object = await c.env.PLUGINS.get(release.r2_key)
  if (!object) return c.json({ error: 'Binary not found in storage' }, 500)

  return new Response(object.body, {
    headers: {
      'Content-Type': 'application/octet-stream',
      'Content-Disposition': `attachment; filename="raps-${slug}"`,
      'X-SHA256': release.sha256,
      'X-Ed25519-Signature': release.ed25519_signature,
    },
  })
})
```

**Step 2: Write `src/lib/platform.ts`**

```typescript
export function detectPlatform(userAgent: string): string {
  if (userAgent.includes('Windows')) return 'win-x64'
  if (userAgent.includes('Darwin') || userAgent.includes('Mac')) return 'darwin-arm64'
  return 'linux-x64'
}
```

**Step 3: Commit**

```bash
git add src/routes/license.ts src/lib/platform.ts
git commit -m "feat: add plugin download endpoint with entitlement check and R2 streaming"
```

---

### Task 8: Admin auth — login + JWT middleware

**Files:**
- Create: `src/routes/admin.ts`
- Create: `src/middleware/adminAuth.ts`
- Create: `test/routes/admin.test.ts`

**Step 1: Write `src/middleware/adminAuth.ts`**

```typescript
import { createMiddleware } from 'hono/factory'
import { SignJWT, jwtVerify } from 'jose'
import { Env } from '../index'

export const adminAuth = createMiddleware<{ Bindings: Env }>(async (c, next) => {
  const cookie = c.req.header('Cookie') ?? ''
  const token = cookie.split(';').find(s => s.trim().startsWith('admin_token='))?.split('=')[1]
  if (!token) return c.json({ error: 'Unauthorized' }, 401)

  try {
    const secret = new TextEncoder().encode(c.env.ADMIN_JWT_SECRET)
    await jwtVerify(token, secret, { audience: 'raps-admin' })
  } catch {
    return c.json({ error: 'Invalid or expired token' }, 401)
  }

  await next()
})

export async function issueAdminJwt(secret: string): Promise<string> {
  const key = new TextEncoder().encode(secret)
  return new SignJWT({ role: 'admin' })
    .setProtectedHeader({ alg: 'HS256' })
    .setAudience('raps-admin')
    .setExpirationTime('1h')
    .sign(key)
}
```

**Step 2: Write `src/routes/admin.ts` (login + stub mounts)**

```typescript
import { Hono } from 'hono'
import { Env } from '../index'
import { adminAuth, issueAdminJwt } from '../middleware/adminAuth'
import { hashPassword } from '../lib/crypto'
import { adminCustomerRoutes } from './admin/customers'
import { adminPluginRoutes } from './admin/plugins'
import { adminMetricsRoutes } from './admin/metrics'

export const adminRoutes = new Hono<{ Bindings: Env }>()

adminRoutes.post('/login', async (c) => {
  const { password } = await c.req.json<{ password: string }>()
  if (!password) return c.json({ error: 'Missing password' }, 400)

  // ADMIN_PASSWORD_HASH = "<salt>:<hash>" stored as secret
  const [salt, storedHash] = c.env.ADMIN_PASSWORD_HASH.split(':')
  const inputHash = await hashPassword(password, salt)

  if (inputHash !== storedHash) return c.json({ error: 'Invalid password' }, 401)

  const token = await issueAdminJwt(c.env.ADMIN_JWT_SECRET)

  return new Response(JSON.stringify({ ok: true }), {
    headers: {
      'Content-Type': 'application/json',
      'Set-Cookie': `admin_token=${token}; HttpOnly; Secure; SameSite=Strict; Path=/admin; Max-Age=3600`,
    },
  })
})

adminRoutes.use('/*', adminAuth)
adminRoutes.route('/customers', adminCustomerRoutes)
adminRoutes.route('/plugins', adminPluginRoutes)
adminRoutes.route('/metrics', adminMetricsRoutes)
```

**Step 3: Write login test**

```typescript
describe('POST /admin/login', () => {
  it('rejects wrong password', async () => {
    const res = await app.fetch(new Request('http://localhost/admin/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ password: 'wrong' }),
    }), env)
    expect(res.status).toBe(401)
  })
})
```

**Step 4: Commit**

```bash
git add src/routes/admin.ts src/middleware/adminAuth.ts test/routes/admin.test.ts
git commit -m "feat: add admin login with PBKDF2 password check and 1h JWT cookie"
```

---

### Task 9: Admin — customer + license endpoints

**Files:**
- Create: `src/routes/admin/customers.ts`

**Step 1: Write `src/routes/admin/customers.ts`**

```typescript
import { Hono } from 'hono'
import { Env } from '../../index'

export const adminCustomerRoutes = new Hono<{ Bindings: Env }>()

adminCustomerRoutes.get('/', async (c) => {
  const page = parseInt(c.req.query('page') ?? '1')
  const limit = 25
  const offset = (page - 1) * limit

  const { results } = await c.env.DB.prepare(`
    SELECT c.id, c.email, c.created_at,
           s.status, s.seat_count, s.current_period_end,
           p.slug as plugin_slug
    FROM customers c
    LEFT JOIN subscriptions s ON s.customer_id = c.id
    LEFT JOIN plugins p ON p.id = s.plugin_id
    ORDER BY c.created_at DESC
    LIMIT ? OFFSET ?
  `).bind(limit, offset).all()

  return c.json({ customers: results, page, limit })
})

adminCustomerRoutes.get('/:id', async (c) => {
  const id = c.req.param('id')
  const customer = await c.env.DB.prepare(
    'SELECT * FROM customers WHERE id = ?'
  ).bind(id).first()
  if (!customer) return c.json({ error: 'Not found' }, 404)

  const { results: subscriptions } = await c.env.DB.prepare(`
    SELECT s.*, p.slug, p.name,
           l.id as license_id, l.key_hash, l.revoked, l.last_validated_at
    FROM subscriptions s
    JOIN plugins p ON p.id = s.plugin_id
    LEFT JOIN licenses l ON l.subscription_id = s.id
    WHERE s.customer_id = ?
  `).bind(id).all()

  return c.json({ customer, subscriptions })
})

adminCustomerRoutes.post('/licenses/:licenseId/revoke', async (c) => {
  const id = c.req.param('licenseId')
  await c.env.DB.prepare('UPDATE licenses SET revoked = 1 WHERE id = ?').bind(id).run()
  return c.json({ revoked: true })
})
```

**Step 2: Commit**

```bash
git add src/routes/admin/customers.ts
git commit -m "feat: add admin customer list, detail, and license revocation endpoints"
```

---

### Task 10: Admin — plugin management + release upload

**Files:**
- Create: `src/routes/admin/plugins.ts`
- Create: `src/routes/admin/metrics.ts`

**Step 1: Write `src/routes/admin/plugins.ts`**

```typescript
import { Hono } from 'hono'
import { Env } from '../../index'
import { verifyEd25519 } from '../../lib/crypto'

export const adminPluginRoutes = new Hono<{ Bindings: Env }>()

adminPluginRoutes.get('/', async (c) => {
  const { results } = await c.env.DB.prepare('SELECT * FROM plugins ORDER BY name').all()
  return c.json(results)
})

adminPluginRoutes.post('/:slug/publish', async (c) => {
  const slug = c.req.param('slug')
  const { published } = await c.req.json<{ published: boolean }>()
  await c.env.DB.prepare('UPDATE plugins SET published = ? WHERE slug = ?')
    .bind(published ? 1 : 0, slug).run()
  return c.json({ ok: true })
})

// Upload a new signed binary release
// Body: multipart/form-data with fields: version, platform, signature, binary (file)
adminPluginRoutes.post('/:slug/releases', async (c) => {
  const slug = c.req.param('slug')
  const plugin = await c.env.DB.prepare('SELECT id FROM plugins WHERE slug = ?').bind(slug).first<{ id: number }>()
  if (!plugin) return c.json({ error: 'Plugin not found' }, 404)

  const form = await c.req.formData()
  const version = form.get('version') as string
  const platform = form.get('platform') as string
  const signatureHex = form.get('signature') as string
  const binary = form.get('binary') as File

  if (!version || !platform || !signatureHex || !binary) {
    return c.json({ error: 'Missing required fields: version, platform, signature, binary' }, 400)
  }

  const data = await binary.arrayBuffer()

  // Verify Ed25519 signature against the hardcoded public key
  const valid = await verifyEd25519(data, signatureHex, c.env.ED25519_PUBLIC_KEY)
  if (!valid) return c.json({ error: 'Ed25519 signature verification failed' }, 400)

  // Compute SHA-256 of binary
  const hashBuf = await crypto.subtle.digest('SHA-256', data)
  const sha256 = Array.from(new Uint8Array(hashBuf)).map(b => b.toString(16).padStart(2, '0')).join('')

  // Upload to R2
  const r2Key = `plugins/${slug}/v${version}/${platform}/raps-${slug}`
  await c.env.PLUGINS.put(r2Key, data, {
    httpMetadata: { contentType: 'application/octet-stream' },
    customMetadata: { sha256, signature: signatureHex },
  })

  // Insert release record
  await c.env.DB.prepare(`
    INSERT INTO plugin_releases (plugin_id, version, platform, r2_key, sha256, ed25519_signature)
    VALUES (?, ?, ?, ?, ?, ?)
    ON CONFLICT(plugin_id, version, platform) DO UPDATE SET
      r2_key = excluded.r2_key, sha256 = excluded.sha256, ed25519_signature = excluded.ed25519_signature
  `).bind(plugin.id, version, platform, r2Key, sha256, signatureHex).run()

  // Update latest_version if this is newer
  await c.env.DB.prepare(`
    UPDATE plugins SET latest_version = ?
    WHERE id = ? AND (latest_version = '0.0.0' OR latest_version < ?)
  `).bind(version, plugin.id, version).run()

  return c.json({ ok: true, r2_key: r2Key, sha256 })
})

adminPluginRoutes.get('/:slug/releases', async (c) => {
  const slug = c.req.param('slug')
  const { results } = await c.env.DB.prepare(`
    SELECT pr.* FROM plugin_releases pr
    JOIN plugins p ON p.id = pr.plugin_id
    WHERE p.slug = ?
    ORDER BY pr.published_at DESC
  `).bind(slug).all()
  return c.json(results)
})
```

**Step 2: Write `src/routes/admin/metrics.ts`**

```typescript
import { Hono } from 'hono'
import { Env } from '../../index'

export const adminMetricsRoutes = new Hono<{ Bindings: Env }>()

adminMetricsRoutes.get('/', async (c) => {
  const [activeCount, totalSeats, canceledCount, recentSignups] = await Promise.all([
    c.env.DB.prepare("SELECT COUNT(*) as n FROM subscriptions WHERE status = 'active'").first<{ n: number }>(),
    c.env.DB.prepare("SELECT COALESCE(SUM(seat_count), 0) as n FROM subscriptions WHERE status = 'active'").first<{ n: number }>(),
    c.env.DB.prepare("SELECT COUNT(*) as n FROM subscriptions WHERE status = 'canceled'").first<{ n: number }>(),
    c.env.DB.prepare("SELECT COUNT(*) as n FROM customers WHERE created_at > datetime('now', '-30 days')").first<{ n: number }>(),
  ])

  // MRR: sum of monthly prices for active subs (simplified — does not account for annual billing periods)
  const mrr = await c.env.DB.prepare(`
    SELECT COALESCE(SUM(p.price_monthly_cents * s.seat_count), 0) as cents
    FROM subscriptions s JOIN plugins p ON p.id = s.plugin_id WHERE s.status = 'active'
  `).first<{ cents: number }>()

  return c.json({
    mrr_cents: mrr?.cents ?? 0,
    active_subscriptions: activeCount?.n ?? 0,
    total_seats: totalSeats?.n ?? 0,
    canceled_subscriptions: canceledCount?.n ?? 0,
    new_customers_30d: recentSignups?.n ?? 0,
  })
})
```

**Step 3: Commit**

```bash
git add src/routes/admin/plugins.ts src/routes/admin/metrics.ts
git commit -m "feat: add admin plugin management, release upload, and metrics endpoints"
```

---

### Task 11: Rate limiting + deploy

**Files:**
- Modify: `src/index.ts`
- Modify: `wrangler.toml`

**Step 1: Add rate limiting middleware**

Add to `src/index.ts` before route mounts:

```typescript
import { rateLimiter } from 'hono/rate-limiter' // or implement via KV

// Simple KV-based rate limiter for sensitive endpoints
app.use('/license/validate', kvRateLimit(10, '1m'))  // 10 req/min per IP
app.use('/admin/login', kvRateLimit(5, '5m'))         // 5 req/5min per IP
```

Write `src/middleware/kvRateLimit.ts`:

```typescript
import { createMiddleware } from 'hono/factory'
import { Env } from '../index'

export function kvRateLimit(max: number, windowSeconds: number) {
  return createMiddleware<{ Bindings: Env }>(async (c, next) => {
    const ip = c.req.header('CF-Connecting-IP') ?? 'unknown'
    const key = `rl:${c.req.path}:${ip}`
    const current = parseInt(await c.env.LICENSE_CACHE.get(key) ?? '0')

    if (current >= max) {
      return c.json({ error: 'Too many requests' }, 429)
    }

    await c.env.LICENSE_CACHE.put(key, String(current + 1), { expirationTtl: windowSeconds })
    await next()
  })
}
```

**Step 2: Deploy**

```bash
# Create KV namespace
npx wrangler kv namespace create LICENSE_CACHE
# Copy id into wrangler.toml

# Set secrets
npx wrangler secret put STRIPE_SECRET_KEY
npx wrangler secret put STRIPE_WEBHOOK_SECRET
npx wrangler secret put ADMIN_JWT_SECRET
# For ADMIN_PASSWORD_HASH: generate salt+hash locally, then:
npx wrangler secret put ADMIN_PASSWORD_HASH
npx wrangler secret put ED25519_PUBLIC_KEY

# Deploy
npx wrangler deploy
```

**Step 3: Commit**

```bash
git add src/middleware/kvRateLimit.ts src/index.ts wrangler.toml
git commit -m "feat: add KV-based rate limiting and deploy Worker"
```

---

## PART 2: `raps-marketplace-frontend` (new private repo)

---

### Task 12: Scaffold storefront (Astro)

**Files:**
- Create: `raps-marketplace-frontend/storefront/` (Astro project)

**Step 1: Init Astro storefront**

```bash
mkdir raps-marketplace-frontend && cd raps-marketplace-frontend
npm create astro@latest storefront -- --template minimal --typescript strict --no-git
cd storefront && npm install
```

**Step 2: Write `storefront/astro.config.mjs`**

```javascript
import { defineConfig } from 'astro/config'
import tailwind from '@astrojs/tailwind'

export default defineConfig({
  integrations: [tailwind()],
  output: 'static',
})
```

**Step 3: Write `storefront/src/env.d.ts`**

```typescript
declare const API_BASE: string  // injected at build time
```

**Step 4: Write `storefront/wrangler.jsonc`** (for Pages deployment)

```jsonc
{
  "name": "raps-marketplace-storefront",
  "pages_build_output_dir": "dist",
  "compatibility_date": "2024-12-01"
}
```

**Step 5: Commit**

```bash
git add storefront/
git commit -m "feat: scaffold Astro storefront"
```

---

### Task 13: Plugin catalog + detail pages

**Files:**
- Create: `storefront/src/pages/index.astro`
- Create: `storefront/src/pages/plugins/[slug].astro`
- Create: `storefront/src/components/PluginCard.astro`
- Create: `storefront/src/lib/api.ts`

**Step 1: Write `storefront/src/lib/api.ts`**

```typescript
const API = import.meta.env.PUBLIC_API_BASE ?? 'https://marketplace.rapscli.xyz'

export type Plugin = {
  slug: string; name: string; description: string;
  price_monthly_cents: number; price_yearly_cents: number; latest_version: string;
}

export async function getPlugins(): Promise<Plugin[]> {
  const res = await fetch(`${API}/plugins`)
  if (!res.ok) return []
  return res.json()
}

export async function getPlugin(slug: string): Promise<Plugin | null> {
  const res = await fetch(`${API}/plugins/${slug}`)
  if (!res.ok) return null
  return res.json()
}

export async function createCheckoutSession(slug: string, billing: 'monthly' | 'yearly'): Promise<string | null> {
  const res = await fetch(`${API}/checkout`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ slug, billing }),
  })
  if (!res.ok) return null
  const { url } = await res.json()
  return url
}
```

**Step 2: Write `storefront/src/pages/index.astro`**

```astro
---
import { getPlugins } from '../lib/api'
import PluginCard from '../components/PluginCard.astro'

const plugins = await getPlugins()
---
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <title>RAPS Pro Plugins</title>
  <meta name="description" content="Pro plugins for the RAPS CLI" />
</head>
<body class="bg-zinc-950 text-zinc-100 min-h-screen">
  <main class="max-w-4xl mx-auto px-6 py-16">
    <h1 class="text-3xl font-bold mb-2">RAPS Pro Plugins</h1>
    <p class="text-zinc-400 mb-10">Extend your RAPS CLI with professional-grade integrations.</p>
    <div class="grid gap-6 sm:grid-cols-2">
      {plugins.map(p => <PluginCard plugin={p} />)}
    </div>
  </main>
</body>
</html>
```

**Step 3: Write `storefront/src/components/PluginCard.astro`**

```astro
---
import type { Plugin } from '../lib/api'
const { plugin } = Astro.props as { plugin: Plugin }
const monthly = (plugin.price_monthly_cents / 100).toFixed(2)
const yearly = (plugin.price_yearly_cents / 100).toFixed(2)
---
<a href={`/plugins/${plugin.slug}`}
   class="block border border-zinc-800 rounded-xl p-6 hover:border-zinc-600 transition-colors">
  <div class="flex items-start justify-between mb-3">
    <h2 class="font-semibold text-lg">{plugin.name}</h2>
    <span class="text-xs bg-violet-900 text-violet-200 px-2 py-0.5 rounded font-mono">PRO</span>
  </div>
  <p class="text-zinc-400 text-sm mb-4 line-clamp-2">{plugin.description}</p>
  <div class="text-sm text-zinc-300">
    <span class="font-medium">${monthly}/mo</span>
    <span class="text-zinc-500 ml-2">or ${yearly}/yr</span>
  </div>
</a>
```

**Step 4: Write `storefront/src/pages/plugins/[slug].astro`**

```astro
---
import { getPlugins, getPlugin } from '../../lib/api'

export async function getStaticPaths() {
  const plugins = await getPlugins()
  return plugins.map(p => ({ params: { slug: p.slug } }))
}

const { slug } = Astro.params
const plugin = await getPlugin(slug!)
if (!plugin) return Astro.redirect('/404')

const monthly = (plugin.price_monthly_cents / 100).toFixed(2)
const yearly = (plugin.price_yearly_cents / 100).toFixed(2)
---
<html lang="en">
<head><meta charset="UTF-8" /><title>{plugin.name} — RAPS Pro</title></head>
<body class="bg-zinc-950 text-zinc-100 min-h-screen">
  <main class="max-w-2xl mx-auto px-6 py-16">
    <a href="/" class="text-zinc-500 text-sm mb-8 block hover:text-zinc-300">← All plugins</a>
    <h1 class="text-3xl font-bold mb-2">{plugin.name}</h1>
    <p class="text-zinc-400 mb-8">{plugin.description}</p>
    <div class="flex gap-4">
      <button data-slug={slug} data-billing="monthly"
        class="buy-btn bg-violet-600 hover:bg-violet-500 text-white px-6 py-3 rounded-lg font-medium">
        ${monthly} / month
      </button>
      <button data-slug={slug} data-billing="yearly"
        class="buy-btn border border-violet-600 text-violet-300 hover:bg-violet-900/30 px-6 py-3 rounded-lg font-medium">
        ${yearly} / year <span class="text-xs opacity-70 ml-1">save {Math.round((1 - plugin.price_yearly_cents / (plugin.price_monthly_cents * 12)) * 100)}%</span>
      </button>
    </div>
  </main>
  <script>
    const API = 'https://marketplace.rapscli.xyz'
    document.querySelectorAll('.buy-btn').forEach(btn => {
      btn.addEventListener('click', async () => {
        const slug = btn.getAttribute('data-slug')!
        const billing = btn.getAttribute('data-billing') as 'monthly' | 'yearly'
        const res = await fetch(`${API}/checkout`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ slug, billing }),
        })
        const { url } = await res.json()
        if (url) window.location.href = url
      })
    })
  </script>
</body>
</html>
```

**Step 5: Commit**

```bash
git add storefront/src/
git commit -m "feat: add plugin catalog and detail pages"
```

---

### Task 14: Success page

**Files:**
- Create: `storefront/src/pages/success.astro`

**Step 1: Write `storefront/src/pages/success.astro`**

```astro
---
// session_id is in query params but we don't need to verify it client-side
// The license key was emailed to the customer
---
<html lang="en">
<head><meta charset="UTF-8" /><title>Purchase successful — RAPS Pro</title></head>
<body class="bg-zinc-950 text-zinc-100 min-h-screen flex items-center justify-center">
  <div class="max-w-lg text-center px-6">
    <div class="text-5xl mb-6">✓</div>
    <h1 class="text-2xl font-bold mb-3">You're all set</h1>
    <p class="text-zinc-400 mb-8">
      Your license key has been sent to your email. Check your inbox (and spam folder).
    </p>
    <div class="bg-zinc-900 border border-zinc-800 rounded-xl p-6 text-left mb-8">
      <p class="text-sm text-zinc-400 mb-4 font-medium">Install your plugin:</p>
      <pre class="text-sm font-mono text-violet-300 whitespace-pre-wrap"><code># 1. Register your license key
raps marketplace license &lt;your-key&gt;

# 2. Install the plugin
raps marketplace install &lt;plugin-name&gt;</code></pre>
    </div>
    <a href="/" class="text-zinc-500 text-sm hover:text-zinc-300">← Back to plugins</a>
  </div>
</body>
</html>
```

**Step 2: Deploy storefront to Cloudflare Pages**

```bash
cd storefront && npm run build
npx wrangler pages deploy dist --project-name raps-marketplace-storefront
# Set custom domain buy.rapscli.xyz in Cloudflare Pages dashboard
```

**Step 3: Commit**

```bash
git add storefront/src/pages/success.astro
git commit -m "feat: add purchase success page with install instructions"
```

---

### Task 15: Admin dashboard scaffold (React + Vite)

**Files:**
- Create: `raps-marketplace-frontend/admin/` (React + Vite SPA)

**Step 1: Scaffold**

```bash
cd raps-marketplace-frontend
npm create vite@latest admin -- --template react-ts
cd admin && npm install
npm install react-router-dom @tanstack/react-query axios
npm install -D tailwindcss postcss autoprefixer
npx tailwindcss init -p
```

**Step 2: Write `admin/src/lib/api.ts`**

```typescript
const API = import.meta.env.VITE_API_BASE ?? 'https://marketplace.rapscli.xyz'

async function apiFetch(path: string, init?: RequestInit) {
  const res = await fetch(`${API}${path}`, { credentials: 'include', ...init })
  if (res.status === 401) { window.location.href = '/login'; throw new Error('Unauthorized') }
  if (!res.ok) throw new Error(`API error ${res.status}`)
  return res.json()
}

export const api = {
  login: (password: string) => fetch(`${API}/admin/login`, {
    method: 'POST', credentials: 'include',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ password }),
  }),
  metrics: () => apiFetch('/admin/metrics'),
  customers: (page = 1) => apiFetch(`/admin/customers?page=${page}`),
  customer: (id: string) => apiFetch(`/admin/customers/${id}`),
  revokelicense: (id: string) => apiFetch(`/admin/customers/licenses/${id}/revoke`, { method: 'POST' }),
  plugins: () => apiFetch('/admin/plugins'),
  publishPlugin: (slug: string, published: boolean) =>
    apiFetch(`/admin/plugins/${slug}/publish`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ published }) }),
  uploadRelease: (slug: string, form: FormData) =>
    apiFetch(`/admin/plugins/${slug}/releases`, { method: 'POST', body: form }),
  releases: (slug: string) => apiFetch(`/admin/plugins/${slug}/releases`),
}
```

**Step 3: Commit scaffold**

```bash
git add admin/
git commit -m "feat: scaffold admin React + Vite SPA"
```

---

### Task 16: Admin pages — login, dashboard, customers, plugins

**Files:**
- Create: `admin/src/pages/Login.tsx`
- Create: `admin/src/pages/Dashboard.tsx`
- Create: `admin/src/pages/Customers.tsx`
- Create: `admin/src/pages/Plugins.tsx`
- Modify: `admin/src/App.tsx`

**Step 1: Write `admin/src/App.tsx`**

```tsx
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import Login from './pages/Login'
import Dashboard from './pages/Dashboard'
import Customers from './pages/Customers'
import Plugins from './pages/Plugins'

const queryClient = new QueryClient()

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          <Route path="/login" element={<Login />} />
          <Route path="/" element={<Dashboard />} />
          <Route path="/customers" element={<Customers />} />
          <Route path="/plugins" element={<Plugins />} />
          <Route path="*" element={<Navigate to="/" />} />
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  )
}
```

**Step 2: Write `admin/src/pages/Login.tsx`**

```tsx
import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { api } from '../lib/api'

export default function Login() {
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const nav = useNavigate()

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setError('')
    const res = await api.login(password)
    if (res.ok) nav('/')
    else setError('Invalid password')
  }

  return (
    <div className="min-h-screen bg-zinc-950 flex items-center justify-center">
      <form onSubmit={handleSubmit} className="bg-zinc-900 border border-zinc-800 rounded-xl p-8 w-full max-w-sm">
        <h1 className="text-xl font-bold text-white mb-6">RAPS Admin</h1>
        {error && <p className="text-red-400 text-sm mb-4">{error}</p>}
        <input
          type="password" value={password} onChange={e => setPassword(e.target.value)}
          placeholder="Password" required
          className="w-full bg-zinc-800 text-white rounded-lg px-4 py-2 mb-4 outline-none focus:ring-2 focus:ring-violet-500"
        />
        <button type="submit" className="w-full bg-violet-600 hover:bg-violet-500 text-white py-2 rounded-lg font-medium">
          Log in
        </button>
      </form>
    </div>
  )
}
```

**Step 3: Write `admin/src/pages/Dashboard.tsx`**

```tsx
import { useQuery } from '@tanstack/react-query'
import { api } from '../lib/api'
import Nav from '../components/Nav'

export default function Dashboard() {
  const { data } = useQuery({ queryKey: ['metrics'], queryFn: api.metrics })

  return (
    <div className="min-h-screen bg-zinc-950 text-white">
      <Nav />
      <main className="max-w-5xl mx-auto px-6 py-10">
        <h1 className="text-2xl font-bold mb-8">Dashboard</h1>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          {[
            { label: 'MRR', value: data ? `$${(data.mrr_cents / 100).toFixed(0)}` : '—' },
            { label: 'Active subs', value: data?.active_subscriptions ?? '—' },
            { label: 'Total seats', value: data?.total_seats ?? '—' },
            { label: 'New customers (30d)', value: data?.new_customers_30d ?? '—' },
          ].map(({ label, value }) => (
            <div key={label} className="bg-zinc-900 border border-zinc-800 rounded-xl p-5">
              <p className="text-zinc-400 text-sm mb-1">{label}</p>
              <p className="text-2xl font-bold">{String(value)}</p>
            </div>
          ))}
        </div>
      </main>
    </div>
  )
}
```

**Step 4: Write `admin/src/pages/Customers.tsx` (abbreviated — full pagination + detail)**

```tsx
import { useQuery } from '@tanstack/react-query'
import { api } from '../lib/api'
import Nav from '../components/Nav'

export default function Customers() {
  const { data } = useQuery({ queryKey: ['customers'], queryFn: () => api.customers() })

  return (
    <div className="min-h-screen bg-zinc-950 text-white">
      <Nav />
      <main className="max-w-5xl mx-auto px-6 py-10">
        <h1 className="text-2xl font-bold mb-8">Customers</h1>
        <div className="border border-zinc-800 rounded-xl overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-zinc-900 text-zinc-400">
              <tr>
                {['Email', 'Plugin', 'Status', 'Seats', 'Period end'].map(h => (
                  <th key={h} className="px-4 py-3 text-left font-medium">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {data?.customers.map((c: any) => (
                <tr key={c.id} className="border-t border-zinc-800 hover:bg-zinc-900/50">
                  <td className="px-4 py-3">{c.email}</td>
                  <td className="px-4 py-3 font-mono text-xs">{c.plugin_slug ?? '—'}</td>
                  <td className="px-4 py-3">
                    <span className={`px-2 py-0.5 rounded text-xs ${c.status === 'active' ? 'bg-green-900 text-green-300' : 'bg-red-900 text-red-300'}`}>
                      {c.status ?? '—'}
                    </span>
                  </td>
                  <td className="px-4 py-3">{c.seat_count ?? '—'}</td>
                  <td className="px-4 py-3 text-zinc-400">{c.current_period_end ? new Date(c.current_period_end).toLocaleDateString() : '—'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </main>
    </div>
  )
}
```

**Step 5: Write `admin/src/pages/Plugins.tsx`** (with release upload)

```tsx
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { api } from '../lib/api'
import Nav from '../components/Nav'

export default function Plugins() {
  const { data, refetch } = useQuery({ queryKey: ['plugins'], queryFn: api.plugins })
  const [uploadSlug, setUploadSlug] = useState('')
  const [uploadForm, setUploadForm] = useState({ version: '', platform: 'linux-x64', signature: '' })
  const [uploadFile, setUploadFile] = useState<File | null>(null)
  const [uploadStatus, setUploadStatus] = useState('')

  async function handleUpload(e: React.FormEvent) {
    e.preventDefault()
    if (!uploadFile) return
    const form = new FormData()
    form.append('version', uploadForm.version)
    form.append('platform', uploadForm.platform)
    form.append('signature', uploadForm.signature)
    form.append('binary', uploadFile)
    try {
      await api.uploadRelease(uploadSlug, form)
      setUploadStatus('Uploaded successfully')
      refetch()
    } catch {
      setUploadStatus('Upload failed')
    }
  }

  return (
    <div className="min-h-screen bg-zinc-950 text-white">
      <Nav />
      <main className="max-w-5xl mx-auto px-6 py-10">
        <h1 className="text-2xl font-bold mb-8">Plugins</h1>
        <table className="w-full text-sm border border-zinc-800 rounded-xl overflow-hidden mb-12">
          <thead className="bg-zinc-900 text-zinc-400">
            <tr>{['Slug', 'Name', 'Version', 'Published', ''].map(h => <th key={h} className="px-4 py-3 text-left">{h}</th>)}</tr>
          </thead>
          <tbody>
            {data?.map((p: any) => (
              <tr key={p.id} className="border-t border-zinc-800">
                <td className="px-4 py-3 font-mono text-xs">{p.slug}</td>
                <td className="px-4 py-3">{p.name}</td>
                <td className="px-4 py-3 text-zinc-400">{p.latest_version}</td>
                <td className="px-4 py-3">
                  <span className={p.published ? 'text-green-400' : 'text-zinc-500'}>{p.published ? 'Yes' : 'No'}</span>
                </td>
                <td className="px-4 py-3">
                  <button onClick={() => api.publishPlugin(p.slug, !p.published).then(() => refetch())}
                    className="text-xs text-violet-400 hover:text-violet-300">
                    {p.published ? 'Unpublish' : 'Publish'}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        <h2 className="text-lg font-semibold mb-4">Upload Release</h2>
        <form onSubmit={handleUpload} className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 grid gap-3 max-w-md">
          {[
            { label: 'Plugin slug', key: 'slug', state: uploadSlug, setter: setUploadSlug },
          ].map(({ label, key, state, setter }) => (
            <div key={key}>
              <label className="text-xs text-zinc-400 mb-1 block">{label}</label>
              <input value={state} onChange={e => setter(e.target.value)} required
                className="w-full bg-zinc-800 text-white rounded px-3 py-2 text-sm" />
            </div>
          ))}
          {[
            { label: 'Version', key: 'version' },
            { label: 'Ed25519 signature (hex)', key: 'signature' },
          ].map(({ label, key }) => (
            <div key={key}>
              <label className="text-xs text-zinc-400 mb-1 block">{label}</label>
              <input value={(uploadForm as any)[key]} onChange={e => setUploadForm(f => ({ ...f, [key]: e.target.value }))} required
                className="w-full bg-zinc-800 text-white rounded px-3 py-2 text-sm" />
            </div>
          ))}
          <div>
            <label className="text-xs text-zinc-400 mb-1 block">Platform</label>
            <select value={uploadForm.platform} onChange={e => setUploadForm(f => ({ ...f, platform: e.target.value }))}
              className="w-full bg-zinc-800 text-white rounded px-3 py-2 text-sm">
              {['linux-x64', 'darwin-arm64', 'win-x64'].map(p => <option key={p}>{p}</option>)}
            </select>
          </div>
          <div>
            <label className="text-xs text-zinc-400 mb-1 block">Binary</label>
            <input type="file" onChange={e => setUploadFile(e.target.files?.[0] ?? null)} required className="text-sm text-zinc-300" />
          </div>
          <button type="submit" className="bg-violet-600 hover:bg-violet-500 text-white py-2 rounded font-medium text-sm">
            Upload release
          </button>
          {uploadStatus && <p className="text-sm text-zinc-400">{uploadStatus}</p>}
        </form>
      </main>
    </div>
  )
}
```

**Step 6: Write `admin/src/components/Nav.tsx`**

```tsx
import { Link } from 'react-router-dom'

export default function Nav() {
  return (
    <nav className="border-b border-zinc-800 bg-zinc-900">
      <div className="max-w-5xl mx-auto px-6 py-3 flex items-center gap-6">
        <span className="font-bold text-white">RAPS Admin</span>
        {[['/', 'Dashboard'], ['/customers', 'Customers'], ['/plugins', 'Plugins']].map(([to, label]) => (
          <Link key={to} to={to} className="text-sm text-zinc-400 hover:text-white">{label}</Link>
        ))}
      </div>
    </nav>
  )
}
```

**Step 7: Deploy admin**

```bash
cd admin && npm run build
npx wrangler pages deploy dist --project-name raps-marketplace-admin
# Set custom domain admin.rapscli.xyz in Cloudflare Pages dashboard
```

**Step 8: Commit**

```bash
git add admin/src/
git commit -m "feat: add admin dashboard pages (metrics, customers, plugins, upload)"
```

---

## PART 3: RAPS CLI (existing `raps` repo)

---

### Task 17: Add shared types to `raps-kernel`

**Files:**
- Create: `raps-kernel/src/marketplace.rs`
- Modify: `raps-kernel/src/lib.rs`

**Step 1: Write `raps-kernel/src/marketplace.rs`**

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Shared marketplace types used by raps-cli marketplace commands.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginTier {
    Free,
    Pro,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub price_monthly_cents: u32,
    pub price_yearly_cents: u32,
    pub latest_version: String,
    #[serde(default)]
    pub tier: PluginTierField,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PluginTierField(pub PluginTier);

impl Default for PluginTier {
    fn default() -> Self { PluginTier::Pro }
}

impl PartialEq<PluginTier> for PluginTierField {
    fn eq(&self, other: &PluginTier) -> bool { &self.0 == other }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub plugins: Vec<String>,
    pub valid_until: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installation {
    pub name: String,
    pub version: String,
    pub slug: String,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub yanked: bool,
    pub changelog: Option<String>,
    pub raps_compatibility: Option<String>,
}
```

**Step 2: Add to `raps-kernel/src/lib.rs`**

Add the line:
```rust
pub mod marketplace;
```

**Step 3: Write test in `raps-kernel/src/marketplace.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_tier_default_is_pro() {
        assert_eq!(PluginTier::default(), PluginTier::Pro);
    }

    #[test]
    fn validate_response_deserializes() {
        let json = r#"{"valid":true,"plugins":["acc-bulk"],"valid_until":"2026-03-15T00:00:00Z"}"#;
        let r: ValidateResponse = serde_json::from_str(json).unwrap();
        assert!(r.valid);
        assert_eq!(r.plugins, vec!["acc-bulk"]);
    }
}
```

**Step 4: Run tests**

```bash
cd raps-kernel && cargo test marketplace
```
Expected: 2 tests pass.

**Step 5: Commit**

```bash
git add raps-kernel/src/marketplace.rs raps-kernel/src/lib.rs
git commit -m "feat: add marketplace types to raps-kernel"
```

---

### Task 18: Create `raps-cli/src/marketplace/` module

**Files:**
- Create: `raps-cli/src/marketplace/mod.rs`
- Create: `raps-cli/src/marketplace/auth.rs`
- Create: `raps-cli/src/marketplace/client.rs`
- Create: `raps-cli/src/marketplace/subscription.rs`
- Create: `raps-cli/src/marketplace/installer.rs`
- Modify: `raps-cli/src/lib.rs`

**Step 1: Write `raps-cli/src/marketplace/mod.rs`**

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

pub mod auth;
pub mod client;
pub mod installer;
pub mod subscription;

pub use auth::MarketplaceAuth;
pub use client::MarketplaceClient;
pub use installer::PluginInstaller;
pub use subscription::SubscriptionManager;
```

**Step 2: Add to `raps-cli/src/lib.rs`**

```rust
pub mod marketplace;
```

**Step 3: Write `raps-cli/src/marketplace/auth.rs`**

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! License key storage and retrieval via system keyring.

use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE: &str = "raps-marketplace";
const ACCOUNT: &str = "license-key";

pub struct MarketplaceAuth;

pub struct TokenResponse {
    pub expires_in: u64,
}

impl MarketplaceAuth {
    pub fn new() -> Self { Self }

    /// Store a license key in the system keyring.
    pub fn store_license_key(&self, key: &str) -> Result<()> {
        let entry = Entry::new(SERVICE, ACCOUNT)
            .context("Failed to access system keyring")?;
        entry.set_password(key)
            .context("Failed to store license key in keyring")?;
        Ok(())
    }

    /// Retrieve the stored license key from the system keyring.
    pub fn get_license_key(&self) -> Result<Option<String>> {
        let entry = Entry::new(SERVICE, ACCOUNT)
            .context("Failed to access system keyring")?;
        match entry.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Keyring error: {e}")),
        }
    }

    /// Remove the stored license key from the system keyring.
    pub fn clear_license_key(&self) -> Result<()> {
        let entry = Entry::new(SERVICE, ACCOUNT)
            .context("Failed to access system keyring")?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Keyring error: {e}")),
        }
    }

    /// Stub: not used in license-key flow, kept for command compatibility.
    pub async fn login(&self) -> Result<TokenResponse> {
        anyhow::bail!(
            "RAPS Pro uses license keys, not passwords.\n\
             Run 'raps marketplace license <key>' to register your key."
        )
    }

    pub async fn load_tokens(&self) -> Result<()> { Ok(()) }

    pub async fn is_authenticated(&self) -> bool {
        self.get_license_key().ok().flatten().is_some()
    }

    pub async fn get_access_token(&self) -> Option<String> {
        self.get_license_key().ok().flatten()
    }

    pub async fn clear_tokens(&self) -> Result<()> {
        self.clear_license_key()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_returns_instance() {
        let _ = MarketplaceAuth::new();
    }
}
```

**Step 4: Commit skeleton**

```bash
git add raps-cli/src/marketplace/ raps-cli/src/lib.rs
git commit -m "feat: add marketplace module skeleton with MarketplaceAuth"
```

---

### Task 19: `MarketplaceClient` — HTTP client

**Files:**
- Create: `raps-cli/src/marketplace/client.rs`

**Step 1: Write `raps-cli/src/marketplace/client.rs`**

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! HTTP client for the RAPS Marketplace API.

use anyhow::{Context, Result};
use reqwest::Client;
use raps_kernel::marketplace::{Plugin, VersionInfo};

const API_BASE: &str = "https://marketplace.rapscli.xyz";

#[derive(Clone)]
pub struct MarketplaceClient {
    client: Client,
}

impl MarketplaceClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent(format!("raps-cli/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    pub async fn get_plugin(&self, slug: &str) -> Result<Plugin> {
        let url = format!("{API_BASE}/plugins/{slug}");
        self.client
            .get(&url)
            .send()
            .await
            .context("Failed to reach marketplace")?
            .error_for_status()
            .context(format!("Plugin '{slug}' not found"))?
            .json::<Plugin>()
            .await
            .context("Failed to parse plugin response")
    }

    pub async fn list_plugins(&self) -> Result<Vec<Plugin>> {
        let url = format!("{API_BASE}/plugins");
        self.client
            .get(&url)
            .send()
            .await
            .context("Failed to reach marketplace")?
            .error_for_status()?
            .json::<Vec<Plugin>>()
            .await
            .context("Failed to parse plugins response")
    }

    pub async fn get_versions(&self, _slug: &str) -> Result<Vec<VersionInfo>> {
        // Versions are embedded in plugin_releases; this returns empty for now
        // until a /plugins/:slug/versions endpoint is added to the Worker
        Ok(vec![])
    }

    /// Download a plugin binary. Returns (bytes, sha256_header, sig_header).
    pub async fn download_plugin(
        &self,
        slug: &str,
        platform: &str,
        license_key: &str,
    ) -> Result<(Vec<u8>, String, String)> {
        let url = format!("{API_BASE}/license/plugins/{slug}/download?platform={platform}");
        let resp = self.client
            .get(&url)
            .bearer_auth(license_key)
            .send()
            .await
            .context("Failed to reach marketplace download endpoint")?
            .error_for_status()
            .context("Download rejected (check license key and entitlements)")?;

        let sha256 = resp
            .headers()
            .get("X-SHA256")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let signature = resp
            .headers()
            .get("X-Ed25519-Signature")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let bytes = resp.bytes().await.context("Failed to read download body")?.to_vec();
        Ok((bytes, sha256, signature))
    }
}
```

**Step 2: Commit**

```bash
git add raps-cli/src/marketplace/client.rs
git commit -m "feat: add MarketplaceClient with plugin listing and download"
```

---

### Task 20: `SubscriptionManager` — validation + 7-day cache

**Files:**
- Create: `raps-cli/src/marketplace/subscription.rs`

**Step 1: Write `raps-cli/src/marketplace/subscription.rs`**

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! License validation with 7-day local caching.

use anyhow::{Context, Result};
use directories::ProjectDirs;
use reqwest::Client;
use raps_kernel::marketplace::ValidateResponse;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const API_BASE: &str = "https://marketplace.rapscli.xyz";

#[derive(Debug, Serialize, Deserialize)]
struct CachedValidation {
    plugins: Vec<String>,
    valid_until: String,  // ISO 8601
}

pub struct SubscriptionManager {
    cache_path: PathBuf,
    client: Client,
}

impl SubscriptionManager {
    pub fn new() -> Result<Self> {
        let dirs = ProjectDirs::from("com", "autodesk", "raps")
            .context("Cannot determine project directories")?;
        let cache_path = dirs.cache_dir().join("marketplace_license.json");
        Ok(Self {
            cache_path,
            client: Client::builder()
                .user_agent(format!("raps-cli/{}", env!("CARGO_PKG_VERSION")))
                .build()?,
        })
    }

    /// Check whether the cached validation is still within the 7-day window.
    fn load_cache(&self) -> Option<CachedValidation> {
        let content = std::fs::read_to_string(&self.cache_path).ok()?;
        let cached: CachedValidation = serde_json::from_str(&content).ok()?;
        let valid_until = chrono::DateTime::parse_from_rfc3339(&cached.valid_until).ok()?;
        if chrono::Utc::now() < valid_until {
            Some(cached)
        } else {
            None
        }
    }

    fn save_cache(&self, resp: &ValidateResponse) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let cached = CachedValidation {
            plugins: resp.plugins.clone(),
            valid_until: resp.valid_until.clone(),
        };
        std::fs::write(&self.cache_path, serde_json::to_string(&cached)?)?;
        Ok(())
    }

    /// Validate license key against the API (with 7-day cache).
    pub async fn validate(&self, license_key: &str) -> Result<ValidateResponse> {
        // Return cached result if still valid
        if let Some(cached) = self.load_cache() {
            return Ok(ValidateResponse {
                valid: true,
                plugins: cached.plugins,
                valid_until: cached.valid_until,
            });
        }

        // Call API
        let resp = self.client
            .post(format!("{API_BASE}/license/validate"))
            .bearer_auth(license_key)
            .send()
            .await
            .context("Failed to reach marketplace for license validation")?
            .error_for_status()
            .context("License validation failed")?
            .json::<ValidateResponse>()
            .await
            .context("Failed to parse validation response")?;

        self.save_cache(&resp).ok(); // cache failure is non-fatal
        Ok(resp)
    }

    pub async fn get_subscription(&self, key: &str) -> Result<ValidateResponse> {
        self.validate(key).await
    }

    pub async fn can_use_pro(&self, key: &str) -> Result<bool> {
        let resp = self.validate(key).await?;
        Ok(resp.valid && !resp.plugins.is_empty())
    }

    pub async fn can_update_pro(&self, key: &str) -> Result<bool> {
        self.can_use_pro(key).await
    }

    pub async fn register_license(&self, _token: &str, key: &str) -> Result<ValidateResponse> {
        // `register_license` called from `raps marketplace license <key>` command
        // Validate immediately to confirm key works, then cache
        self.validate(key).await
    }

    pub async fn clear_cache(&self) -> Result<()> {
        if self.cache_path.exists() {
            std::fs::remove_file(&self.cache_path)?;
        }
        Ok(())
    }

    pub fn format_subscription_status(resp: &ValidateResponse) -> String {
        use colored::Colorize;
        format!(
            "  {:<16} {}\n  {:<16} {}\n  {:<16} {}",
            "Status:".bold(), "active".green(),
            "Plugins:".bold(), resp.plugins.join(", "),
            "Valid until:".bold(), resp.valid_until,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_miss_on_expired() {
        let cached = CachedValidation {
            plugins: vec!["acc-bulk".to_string()],
            valid_until: "2020-01-01T00:00:00Z".to_string(), // past
        };
        let content = serde_json::to_string(&cached).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), content).unwrap();

        // Parse and check expiry logic manually
        let v: CachedValidation = serde_json::from_str(&std::fs::read_to_string(tmp.path()).unwrap()).unwrap();
        let valid_until = chrono::DateTime::parse_from_rfc3339(&v.valid_until).unwrap();
        assert!(chrono::Utc::now() > valid_until, "Should be expired");
    }
}
```

**Step 2: Add `chrono` to `raps-cli/Cargo.toml` if not present**

```bash
grep -q "chrono" raps-cli/Cargo.toml || \
  sed -i '/\[dependencies\]/a chrono = { version = "0.4", features = ["serde"] }' raps-cli/Cargo.toml
```

**Step 3: Run test**

```bash
cargo test -p raps-cli marketplace::subscription
```

**Step 4: Commit**

```bash
git add raps-cli/src/marketplace/subscription.rs raps-cli/Cargo.toml
git commit -m "feat: add SubscriptionManager with 7-day license validation cache"
```

---

### Task 21: `PluginInstaller` — download, verify, install

**Files:**
- Create: `raps-cli/src/marketplace/installer.rs`

**Step 1: Write `raps-cli/src/marketplace/installer.rs`**

```rust
// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Downloads, verifies (Ed25519 + SHA-256), and installs plugin binaries.

use anyhow::{Context, Result};
use raps_kernel::marketplace::Installation;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use super::client::MarketplaceClient;
use crate::plugins::{compute_binary_hash, PluginConfig, PluginEntry};

// This public key is baked into the CLI binary — the signing key never leaves your machine.
// Generate with: `openssl genpkey -algorithm ed25519 | openssl pkey -outform DER | xxd -p -c 256`
const MARKETPLACE_PUBLIC_KEY: &str = env!("RAPS_MARKETPLACE_ED25519_PUBKEY");

pub struct InstallResult {
    pub name: String,
    pub version: String,
    pub binary_path: PathBuf,
    pub suggest_path: bool,
}

pub struct PluginInstaller {
    client: MarketplaceClient,
    install_dir: PathBuf,
}

impl PluginInstaller {
    pub fn new(client: MarketplaceClient) -> Result<Self> {
        let install_dir = Self::default_install_dir()?;
        Ok(Self { client, install_dir })
    }

    fn default_install_dir() -> Result<PathBuf> {
        if cfg!(windows) {
            Ok(dirs::home_dir()
                .context("Cannot determine home directory")?
                .join(".raps")
                .join("bin"))
        } else {
            Ok(dirs::home_dir()
                .context("Cannot determine home directory")?
                .join(".local")
                .join("bin"))
        }
    }

    pub fn path_suggestion(&self) -> String {
        format!(
            "Add {} to your PATH:\n  export PATH=\"$PATH:{}\"",
            self.install_dir.display(),
            self.install_dir.display()
        )
    }

    fn is_in_path(&self) -> bool {
        if let Ok(path_var) = std::env::var("PATH") {
            path_var.split(':').any(|p| PathBuf::from(p) == self.install_dir)
        } else {
            false
        }
    }

    pub async fn install(&self, slug: &str, _version: Option<&str>) -> Result<InstallResult> {
        use super::auth::MarketplaceAuth;
        use super::subscription::SubscriptionManager;

        let auth = MarketplaceAuth::new();
        let key = auth.get_license_key()
            .context("Keyring error")?
            .context("No license key found. Run 'raps marketplace license <key>' first.")?;

        // Validate license and get entitlements (uses 7-day cache)
        let sub_manager = SubscriptionManager::new()?;
        let validation = sub_manager.validate(&key).await?;
        if !validation.plugins.contains(&slug.to_string()) {
            anyhow::bail!(
                "Your license does not include '{}'. Plugins covered: {}",
                slug,
                if validation.plugins.is_empty() { "none".to_string() } else { validation.plugins.join(", ") }
            );
        }

        let platform = detect_platform();
        let (bytes, sha256_header, signature_hex) = self.client
            .download_plugin(slug, &platform, &key)
            .await?;

        // Verify SHA-256
        let computed = hex::encode(Sha256::digest(&bytes));
        if !sha256_header.is_empty() && computed != sha256_header {
            anyhow::bail!(
                "SHA-256 mismatch for '{}'. Expected {}, got {}.",
                slug, sha256_header, computed
            );
        }

        // Verify Ed25519 signature
        self.verify_signature(&bytes, &signature_hex)
            .context(format!("Ed25519 signature verification failed for '{slug}'"))?;

        // Write binary to install dir
        std::fs::create_dir_all(&self.install_dir)?;
        let bin_name = if cfg!(windows) {
            format!("raps-{slug}.exe")
        } else {
            format!("raps-{slug}")
        };
        let dest = self.install_dir.join(&bin_name);

        // Write to temp then rename (atomic on same filesystem)
        let tmp = dest.with_extension("tmp");
        std::fs::write(&tmp, &bytes)?;

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
        }

        std::fs::rename(&tmp, &dest)?;

        // Record in plugins.json with sha256 + signature
        let mut config = PluginConfig::load().unwrap_or_default();
        config.plugins.insert(slug.to_string(), PluginEntry {
            enabled: true,
            path: Some(dest.to_string_lossy().to_string()),
            description: None,
            sha256: Some(computed),
            public_key: Some(MARKETPLACE_PUBLIC_KEY.to_string()),
            signature: Some(signature_hex),
            trusted: true,
        });
        config.save().ok();

        Ok(InstallResult {
            name: slug.to_string(),
            version: "latest".to_string(), // TODO: parse from response headers
            binary_path: dest,
            suggest_path: !self.is_in_path(),
        })
    }

    fn verify_signature(&self, data: &[u8], signature_hex: &str) -> Result<()> {
        use ed25519_dalek::{Signature, VerifyingKey};

        let pubkey_bytes = hex::decode(MARKETPLACE_PUBLIC_KEY)
            .context("Invalid hardcoded public key")?;
        let pubkey_array: [u8; 32] = pubkey_bytes.try_into()
            .map_err(|_| anyhow::anyhow!("Public key must be 32 bytes"))?;
        let verifying_key = VerifyingKey::from_bytes(&pubkey_array)
            .context("Invalid ed25519 public key")?;

        let sig_bytes = hex::decode(signature_hex)
            .context("Invalid signature hex from server")?;
        let sig_array: [u8; 64] = sig_bytes.try_into()
            .map_err(|_| anyhow::anyhow!("Signature must be 64 bytes"))?;
        let signature = Signature::from_bytes(&sig_array);

        verifying_key.verify_strict(data, &signature)
            .context("Signature does not match binary")?;
        Ok(())
    }

    pub async fn uninstall(&self, slug: &str) -> Result<()> {
        let bin_name = if cfg!(windows) { format!("raps-{slug}.exe") } else { format!("raps-{slug}") };
        let path = self.install_dir.join(&bin_name);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        let mut config = PluginConfig::load().unwrap_or_default();
        config.plugins.remove(slug);
        config.save().ok();
        Ok(())
    }

    pub async fn update_with_rollback(&self, slug: &str, version: Option<&str>) -> Result<InstallResult> {
        // Back up existing binary
        let bin_name = if cfg!(windows) { format!("raps-{slug}.exe") } else { format!("raps-{slug}") };
        let dest = self.install_dir.join(&bin_name);
        let backup = dest.with_extension("bak");
        if dest.exists() { std::fs::copy(&dest, &backup).ok(); }

        match self.install(slug, version).await {
            Ok(r) => {
                if backup.exists() { std::fs::remove_file(&backup).ok(); }
                Ok(r)
            }
            Err(e) => {
                if backup.exists() { std::fs::rename(&backup, &dest).ok(); }
                Err(e)
            }
        }
    }

    pub async fn load_registry(&self) -> Result<Vec<Installation>> {
        let config = PluginConfig::load().unwrap_or_default();
        Ok(config.plugins.iter()
            .filter(|(_, e)| e.trusted && e.path.is_some())
            .map(|(name, _)| Installation {
                name: name.clone(),
                version: "unknown".to_string(),
                slug: name.clone(),
                installed_at: String::new(),
            })
            .collect())
    }

    pub fn check_raps_compatibility(compat: &Option<String>) -> Result<bool> {
        match compat {
            None => Ok(true), // no constraint = compatible
            Some(req) => {
                let current = semver::Version::parse(env!("CARGO_PKG_VERSION"))?;
                let requirement = semver::VersionReq::parse(req)?;
                Ok(requirement.matches(&current))
            }
        }
    }
}

fn detect_platform() -> String {
    if cfg!(target_os = "windows") { "win-x64".to_string() }
    else if cfg!(target_os = "macos") { "darwin-arm64".to_string() }
    else { "linux-x64".to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_platform_is_non_empty() {
        let p = detect_platform();
        assert!(!p.is_empty());
        assert!(["linux-x64", "darwin-arm64", "win-x64"].contains(&p.as_str()));
    }

    #[test]
    fn check_raps_compatibility_none_is_true() {
        assert!(PluginInstaller::check_raps_compatibility(&None).unwrap());
    }
}
```

**Step 2: Add `RAPS_MARKETPLACE_ED25519_PUBKEY` to build**

In `raps-cli/build.rs` (create if it doesn't exist):

```rust
fn main() {
    // Ed25519 public key for marketplace binary verification.
    // Set this env var in CI before building release binaries.
    // For development builds, uses a placeholder that will fail sig verification.
    let pubkey = std::env::var("RAPS_MARKETPLACE_ED25519_PUBKEY")
        .unwrap_or_else(|_| "0".repeat(64));
    println!("cargo:rustc-env=RAPS_MARKETPLACE_ED25519_PUBKEY={pubkey}");
    println!("cargo:rerun-if-env-changed=RAPS_MARKETPLACE_ED25519_PUBKEY");
}
```

**Step 3: Run tests**

```bash
cargo test -p raps-cli marketplace::installer
```

**Step 4: Commit**

```bash
git add raps-cli/src/marketplace/installer.rs raps-cli/build.rs
git commit -m "feat: add PluginInstaller with Ed25519+SHA256 verification and atomic install"
```

---

### Task 22: Wire `raps marketplace license` command

The `commands/marketplace/auth.rs` already calls `auth.load_tokens()` and `sub_manager.register_license()`. The implementations now exist. The only thing remaining is to ensure `commands/marketplace/auth.rs` stores the license key when `raps marketplace license <key>` is called.

**Files:**
- Modify: `raps-cli/src/commands/marketplace/auth.rs`

**Step 1: Update `license()` in `auth.rs` to store the key**

Find the existing `license()` function and replace its body:

```rust
pub(super) async fn license(args: LicenseArgs, output_format: OutputFormat) -> Result<()> {
    let auth = MarketplaceAuth::new();

    // Store the key in keyring
    auth.store_license_key(&args.key)
        .context("Failed to store license key")?;

    // Validate immediately so the user knows if the key is good
    let sub_manager = SubscriptionManager::new()?;
    let subscription = sub_manager.validate(&args.key).await
        .context("License key stored, but validation failed — check the key and try again")?;

    match output_format {
        OutputFormat::Table => {
            println!("{} License key registered and validated!", "✓".green().bold());
            println!("{}", SubscriptionManager::format_subscription_status(&subscription));
        }
        _ => {
            output_format.write(&subscription)?;
        }
    }

    Ok(())
}
```

**Step 2: Build and smoke test**

```bash
cargo build -p raps-cli 2>&1 | tail -20
```
Expected: builds without error.

**Step 3: Commit**

```bash
git add raps-cli/src/commands/marketplace/auth.rs
git commit -m "feat: wire 'raps marketplace license' to store key and validate"
```

---

### Task 23: Final build verification + integration smoke test

**Step 1: Full build**

```bash
cargo build --workspace
```
Expected: no errors.

**Step 2: Run all tests**

```bash
cargo test --workspace 2>&1 | tail -30
```
Expected: all tests pass.

**Step 3: Smoke test CLI help**

```bash
./target/debug/raps marketplace --help
```
Expected: shows Search, Install, Uninstall, Update, Login, Logout, Status, License, Init, Package, Publish, Review, ClearCache subcommands.

```bash
./target/debug/raps marketplace license --help
```
Expected: shows `<key>` argument.

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: complete RAPS marketplace CLI integration"
```

---

## Deployment Checklist

### Before going live:

- [ ] Generate Ed25519 keypair: `openssl genpkey -algorithm ed25519 > signing.key && openssl pkey -in signing.key -pubout -outform DER | xxd -p -c 256 > pubkey.hex`
- [ ] Set `RAPS_MARKETPLACE_ED25519_PUBKEY` in CI before release builds
- [ ] Set all Worker secrets via `wrangler secret put`
- [ ] Generate `ADMIN_PASSWORD_HASH`: `<salt>:<pbkdf2_hash>` (run the hash function locally)
- [ ] Register Stripe webhook for `marketplace.rapscli.xyz/webhooks/stripe`
- [ ] Create at least one plugin row in D1 via `wrangler d1 execute`
- [ ] Set up Cloudflare Pages custom domains: `buy.rapscli.xyz`, `admin.rapscli.xyz`
- [ ] Integrate Resend (or Cloudflare Email Workers) for license key delivery emails in `handleCheckoutCompleted`

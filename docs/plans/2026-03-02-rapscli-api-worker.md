# rapscli.xyz API Worker — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Single Cloudflare Worker at `rapscli.xyz` serving install scripts, version API, and download badge — the three highest-value endpoints for CLI distribution.

**Architecture:** One Worker with route-based dispatch (same pattern as device-auth and webhook-gateway). Uses KV for cached GitHub release metadata (updated by a scheduled cron trigger every hour). Install script serving is pure edge response with `User-Agent` sniffing for OS detection. Badge is SVG generation with shield.io-compatible format.

**Tech Stack:** Cloudflare Workers, KV namespace, Cron Triggers, GitHub API

---

## File Layout

```
workers/rapscli-api/
├── wrangler.toml          # Routes + KV binding + cron trigger
├── package.json           # wrangler + vitest
├── src/
│   ├── index.js           # Route dispatch + cron handler
│   ├── install.js         # GET /install — serve install.sh or install.ps1
│   ├── version.js         # GET /api/version — latest release info
│   ├── badge.js           # GET /api/badge/* — SVG badge generation
│   └── github.js          # GitHub API client (fetch latest release)
```

---

### Task 1: Create Worker scaffolding

**Files:**
- Create: `workers/rapscli-api/wrangler.toml`
- Create: `workers/rapscli-api/package.json`

**Step 1: Create wrangler.toml**

```toml
name = "rapscli-api"
main = "src/index.js"
compatibility_date = "2025-12-01"

# KV namespace for cached release metadata
kv_namespaces = [
  { binding = "RELEASE_CACHE", id = "TBD", preview_id = "TBD" }
]

# Cron trigger: refresh release metadata every hour
[triggers]
crons = ["0 * * * *"]

routes = [
  { pattern = "rapscli.xyz/install", zone_name = "rapscli.xyz" },
  { pattern = "rapscli.xyz/install.sh", zone_name = "rapscli.xyz" },
  { pattern = "rapscli.xyz/install.ps1", zone_name = "rapscli.xyz" },
  { pattern = "rapscli.xyz/api/version", zone_name = "rapscli.xyz" },
  { pattern = "rapscli.xyz/api/badge/*", zone_name = "rapscli.xyz" }
]
```

Note: KV namespace IDs are filled in after `wrangler kv namespace create RELEASE_CACHE`.

**Step 2: Create package.json**

```json
{
  "name": "@raps/rapscli-api",
  "version": "1.0.0",
  "private": true,
  "description": "Cloudflare Worker — install scripts, version API, and badges for rapscli.xyz",
  "main": "src/index.js",
  "scripts": {
    "dev": "wrangler dev",
    "deploy": "wrangler deploy",
    "test": "vitest run",
    "test:watch": "vitest"
  },
  "devDependencies": {
    "wrangler": "^4.0.0",
    "vitest": "^3.0.0"
  }
}
```

**Step 3: Commit**

```bash
git add workers/rapscli-api/
git commit -m "chore: scaffold rapscli-api worker"
```

---

### Task 2: Implement GitHub release fetcher

**Files:**
- Create: `workers/rapscli-api/src/github.js`

**Step 1: Write the module**

This module fetches the latest release from GitHub and caches in KV. The cron trigger calls `refreshReleaseCache()`. Other handlers call `getLatestRelease()` which reads KV first.

```js
// workers/rapscli-api/src/github.js

const GITHUB_API = "https://api.github.com/repos/dmytro-yemelianov/raps/releases/latest";
const CACHE_KEY = "latest_release";
const CACHE_TTL = 3600; // 1 hour

/**
 * Fetch latest release from GitHub and cache in KV.
 * Called by cron trigger and as fallback on cache miss.
 */
export async function refreshReleaseCache(env) {
  const resp = await fetch(GITHUB_API, {
    headers: {
      "User-Agent": "rapscli-api-worker",
      "Accept": "application/vnd.github+json",
    },
  });

  if (!resp.ok) return null;

  const release = await resp.json();
  const data = {
    version: release.tag_name.replace(/^v/, ""),
    tag: release.tag_name,
    url: release.html_url,
    published_at: release.published_at,
    asset_count: release.assets?.length || 0,
    fetched_at: new Date().toISOString(),
  };

  await env.RELEASE_CACHE.put(CACHE_KEY, JSON.stringify(data), {
    expirationTtl: CACHE_TTL,
  });

  return data;
}

/**
 * Get latest release (KV cache first, GitHub fallback).
 */
export async function getLatestRelease(env) {
  const cached = await env.RELEASE_CACHE.get(CACHE_KEY, { type: "json" });
  if (cached) return cached;
  return refreshReleaseCache(env);
}
```

**Step 2: Commit**

```bash
git add workers/rapscli-api/src/github.js
git commit -m "feat: add GitHub release fetcher with KV cache"
```

---

### Task 3: Implement install script endpoint

**Files:**
- Create: `workers/rapscli-api/src/install.js`

**Step 1: Write the module**

Serves the install script from the repo's raw GitHub content, with `User-Agent` sniffing for smart defaults. `GET /install` returns bash or PowerShell based on UA. `/install.sh` and `/install.ps1` serve the explicit format.

```js
// workers/rapscli-api/src/install.js

const INSTALL_SH_URL = "https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh";
const INSTALL_PS1_URL = "https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.ps1";

/**
 * GET /install — auto-detect OS from User-Agent, serve appropriate script.
 * GET /install.sh — always serve bash script.
 * GET /install.ps1 — always serve PowerShell script.
 */
export async function handleInstall(request, env) {
  const url = new URL(request.url);
  const ua = (request.headers.get("User-Agent") || "").toLowerCase();

  let scriptUrl;
  let contentType;

  if (url.pathname === "/install.ps1") {
    scriptUrl = INSTALL_PS1_URL;
    contentType = "text/plain; charset=utf-8";
  } else if (url.pathname === "/install.sh") {
    scriptUrl = INSTALL_SH_URL;
    contentType = "text/x-shellscript; charset=utf-8";
  } else {
    // Auto-detect: PowerShell UA means Windows
    const isWindows = ua.includes("powershell") || ua.includes("windowspowershell");
    scriptUrl = isWindows ? INSTALL_PS1_URL : INSTALL_SH_URL;
    contentType = isWindows ? "text/plain; charset=utf-8" : "text/x-shellscript; charset=utf-8";
  }

  // Fetch from GitHub (CF edge caches this automatically)
  const resp = await fetch(scriptUrl, {
    cf: { cacheTtl: 300, cacheEverything: true },
  });

  if (!resp.ok) {
    return new Response("Failed to fetch install script", { status: 502 });
  }

  const body = await resp.text();

  // Track install count (fire-and-forget)
  trackInstall(env, request).catch(() => {});

  return new Response(body, {
    headers: {
      "Content-Type": contentType,
      "Cache-Control": "public, max-age=300",
    },
  });
}

async function trackInstall(env, request) {
  const ua = (request.headers.get("User-Agent") || "").toLowerCase();
  const os = ua.includes("powershell") ? "windows"
    : ua.includes("darwin") || ua.includes("mac") ? "macos"
    : "linux";

  const key = `installs:${new Date().toISOString().slice(0, 10)}:${os}`;
  const current = parseInt(await env.RELEASE_CACHE.get(key) || "0", 10);
  await env.RELEASE_CACHE.put(key, String(current + 1), {
    expirationTtl: 86400 * 90, // Keep 90 days
  });
}
```

**Step 2: Commit**

```bash
git add workers/rapscli-api/src/install.js
git commit -m "feat: add install script endpoint with OS auto-detection"
```

---

### Task 4: Implement version API endpoint

**Files:**
- Create: `workers/rapscli-api/src/version.js`

**Step 1: Write the module**

```js
// workers/rapscli-api/src/version.js

import { getLatestRelease } from "./github.js";

/**
 * GET /api/version?current=4.9.0
 *
 * Returns:
 *   { latest, tag, url, published_at, update_available, breaking }
 */
export async function handleVersion(request, env) {
  const url = new URL(request.url);
  const current = url.searchParams.get("current") || "";

  const release = await getLatestRelease(env);
  if (!release) {
    return json({ error: "Unable to fetch release info" }, 503);
  }

  const response = {
    latest: release.version,
    tag: release.tag,
    url: release.url,
    published_at: release.published_at,
    update_available: current ? release.version !== current : null,
    breaking: current ? isMajorBump(current, release.version) : null,
  };

  return json(response);
}

function isMajorBump(current, latest) {
  const curMajor = parseInt(current.split(".")[0], 10);
  const latMajor = parseInt(latest.split(".")[0], 10);
  return latMajor > curMajor;
}

function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json",
      "Cache-Control": "public, max-age=300",
      "Access-Control-Allow-Origin": "*",
    },
  });
}
```

**Step 2: Commit**

```bash
git add workers/rapscli-api/src/version.js
git commit -m "feat: add version check API endpoint"
```

---

### Task 5: Implement badge endpoint

**Files:**
- Create: `workers/rapscli-api/src/badge.js`

**Step 1: Write the module**

Generates shields.io-compatible SVG badges. Supports:
- `GET /api/badge/version` — latest version badge
- `GET /api/badge/downloads` — total install count (from KV tracking)

```js
// workers/rapscli-api/src/badge.js

import { getLatestRelease } from "./github.js";

/**
 * GET /api/badge/version — version badge SVG
 * GET /api/badge/downloads — download count badge SVG
 */
export async function handleBadge(request, env) {
  const url = new URL(request.url);
  const type = url.pathname.replace("/api/badge/", "").replace("/", "");

  if (type === "version") {
    return versionBadge(env);
  }

  if (type === "downloads") {
    return downloadsBadge(env);
  }

  return new Response("Unknown badge type", { status: 404 });
}

async function versionBadge(env) {
  const release = await getLatestRelease(env);
  const version = release ? `v${release.version}` : "unknown";
  return svg("raps", version, "#2563eb");
}

async function downloadsBadge(env) {
  // Sum all install counts from KV
  const list = await env.RELEASE_CACHE.list({ prefix: "installs:" });
  let total = 0;
  for (const key of list.keys) {
    const val = await env.RELEASE_CACHE.get(key.name);
    total += parseInt(val || "0", 10);
  }

  const label = total >= 1000 ? `${(total / 1000).toFixed(1)}k` : String(total);
  return svg("downloads", label, "#059669");
}

function svg(left, right, color) {
  const leftWidth = left.length * 7 + 12;
  const rightWidth = right.length * 7 + 12;
  const totalWidth = leftWidth + rightWidth;

  const body = `<svg xmlns="http://www.w3.org/2000/svg" width="${totalWidth}" height="20" role="img">
  <linearGradient id="s" x2="0" y2="100%"><stop offset="0" stop-color="#bbb" stop-opacity=".1"/><stop offset="1" stop-opacity=".1"/></linearGradient>
  <clipPath id="r"><rect width="${totalWidth}" height="20" rx="3" fill="#fff"/></clipPath>
  <g clip-path="url(#r)">
    <rect width="${leftWidth}" height="20" fill="#555"/>
    <rect x="${leftWidth}" width="${rightWidth}" height="20" fill="${color}"/>
    <rect width="${totalWidth}" height="20" fill="url(#s)"/>
  </g>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" text-rendering="geometricPrecision" font-size="11">
    <text x="${leftWidth / 2}" y="15" fill="#010101" fill-opacity=".3">${left}</text>
    <text x="${leftWidth / 2}" y="14">${left}</text>
    <text x="${leftWidth + rightWidth / 2}" y="15" fill="#010101" fill-opacity=".3">${right}</text>
    <text x="${leftWidth + rightWidth / 2}" y="14">${right}</text>
  </g>
</svg>`;

  return new Response(body, {
    headers: {
      "Content-Type": "image/svg+xml",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
```

**Step 2: Commit**

```bash
git add workers/rapscli-api/src/badge.js
git commit -m "feat: add SVG badge endpoint for version and downloads"
```

---

### Task 6: Wire up route dispatch and cron handler

**Files:**
- Create: `workers/rapscli-api/src/index.js`

**Step 1: Write the main entry point**

```js
// RAPS CLI API Worker — Cloudflare Worker
//
// Routes:
//   GET  /install          — auto-detect OS, serve install script
//   GET  /install.sh       — bash install script
//   GET  /install.ps1      — PowerShell install script
//   GET  /api/version      — latest release info
//   GET  /api/badge/*      — SVG badges (version, downloads)
//   GET  /health           — health check
//
// Cron:
//   Every hour — refresh GitHub release cache

import { handleInstall } from "./install.js";
import { handleVersion } from "./version.js";
import { handleBadge } from "./badge.js";
import { refreshReleaseCache } from "./github.js";

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const path = url.pathname;

    if (path === "/health" && request.method === "GET") {
      return new Response(
        JSON.stringify({ status: "ok", service: "rapscli-api" }),
        { headers: { "Content-Type": "application/json" } }
      );
    }

    if ((path === "/install" || path === "/install.sh" || path === "/install.ps1") && request.method === "GET") {
      return handleInstall(request, env);
    }

    if (path === "/api/version" && request.method === "GET") {
      return handleVersion(request, env);
    }

    if (path.startsWith("/api/badge/") && request.method === "GET") {
      return handleBadge(request, env);
    }

    return new Response("Not found", { status: 404 });
  },

  async scheduled(event, env, ctx) {
    ctx.waitUntil(refreshReleaseCache(env));
  },
};
```

**Step 2: Commit**

```bash
git add workers/rapscli-api/src/index.js
git commit -m "feat: wire route dispatch and cron handler for rapscli-api"
```

---

### Task 7: Create KV namespace and deploy

**Step 1: Install dependencies**

```bash
cd workers/rapscli-api && npm install
```

**Step 2: Create KV namespace**

```bash
npx wrangler kv namespace create RELEASE_CACHE
```

Copy the output `id` into `wrangler.toml`.

For preview:
```bash
npx wrangler kv namespace create RELEASE_CACHE --preview
```

Copy the `preview_id` into `wrangler.toml`.

**Step 3: Deploy**

```bash
CLOUDFLARE_API_TOKEN=<token> npx wrangler deploy
```

**Step 4: Verify all endpoints**

```bash
# Health
curl https://rapscli.xyz/health

# Install script (bash)
curl -fsSL https://rapscli.xyz/install | head -5

# Install script (PowerShell auto-detect)
curl -A "PowerShell/7.0" https://rapscli.xyz/install | head -5

# Explicit formats
curl https://rapscli.xyz/install.sh | head -3
curl https://rapscli.xyz/install.ps1 | head -3

# Version API
curl https://rapscli.xyz/api/version
curl "https://rapscli.xyz/api/version?current=4.18.0"

# Badges
curl https://rapscli.xyz/api/badge/version -o badge.svg
curl https://rapscli.xyz/api/badge/downloads -o downloads.svg
```

**Step 5: Commit wrangler.toml with real KV IDs**

```bash
git add workers/rapscli-api/wrangler.toml
git commit -m "chore: add KV namespace IDs after deployment"
```

---

### Task 8: Update device-auth worker health route

**Files:**
- Modify: `workers/device-auth/wrangler.toml`

The device-auth worker currently claims `rapscli.xyz/health`. The new rapscli-api worker should own the shared `/health` route. Remove it from device-auth routes.

**Step 1: Edit wrangler.toml**

Remove the health route line:
```toml
routes = [
  { pattern = "rapscli.xyz/device/*", zone_name = "rapscli.xyz" },
  { pattern = "rapscli.xyz/device", zone_name = "rapscli.xyz" }
]
```

**Step 2: Add `/health` route to rapscli-api wrangler.toml**

Add to routes array:
```toml
  { pattern = "rapscli.xyz/health", zone_name = "rapscli.xyz" }
```

**Step 3: Redeploy both workers**

```bash
cd workers/device-auth && CLOUDFLARE_API_TOKEN=<token> npx wrangler deploy
cd workers/rapscli-api && CLOUDFLARE_API_TOKEN=<token> npx wrangler deploy
```

**Step 4: Commit**

```bash
git add workers/device-auth/wrangler.toml workers/rapscli-api/wrangler.toml
git commit -m "fix: move /health route from device-auth to rapscli-api worker"
```

---

## Verification Checklist

- [ ] `curl https://rapscli.xyz/health` → `{"status":"ok","service":"rapscli-api"}`
- [ ] `curl -fsSL https://rapscli.xyz/install | sh` installs RAPS on Linux/macOS
- [ ] `curl https://rapscli.xyz/api/version` returns latest version JSON
- [ ] `curl https://rapscli.xyz/api/version?current=4.0.0` shows `update_available: true, breaking: true`
- [ ] `curl https://rapscli.xyz/api/badge/version` returns valid SVG
- [ ] `curl https://rapscli.xyz/api/badge/downloads` returns valid SVG
- [ ] `curl https://rapscli.xyz/device` still works (device-auth worker unaffected)
- [ ] Cron trigger fires and populates KV (check `wrangler kv key list --namespace-id=...`)

# URL Shortener Design

## What it is

A Cloudflare Worker at `go.rapscli.xyz` that provides admin-managed URL shortening. Links can be hardcoded in source or created dynamically via API or admin UI.

## Architecture

Single Worker (`raps-url-shortener`) with a KV namespace (`URL_SHORTENER`). No external dependencies beyond the Cloudflare runtime.

```
go.rapscli.xyz/<code>     → 301 redirect
go.rapscli.xyz/api/*      → JSON API (token-protected)
go.rapscli.xyz/admin      → Admin UI (HTML, served by Worker)
```

## Routes

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/<code>` | Redirect to destination URL |
| `POST` | `/api/shorten` | Create link |
| `DELETE` | `/api/links/<code>` | Delete dynamic link |
| `GET` | `/api/links` | List all dynamic links |
| `GET` | `/admin` | Admin UI |

## Storage

**KV namespace:** `URL_SHORTENER`
- Key: short code (e.g. `discord`)
- Value: `{ "url": "https://...", "created_at": "2026-03-09T..." }`

**Hardcoded links:** `const HARDCODED: Record<string, string>` in Worker source. Checked before KV on every request. Cannot be modified via API — require a code deploy.

**Lookup order:** hardcoded → KV → 404

## API

### POST /api/shorten

```json
// Request
{ "url": "https://example.com", "code": "optional-custom-code" }

// Response
{ "code": "abc123", "short_url": "https://go.rapscli.xyz/abc123" }
```

- `code` is optional; if omitted, a 6-char alphanumeric code is auto-generated
- Returns 409 if code already exists

### DELETE /api/links/:code

Deletes a dynamic link. Returns 404 if not found or if it's a hardcoded link.

### GET /api/links

Returns all dynamic links from KV:

```json
[{ "code": "abc123", "url": "https://...", "created_at": "..." }]
```

## Auth

All `/api/*` routes require `Authorization: Bearer <ADMIN_TOKEN>`. Token stored as a Worker secret (`ADMIN_TOKEN`).

## Admin UI

Inline HTML page served at `/admin`. Features:
- List all links (hardcoded labeled as such, dynamic with delete button)
- Create form: URL field + optional custom code field
- Token input stored in `localStorage`
- Calls the `/api/*` endpoints

## Short code generation

6 random alphanumeric characters (`[a-z0-9]`). Retries on collision (up to 5 attempts).

## Redirect behavior

- 301 (permanent) for all redirects
- 404 page served as plain HTML for unknown codes

## Repository structure

Lives in `raps/workers/url-shortener/` as a new Worker alongside the existing `device-auth`, `rapscli-api`, and `webhook-gateway` workers.

```
workers/url-shortener/
  wrangler.toml
  src/
    index.ts      — request router + redirect logic
    api.ts        — /api/* handlers
    admin.ts      — admin UI HTML
    auth.ts       — token verification middleware
```

## Deployment

```bash
wrangler kv namespace create URL_SHORTENER
wrangler secret put ADMIN_TOKEN
wrangler deploy
```

Route in `wrangler.toml`:
```toml
routes = [{ pattern = "go.rapscli.xyz/*", zone_name = "rapscli.xyz" }]
```

## GitHub Actions

Add `build-url-shortener.yml` workflow triggered on push to `workers/url-shortener/**` on main branch, running `wrangler deploy`.

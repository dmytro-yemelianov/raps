# RAPS Pro Plugin Marketplace — Design

**Date:** 2026-03-08
**Status:** Approved

## Overview

A paid marketplace where pro-version RAPS plugins can be purchased and downloaded by paying customers. Includes vendor admin infrastructure for customer and subscription management. Built on Cloudflare Workers + D1 + R2, with Stripe as the payment processor.

---

## Repositories

Two private repositories:

**`raps-marketplace-api`** — Cloudflare Worker
- All backend logic: license validation, plugin downloads, Stripe webhooks, admin API
- Bindings: D1 (database), R2 (plugin binaries), KV (license token cache)

**`raps-marketplace-frontend`** — two Cloudflare Pages sites
- `storefront/` — public plugin catalog at `buy.rapscli.xyz` (or `pro.rapscli.xyz`)
- `admin/` — vendor admin dashboard at `admin.rapscli.xyz`

Both repos live under the same Cloudflare account as the existing `rapscli.xyz` Pages site.

---

## Licensing Model

- Per-seat subscription (monthly or annual, ~20% annual discount)
- Seat count on honor system — no technical enforcement
- License enforced via a 7-day periodic online check; CLI caches the result locally
- License key is the only customer credential for CLI use

---

## Data Model (D1)

### `customers`
| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER PK | |
| `email` | TEXT UNIQUE | |
| `stripe_customer_id` | TEXT UNIQUE | |
| `created_at` | DATETIME | |

### `subscriptions`
| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER PK | |
| `customer_id` | INTEGER FK | |
| `plugin_id` | INTEGER FK | |
| `stripe_subscription_id` | TEXT UNIQUE | |
| `seat_count` | INTEGER | Honor system |
| `status` | TEXT | active / canceled / past_due |
| `current_period_end` | DATETIME | |

### `licenses`
| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER PK | |
| `subscription_id` | INTEGER FK | |
| `key_hash` | TEXT UNIQUE | bcrypt of 32-byte hex key |
| `created_at` | DATETIME | |
| `last_validated_at` | DATETIME | |
| `revoked` | BOOLEAN | Default false |

### `plugins`
| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER PK | |
| `slug` | TEXT UNIQUE | e.g. `acc-bulk` |
| `name` | TEXT | |
| `description` | TEXT | |
| `price_monthly_cents` | INTEGER | |
| `price_yearly_cents` | INTEGER | |
| `stripe_price_id_monthly` | TEXT | |
| `stripe_price_id_yearly` | TEXT | |
| `latest_version` | TEXT | |
| `published` | BOOLEAN | |

### `plugin_releases`
| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER PK | |
| `plugin_id` | INTEGER FK | |
| `version` | TEXT | semver |
| `platform` | TEXT | linux-x64, darwin-arm64, win-x64 |
| `r2_key` | TEXT | e.g. `plugins/acc-bulk/v1.2.0/linux-x64/raps-acc-bulk` |
| `sha256` | TEXT | |
| `ed25519_signature` | TEXT | hex-encoded |
| `published_at` | DATETIME | |

R2 binaries are not publicly accessible — served only through the Worker after license validation.

---

## API Endpoints

### Public (no auth)
- `GET /plugins` — list published plugins with pricing
- `POST /checkout` — create Stripe Checkout session, return redirect URL
- `POST /webhooks/stripe` — handle `checkout.session.completed`, `customer.subscription.updated`, `customer.subscription.deleted`

### CLI (Bearer license key)
- `POST /license/validate` — verify key is active, return plugin entitlements + `valid_until` (now + 7 days)
- `GET /plugins/:slug/download?platform=<platform>` — validate license, stream signed binary from R2

### Admin (httpOnly JWT cookie)
- `POST /admin/login` — issue 1h JWT, no self-registration
- `GET /admin/customers` — paginated customer list
- `GET /admin/customers/:id` — full detail: subscriptions, licenses
- `POST /admin/licenses/:id/revoke` — mark license revoked
- `POST /admin/plugins/:slug/releases` — upload new signed binary to R2
- `GET /admin/metrics` — MRR, active seats, churn

---

## CLI Integration Flow

`raps plugin install acc-bulk`:

1. Check `~/.config/raps/plugins.json` for stored license key; if missing prompt `raps plugin auth <key>`
2. License key stored in system keyring (via `keyring` crate already in workspace)
3. Check local cache: if `valid_until` is in the future, skip network call
4. Otherwise call `POST /license/validate` — update local cache with new `valid_until`
5. Call `GET /plugins/acc-bulk/download?platform=<detected>` — stream binary to temp file
6. Verify Ed25519 signature against public key hardcoded in the RAPS binary
7. Move binary to `~/.local/bin/raps-acc-bulk` (platform-appropriate path)
8. Write entry to `plugins.json` with `sha256` and `signature`

Subsequent invocations use the existing TOFU+Ed25519 mechanism in `plugins.rs` — no marketplace involvement. The 7-day re-check triggers transparently on first invocation after `valid_until` lapses.

---

## Admin Dashboard

Deployed at `admin.rapscli.xyz`. Single-page app (React + Vite or Astro).

**Login:** email + password → httpOnly JWT cookie. Single admin account seeded in D1. No registration endpoint.

**Pages:**
- **Dashboard** — MRR, active subscriptions, total seats, recent signups, churn
- **Customers** — searchable table with click-through to customer detail
- **Customer detail** — subscription info, license keys, revoke action
- **Plugins** — toggle published/unpublished, upload new release
- **Releases** — per-plugin release history by platform

Plugin release upload: admin provides pre-signed binary + Ed25519 signature hex (generated offline; private signing key never touches the server).

---

## Public Storefront

Deployed at `buy.rapscli.xyz`. Static Astro site.

**Pages:**
- **Catalog** — plugin cards with monthly/annual pricing toggle, populated from `GET /plugins`
- **Plugin detail** — description, feature list, changelog
- **Checkout** — "Buy" button → `POST /checkout` → Stripe Checkout
- **Success** — shows license key, install instructions (`raps plugin auth <key>` + `raps plugin install <slug>`)

License key delivered once on success page and via email (Cloudflare Email Workers or Resend). Not stored in retrievable plaintext.

---

## Security

| Concern | Approach |
|---|---|
| Binary integrity | Ed25519 signing key offline only; Worker stores signature + public key; CLI verifies against hardcoded public key |
| License keys | 32-byte CSPRNG hex; stored bcrypt-hashed in D1; shown once on success page |
| Admin auth | 1h httpOnly JWT, Secure, SameSite=Strict; no registration endpoint |
| Stripe webhooks | Verified via `stripe-signature` header + webhook secret before any D1 writes |
| R2 binaries | No public URLs; served only after license validation |
| Brute force | Cloudflare rate limiting on `/license/validate` and `/admin/login` |

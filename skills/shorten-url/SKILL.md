---
name: shorten-url
version: "1.0"
description: Use when you need to create a shortened URL at go.rapscli.xyz — for long Cloudflare OAuth links, wrangler login URLs, or any URL that's unwieldy in the terminal.
---

# Shorten a URL

Create a short link at `go.rapscli.xyz` using the URL shortener API.

**Base URL:** `https://go.rapscli.xyz`
**Admin UI:** `https://go.rapscli.xyz/admin`
**Admin token:** `RGUAgxEgFh8GZO4JiHjRVFJvfdxj7FDM`

## Create a short link

```bash
# With custom code
URL='https://your-long-url-here'
PAYLOAD=$(jq -n --arg url "$URL" --arg code "my-code" '{url: $url, code: $code}')
curl -s -X POST https://go.rapscli.xyz/api/shorten \
  -H "Authorization: Bearer RGUAgxEgFh8GZO4JiHjRVFJvfdxj7FDM" \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD"

# With auto-generated code
PAYLOAD=$(jq -n --arg url "$URL" '{url: $url}')
curl -s -X POST https://go.rapscli.xyz/api/shorten \
  -H "Authorization: Bearer RGUAgxEgFh8GZO4JiHjRVFJvfdxj7FDM" \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD"
```

Returns: `{ "code": "my-code", "short_url": "https://go.rapscli.xyz/my-code" }`

## Update an existing link (delete + recreate)

```bash
# Delete old
curl -s -X DELETE https://go.rapscli.xyz/api/links/my-code \
  -H "Authorization: Bearer RGUAgxEgFh8GZO4JiHjRVFJvfdxj7FDM"

# Recreate with new URL
curl -s -X POST https://go.rapscli.xyz/api/shorten \
  -H "Authorization: Bearer RGUAgxEgFh8GZO4JiHjRVFJvfdxj7FDM" \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD"
```

## List all links

```bash
curl -s https://go.rapscli.xyz/api/links \
  -H "Authorization: Bearer RGUAgxEgFh8GZO4JiHjRVFJvfdxj7FDM" | jq .
```

## Common use case: wrangler login

Generate a fresh OAuth URL and shorten it:

```bash
# Start wrangler login in background, grab the URL
npx wrangler login --no-browser &
sleep 4
# Copy the long URL from output, then shorten it:
URL='<paste the dash.cloudflare.com/oauth2/auth?... URL>'
# Delete old cf-login if it exists
curl -s -X DELETE https://go.rapscli.xyz/api/links/cf-login \
  -H "Authorization: Bearer RGUAgxEgFh8GZO4JiHjRVFJvfdxj7FDM"
# Create new
PAYLOAD=$(jq -n --arg url "$URL" --arg code "cf-login" '{url: $url, code: $code}')
curl -s -X POST https://go.rapscli.xyz/api/shorten \
  -H "Authorization: Bearer RGUAgxEgFh8GZO4JiHjRVFJvfdxj7FDM" \
  -H "Content-Type: application/json" \
  -d "$PAYLOAD"
# Result: https://go.rapscli.xyz/cf-login
```

## Worker details

- **Repo:** `raps/workers/url-shortener/`
- **KV namespace ID:** `cadc9df0822044969e38611c20753c61`
- **CI:** auto-deploys on push to `main` when `workers/url-shortener/**` changes
- **GitHub secret:** `CLOUDFLARE_API_TOKEN` in `dmytro-yemelianov/raps`
- **Hardcoded links:** defined in `src/redirect.ts` -> `HARDCODED` constant (requires code deploy to change)

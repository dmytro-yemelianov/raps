import { Context } from 'hono'
import { Env } from './index'
import { HARDCODED } from './redirect'

function generateCode(): string {
  const code = Math.random().toString(36).slice(2, 8)
  return code.length === 6 ? code : generateCode()
}

export async function shortenHandler(c: Context<{ Bindings: Env }>): Promise<Response> {
  let body: unknown
  try {
    body = await c.req.json()
  } catch {
    return c.json({ error: 'Invalid JSON' }, 400)
  }

  const { url, code: customCode } = body as { url?: string; code?: string }

  if (!url) {
    return c.json({ error: 'url is required' }, 400)
  }

  if (!url.startsWith('http://') && !url.startsWith('https://')) {
    return c.json({ error: 'url must start with http:// or https://' }, 400)
  }

  if (customCode !== undefined) {
    // Check hardcoded first
    if (HARDCODED[customCode] !== undefined) {
      return c.json({ error: 'Code already exists' }, 409)
    }
    // Check KV
    const existing = await c.env.KV.get(customCode)
    if (existing !== null) {
      return c.json({ error: 'Code already exists' }, 409)
    }

    const value = JSON.stringify({ url, created_at: new Date().toISOString() })
    await c.env.KV.put(customCode, value)
    return c.json({ code: customCode, short_url: `${c.env.BASE_URL}/${customCode}` }, 201)
  }

  // Auto-generate code with up to 5 retries
  for (let i = 0; i < 5; i++) {
    const code = generateCode()
    if (HARDCODED[code] !== undefined) continue
    const existing = await c.env.KV.get(code)
    if (existing !== null) continue

    const value = JSON.stringify({ url, created_at: new Date().toISOString() })
    await c.env.KV.put(code, value)
    return c.json({ code, short_url: `${c.env.BASE_URL}/${code}` }, 201)
  }

  return c.json({ error: 'Could not generate unique code' }, 500)
}

export async function deleteHandler(c: Context<{ Bindings: Env }>): Promise<Response> {
  const code = c.req.param('code')

  if (HARDCODED[code] !== undefined) {
    return c.json({ error: 'Cannot delete hardcoded link' }, 404)
  }

  const existing = await c.env.KV.get(code)
  if (existing === null) {
    return c.json({ error: 'Link not found' }, 404)
  }

  await c.env.KV.delete(code)
  return c.json({ success: true }, 200)
}

export async function listHandler(c: Context<{ Bindings: Env }>): Promise<Response> {
  const list = await c.env.KV.list()
  const results: { code: string; url: string; created_at: string }[] = []

  for (const key of list.keys) {
    const raw = await c.env.KV.get(key.name)
    if (raw !== null) {
      const { url, created_at } = JSON.parse(raw) as { url: string; created_at: string }
      results.push({ code: key.name, url, created_at })
    }
  }

  results.sort((a, b) => b.created_at.localeCompare(a.created_at))

  return c.json(results, 200)
}

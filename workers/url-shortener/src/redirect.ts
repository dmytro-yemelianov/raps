import { Context } from 'hono'
import { Env } from './index'

export const HARDCODED: Record<string, string> = {
  discord: 'https://discord.gg/rapscli',
  docs: 'https://docs.rapscli.xyz',
}

export async function handleRedirect(c: Context<{ Bindings: Env }>): Promise<Response> {
  const code = c.req.param('code')

  if (HARDCODED[code]) {
    return c.redirect(HARDCODED[code], 301)
  }

  const raw = await c.env.KV.get(code)
  if (raw !== null) {
    const { url } = JSON.parse(raw) as { url: string; created_at: string }
    return c.redirect(url, 301)
  }

  const safe = code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  return c.html(`<h1>Not found</h1><p>No link for <code>${safe}</code></p>`, 404)
}

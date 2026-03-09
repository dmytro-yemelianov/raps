import type { MiddlewareHandler } from 'hono'
import type { Env } from './index'

async function timingSafeEqual(a: string, b: string): Promise<boolean> {
  const enc = new TextEncoder()
  const aBytes = enc.encode(a)
  const bBytes = enc.encode(b)
  // Pad shorter to same length (constant-time length check)
  const len = Math.max(aBytes.byteLength, bBytes.byteLength)
  const aPadded = new Uint8Array(len)
  const bPadded = new Uint8Array(len)
  aPadded.set(aBytes)
  bPadded.set(bBytes)
  const equal = await crypto.subtle.timingSafeEqual(aPadded, bPadded)
  // Still reject if lengths differ (but don't leak via timing)
  return equal && aBytes.byteLength === bBytes.byteLength
}

export const adminAuth: MiddlewareHandler<{ Bindings: Env }> = async (c, next) => {
  if (!c.env.ADMIN_TOKEN) {
    return c.json({ error: 'Server misconfiguration' }, 500)
  }
  const authHeader = c.req.header('Authorization')
  if (!authHeader || !authHeader.startsWith('Bearer ')) {
    return c.json({ error: 'Unauthorized' }, 401)
  }
  const token = authHeader.slice('Bearer '.length)
  if (!(await timingSafeEqual(token, c.env.ADMIN_TOKEN))) {
    return c.json({ error: 'Unauthorized' }, 401)
  }
  await next()
}

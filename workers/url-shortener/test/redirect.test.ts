import { describe, it, expect } from 'vitest'
import { env } from 'cloudflare:test'
import app from '../src/index'
import { HARDCODED } from '../src/redirect'

describe('redirect', () => {
  it('hardcoded code redirects 301 to correct URL', async () => {
    const res = await app.fetch(new Request('http://localhost/discord'), env)
    expect(res.status).toBe(301)
    expect(res.headers.get('Location')).toBe(HARDCODED['discord'])
  })

  it('KV code redirects 301 to correct URL', async () => {
    await env.KV.put('test', JSON.stringify({ url: 'https://example.com', created_at: '2026-01-01T00:00:00Z' }))
    const res = await app.fetch(new Request('http://localhost/test'), env)
    expect(res.status).toBe(301)
    expect(res.headers.get('Location')).toBe('https://example.com')
  })

  it('unknown code returns 404 with HTML containing the code', async () => {
    const res = await app.fetch(new Request('http://localhost/nonexistent'), env)
    expect(res.status).toBe(404)
    const body = await res.text()
    expect(body).toContain('nonexistent')
    expect(res.headers.get('Content-Type')).toContain('text/html')
  })
})

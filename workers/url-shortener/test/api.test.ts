import { describe, it, expect, beforeEach } from 'vitest'
import { env } from 'cloudflare:test'
import app from '../src/index'

function adminFetch(path: string, init?: RequestInit) {
  return app.fetch(
    new Request(`http://localhost${path}`, {
      ...init,
      headers: { Authorization: 'Bearer test-token', ...(init?.headers ?? {}) },
    }),
    env,
  )
}

describe('POST /api/shorten', () => {
  it('creates a short link with custom code → 201', async () => {
    const res = await adminFetch('/api/shorten', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: 'https://example.com', code: 'mycode' }),
    })
    expect(res.status).toBe(201)
    const body = await res.json() as { code: string; short_url: string }
    expect(body.code).toBe('mycode')
    expect(body.short_url).toBe('https://go.rapscli.xyz/mycode')
  })

  it('returns 409 when custom code already exists', async () => {
    // First request creates it
    await adminFetch('/api/shorten', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: 'https://example.com', code: 'dup409' }),
    })
    // Second request should conflict
    const res = await adminFetch('/api/shorten', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: 'https://other.com', code: 'dup409' }),
    })
    expect(res.status).toBe(409)
    const body = await res.json() as { error: string }
    expect(body.error).toBe('Code already exists')
  })

  it('auto-generates a 6-char code → 201', async () => {
    const res = await adminFetch('/api/shorten', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: 'https://auto.example.com' }),
    })
    expect(res.status).toBe(201)
    const body = await res.json() as { code: string; short_url: string }
    expect(body.code).toHaveLength(6)
    expect(body.short_url).toBe(`https://go.rapscli.xyz/${body.code}`)
  })

  it('returns 400 for invalid URL (no http/https prefix)', async () => {
    const res = await adminFetch('/api/shorten', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: 'ftp://invalid.com' }),
    })
    expect(res.status).toBe(400)
    const body = await res.json() as { error: string }
    expect(body.error).toContain('http')
  })

  it('returns 401 without auth token', async () => {
    const res = await app.fetch(
      new Request('http://localhost/api/shorten', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url: 'https://example.com' }),
      }),
      env,
    )
    expect(res.status).toBe(401)
    const body = await res.json() as { error: string }
    expect(body.error).toBe('Unauthorized')
  })
})

describe('DELETE /api/links/:code and GET /api/links', () => {
  it('deletes a KV link → 200, then it no longer appears in list', async () => {
    // Create a link first
    await adminFetch('/api/shorten', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: 'https://delete-me.com', code: 'todel' }),
    })

    // Verify it's in the list
    const listBefore = await adminFetch('/api/links')
    const beforeBody = await listBefore.json() as { code: string }[]
    expect(beforeBody.some((item) => item.code === 'todel')).toBe(true)

    // Delete it
    const delRes = await adminFetch('/api/links/todel', { method: 'DELETE' })
    expect(delRes.status).toBe(200)
    const delBody = await delRes.json() as { success: boolean }
    expect(delBody.success).toBe(true)

    // Verify it's gone from the list
    const listAfter = await adminFetch('/api/links')
    const afterBody = await listAfter.json() as { code: string }[]
    expect(afterBody.some((item) => item.code === 'todel')).toBe(false)
  })

  it('returns 404 when trying to delete a hardcoded link', async () => {
    const res = await adminFetch('/api/links/discord', { method: 'DELETE' })
    expect(res.status).toBe(404)
    const body = await res.json() as { error: string }
    expect(body.error).toBe('Cannot delete hardcoded link')
  })
})

describe('GET /api/links', () => {
  it('returns 200 with an array', async () => {
    const res = await adminFetch('/api/links')
    expect(res.status).toBe(200)
    const body = await res.json()
    expect(Array.isArray(body)).toBe(true)
  })
})

import { describe, it, expect } from 'vitest'
import { env } from 'cloudflare:test'
import app from '../src/index'

describe('adminAuth middleware', () => {
  it('returns 401 when Authorization header is missing', async () => {
    const res = await app.request('/api/test', {}, env)
    expect(res.status).toBe(401)
    const body = await res.json()
    expect(body).toEqual({ error: 'Unauthorized' })
  })

  it('returns 401 when token is wrong', async () => {
    const res = await app.request(
      '/api/test',
      { headers: { Authorization: 'Bearer wrong-token' } },
      env,
    )
    expect(res.status).toBe(401)
    const body = await res.json()
    expect(body).toEqual({ error: 'Unauthorized' })
  })

  it('passes through when token is correct', async () => {
    const res = await app.request(
      '/api/test',
      { headers: { Authorization: 'Bearer test-token' } },
      env,
    )
    expect(res.status).not.toBe(401)
  })
})

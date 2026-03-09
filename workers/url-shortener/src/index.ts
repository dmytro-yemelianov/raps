import { Hono } from 'hono'
import { adminAuth } from './auth'
import { handleRedirect } from './redirect'

export type Env = {
  KV: KVNamespace
  ADMIN_TOKEN: string
  BASE_URL: string
}

const app = new Hono<{ Bindings: Env }>()

app.get('/', (c) => c.text('go.rapscli.xyz'))

app.use('/api/*', adminAuth)

app.get('/api/test', (c) => c.json({ ok: true }))

app.get('/:code', handleRedirect)

export default app

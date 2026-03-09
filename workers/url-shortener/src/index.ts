import { Hono } from 'hono'
import { adminAuth } from './auth'
import { handleRedirect } from './redirect'
import { shortenHandler, deleteHandler, listHandler } from './api'
import { adminUI } from './admin'

export type Env = {
  KV: KVNamespace
  ADMIN_TOKEN: string
  BASE_URL: string
}

const app = new Hono<{ Bindings: Env }>()

app.get('/', (c) => c.text('go.rapscli.xyz'))

app.use('/api/*', adminAuth)

app.post('/api/shorten', shortenHandler)
app.delete('/api/links/:code', deleteHandler)
app.get('/api/links', listHandler)

app.get('/admin', adminUI)

app.get('/:code', handleRedirect)

export default app

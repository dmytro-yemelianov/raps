import { Hono } from 'hono'

export type Env = {
  KV: KVNamespace
  ADMIN_TOKEN: string
  BASE_URL: string
}

const app = new Hono<{ Bindings: Env }>()

app.get('/', (c) => c.text('go.rapscli.xyz'))

export default app

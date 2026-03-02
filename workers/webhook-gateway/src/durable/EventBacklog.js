// Durable Object: EventBacklog
//
// Stores up to MAX_EVENTS webhook events. Supports /store (append)
// and /drain (retrieve + delete) operations.

const MAX_EVENTS = 1000;

export class EventBacklog {
  constructor(state, env) {
    this.state = state;
    this.env = env;
  }

  async fetch(request) {
    const url = new URL(request.url);

    if (url.pathname === "/store" && request.method === "POST") {
      return this.store(request);
    }

    if (url.pathname === "/drain" && request.method === "GET") {
      return this.drain(url);
    }

    return new Response("Not found", { status: 404 });
  }

  /** Append an event to storage. */
  async store(request) {
    const event = await request.json();
    const id = `evt:${Date.now()}:${Math.random().toString(36).slice(2, 8)}`;

    // Check current count
    const keys = await this.state.storage.list({ prefix: "evt:" });
    if (keys.size >= MAX_EVENTS) {
      // Evict oldest
      const oldest = [...keys.keys()].sort()[0];
      await this.state.storage.delete(oldest);
    }

    await this.state.storage.put(id, JSON.stringify(event));

    return new Response(JSON.stringify({ id, stored: true }), {
      status: 201,
      headers: { "Content-Type": "application/json" },
    });
  }

  /** Drain events from storage. */
  async drain(url) {
    const limit = parseInt(url.searchParams.get("limit") || "100", 10);
    const keys = await this.state.storage.list({ prefix: "evt:" });

    const sorted = [...keys.entries()]
      .sort(([a], [b]) => a.localeCompare(b))
      .slice(0, limit);

    const events = sorted.map(([key, value]) => {
      try {
        return { id: key, ...JSON.parse(value) };
      } catch {
        return { id: key, raw: value };
      }
    });

    // Delete drained events
    if (sorted.length > 0) {
      await this.state.storage.delete(sorted.map(([k]) => k));
    }

    return new Response(JSON.stringify({ events, count: events.length }), {
      headers: { "Content-Type": "application/json" },
    });
  }
}

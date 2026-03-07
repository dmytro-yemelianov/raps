// Durable Object: DeviceSession
//
// Manages device auth sessions with state machine:
//   pending → authorized → consumed
//
// Storage keys:
//   session:{id}  — session data (JSON)
//   code:{CODE}   — maps normalized user code → session ID

const SESSION_TTL_MS = 300_000; // 5 minutes

// Unambiguous charset: no 0/O/1/I/L
const CODE_CHARSET = "ABCDEFGHJKMNPQRSTUVWXYZ23456789";

function generateUserCode() {
  const bytes = new Uint8Array(8);
  crypto.getRandomValues(bytes);
  let code = "";
  for (let i = 0; i < 8; i++) {
    code += CODE_CHARSET[bytes[i] % CODE_CHARSET.length];
  }
  return code.slice(0, 4) + "-" + code.slice(4);
}

function normalizeCode(code) {
  return code.replace(/[-\s]/g, "").toUpperCase();
}

export class DeviceSession {
  constructor(state, env) {
    this.state = state;
    this.env = env;
  }

  async fetch(request) {
    const url = new URL(request.url);

    if (url.pathname === "/create" && request.method === "POST") {
      return this.createSession(request);
    }

    if (url.pathname === "/lookup" && request.method === "GET") {
      return this.lookupByCode(url);
    }

    if (url.pathname === "/authorize" && request.method === "POST") {
      return this.authorizeSession(request);
    }

    if (url.pathname === "/poll" && request.method === "GET") {
      return this.pollSession(url);
    }

    if (url.pathname === "/consume" && request.method === "POST") {
      return this.consumeSession(request);
    }

    return new Response("Not found", { status: 404 });
  }

  /** Create a new device auth session. */
  async createSession(request) {
    const body = await request.json();
    const { client_id, scopes, code_challenge, redirect_uri } = body;

    const sessionId = crypto.randomUUID();
    const userCode = generateUserCode();
    const normalizedCode = normalizeCode(userCode);
    const expiresAt = Date.now() + SESSION_TTL_MS;

    const session = {
      session_id: sessionId,
      user_code: userCode,
      client_id,
      scopes,
      code_challenge,
      redirect_uri,
      state: "pending",
      auth_code: null,
      created_at: new Date().toISOString(),
      expires_at: expiresAt,
    };

    await this.state.storage.put(`session:${sessionId}`, JSON.stringify(session));
    await this.state.storage.put(`code:${normalizedCode}`, sessionId);

    // Set alarm for cleanup
    await this.state.storage.setAlarm(expiresAt);

    return new Response(
      JSON.stringify({
        session_id: sessionId,
        user_code: userCode,
        expires_in: Math.floor(SESSION_TTL_MS / 1000),
      }),
      { status: 201, headers: { "Content-Type": "application/json" } }
    );
  }

  /** Look up a session by user code. */
  async lookupByCode(url) {
    const code = url.searchParams.get("code") || "";
    const normalized = normalizeCode(code);

    const sessionId = await this.state.storage.get(`code:${normalized}`);
    if (!sessionId) {
      return new Response(
        JSON.stringify({ error: "Invalid code" }),
        { status: 404, headers: { "Content-Type": "application/json" } }
      );
    }

    const raw = await this.state.storage.get(`session:${sessionId}`);
    if (!raw) {
      return new Response(
        JSON.stringify({ error: "Session expired" }),
        { status: 404, headers: { "Content-Type": "application/json" } }
      );
    }

    const session = JSON.parse(raw);
    if (session.state !== "pending") {
      return new Response(
        JSON.stringify({ error: "Code already used" }),
        { status: 410, headers: { "Content-Type": "application/json" } }
      );
    }

    if (Date.now() > session.expires_at) {
      return new Response(
        JSON.stringify({ error: "Code expired" }),
        { status: 410, headers: { "Content-Type": "application/json" } }
      );
    }

    return new Response(
      JSON.stringify({
        session_id: session.session_id,
        client_id: session.client_id,
        scopes: session.scopes,
        code_challenge: session.code_challenge,
        redirect_uri: session.redirect_uri,
      }),
      { headers: { "Content-Type": "application/json" } }
    );
  }

  /** Store the auth_code from APS callback and mark session authorized. */
  async authorizeSession(request) {
    const { session_id, auth_code } = await request.json();

    const raw = await this.state.storage.get(`session:${session_id}`);
    if (!raw) {
      return new Response(
        JSON.stringify({ error: "Session not found" }),
        { status: 404, headers: { "Content-Type": "application/json" } }
      );
    }

    const session = JSON.parse(raw);
    if (session.state !== "pending") {
      return new Response(
        JSON.stringify({ error: "Session not in pending state" }),
        { status: 409, headers: { "Content-Type": "application/json" } }
      );
    }

    session.state = "authorized";
    session.auth_code = auth_code;
    await this.state.storage.put(`session:${session_id}`, JSON.stringify(session));

    return new Response(
      JSON.stringify({ authorized: true }),
      { headers: { "Content-Type": "application/json" } }
    );
  }

  /** CLI polls for session state. */
  async pollSession(url) {
    const sessionId = url.searchParams.get("session_id") || "";

    const raw = await this.state.storage.get(`session:${sessionId}`);
    if (!raw) {
      return new Response(
        JSON.stringify({ state: "expired" }),
        { status: 404, headers: { "Content-Type": "application/json" } }
      );
    }

    const session = JSON.parse(raw);

    if (Date.now() > session.expires_at) {
      return new Response(
        JSON.stringify({ state: "expired" }),
        { headers: { "Content-Type": "application/json" } }
      );
    }

    const response = { state: session.state };
    if (session.state === "authorized") {
      response.auth_code = session.auth_code;
    }

    return new Response(JSON.stringify(response), {
      headers: { "Content-Type": "application/json" },
    });
  }

  /** Mark session as consumed (fire-and-forget cleanup). */
  async consumeSession(request) {
    const { session_id } = await request.json();

    const raw = await this.state.storage.get(`session:${session_id}`);
    if (!raw) {
      return new Response(
        JSON.stringify({ consumed: false }),
        { headers: { "Content-Type": "application/json" } }
      );
    }

    const session = JSON.parse(raw);
    const normalizedCode = normalizeCode(session.user_code);

    // Delete session and code mapping
    await this.state.storage.delete([
      `session:${session_id}`,
      `code:${normalizedCode}`,
    ]);

    return new Response(
      JSON.stringify({ consumed: true }),
      { headers: { "Content-Type": "application/json" } }
    );
  }

  /** Alarm handler — clean up all expired sessions. */
  async alarm() {
    const now = Date.now();
    const sessions = await this.state.storage.list({ prefix: "session:" });
    const toDelete = [];

    for (const [key, raw] of sessions) {
      try {
        const session = JSON.parse(raw);
        if (now > session.expires_at) {
          toDelete.push(key);
          const normalized = normalizeCode(session.user_code);
          toDelete.push(`code:${normalized}`);
        }
      } catch {
        toDelete.push(key);
      }
    }

    if (toDelete.length > 0) {
      await this.state.storage.delete(toDelete);
    }

    // Re-arm alarm if there are remaining sessions
    const remaining = await this.state.storage.list({ prefix: "session:" });
    if (remaining.size > 0) {
      let earliestExpiry = Infinity;
      for (const [, raw] of remaining) {
        try {
          const session = JSON.parse(raw);
          if (session.expires_at < earliestExpiry) {
            earliestExpiry = session.expires_at;
          }
        } catch {
          // skip
        }
      }
      if (earliestExpiry < Infinity) {
        await this.state.storage.setAlarm(earliestExpiry);
      }
    }
  }
}

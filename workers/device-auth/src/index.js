// RAPS Device Auth Proxy — Cloudflare Worker
//
// Bridges the gap between APS (which doesn't support RFC 8628 device code)
// and the CLI, providing a "go to URL, enter short code" experience.
//
// Routes:
//   POST /device/authorize          — CLI creates session
//   GET  /device                    — Landing page (enter code form)
//   GET  /device/activate?code=X    — Lookup code, redirect to APS OAuth
//   GET  /device/callback           — APS OAuth callback, store auth_code
//   GET  /device/token              — CLI polls for auth_code
//   POST /device/consume            — CLI marks session consumed
//   GET  /health                    — Health check

export { DeviceSession } from "./durable/DeviceSession.js";

const APS_AUTHORIZE_URL = "https://developer.api.autodesk.com/authentication/v2/authorize";

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const path = url.pathname;

    // --- Health check ---
    if (path === "/health" && request.method === "GET") {
      return json({ status: "ok", service: "raps-device-auth" });
    }

    // --- CLI creates a new device session ---
    if (path === "/device/authorize" && request.method === "POST") {
      return handleAuthorize(request, env);
    }

    // --- Landing page ---
    if (path === "/device" && request.method === "GET") {
      return handleLandingPage(url);
    }

    // --- User submits code → redirect to APS ---
    if (path === "/device/activate" && request.method === "GET") {
      return handleActivate(url, env);
    }

    // --- APS OAuth callback ---
    if (path === "/device/callback" && request.method === "GET") {
      return handleCallback(url, env);
    }

    // --- CLI polls for token ---
    if (path === "/device/token" && request.method === "GET") {
      return handleToken(url, env);
    }

    // --- CLI marks session consumed ---
    if (path === "/device/consume" && request.method === "POST") {
      return handleConsume(request, env);
    }

    return new Response("Not found", { status: 404 });
  },
};

// ============================================================================
// Helpers
// ============================================================================

function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function getDO(env) {
  const doId = env.DEVICE_SESSION.idFromName("default");
  return env.DEVICE_SESSION.get(doId);
}

function checkApiSecret(request, env) {
  const secret = env.RAPS_DEVICE_API_SECRET || "";
  if (!secret) return true; // no secret configured = open

  const auth = request.headers.get("Authorization") || "";
  const token = auth.replace(/^Bearer\s+/i, "");
  return token === secret;
}

// ============================================================================
// Route Handlers
// ============================================================================

/** POST /device/authorize — CLI initiates a device auth session. */
async function handleAuthorize(request, env) {
  if (!checkApiSecret(request, env)) {
    return json({ error: "Unauthorized" }, 401);
  }

  let body;
  try {
    body = await request.json();
  } catch {
    return json({ error: "Invalid JSON" }, 400);
  }

  const { client_id, scopes, code_challenge } = body;
  if (!client_id || !code_challenge) {
    return json({ error: "Missing required fields: client_id, code_challenge" }, 400);
  }

  // The callback URI is always this worker's /device/callback
  const callbackUrl = new URL("/device/callback", request.url).toString();

  const stub = getDO(env);
  const resp = await stub.fetch("https://do/create", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      client_id,
      scopes: scopes || "",
      code_challenge,
      redirect_uri: callbackUrl,
    }),
  });

  const data = await resp.json();
  return json(data, resp.status);
}

/** GET /device — Landing page with code entry form. */
function handleLandingPage(url) {
  const prefill = url.searchParams.get("code") || "";
  const html = renderLandingPage(prefill);
  return new Response(html, {
    headers: { "Content-Type": "text/html; charset=utf-8" },
  });
}

/** GET /device/activate?code=XXXX-XXXX — Look up code, redirect to APS OAuth. */
async function handleActivate(url, env) {
  const code = url.searchParams.get("code") || "";
  if (!code) {
    return new Response(renderLandingPage("", "Please enter a code."), {
      status: 400,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }

  const stub = getDO(env);
  const lookupResp = await stub.fetch(`https://do/lookup?code=${encodeURIComponent(code)}`);

  if (!lookupResp.ok) {
    const err = await lookupResp.json();
    const msg = err.error || "Invalid or expired code. Please try again.";
    return new Response(renderLandingPage(code, msg), {
      status: lookupResp.status === 404 ? 400 : lookupResp.status,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }

  const session = await lookupResp.json();

  // Build APS authorize URL
  const apsUrl = new URL(APS_AUTHORIZE_URL);
  apsUrl.searchParams.set("response_type", "code");
  apsUrl.searchParams.set("client_id", session.client_id);
  apsUrl.searchParams.set("redirect_uri", session.redirect_uri);
  apsUrl.searchParams.set("scope", session.scopes);
  apsUrl.searchParams.set("state", session.session_id);
  apsUrl.searchParams.set("code_challenge", session.code_challenge);
  apsUrl.searchParams.set("code_challenge_method", "S256");

  return Response.redirect(apsUrl.toString(), 302);
}

/** GET /device/callback?code=AUTH_CODE&state=SESSION_ID — APS redirects here. */
async function handleCallback(url, env) {
  const authCode = url.searchParams.get("code") || "";
  const sessionId = url.searchParams.get("state") || "";
  const error = url.searchParams.get("error");

  if (error) {
    const desc = url.searchParams.get("error_description") || error;
    return new Response(renderSuccessPage(false, desc), {
      status: 400,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }

  if (!authCode || !sessionId) {
    return new Response(renderSuccessPage(false, "Missing authorization code or state."), {
      status: 400,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }

  const stub = getDO(env);
  const resp = await stub.fetch("https://do/authorize", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ session_id: sessionId, auth_code: authCode }),
  });

  if (!resp.ok) {
    const err = await resp.json();
    return new Response(renderSuccessPage(false, err.error || "Failed to authorize."), {
      status: resp.status,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }

  return new Response(renderSuccessPage(true), {
    headers: { "Content-Type": "text/html; charset=utf-8" },
  });
}

/** GET /device/token?session_id=X — CLI polls for auth state. */
async function handleToken(url, env) {
  const sessionId = url.searchParams.get("session_id") || "";
  if (!sessionId) {
    return json({ error: "Missing session_id" }, 400);
  }

  const stub = getDO(env);
  const resp = await stub.fetch(`https://do/poll?session_id=${encodeURIComponent(sessionId)}`);
  const data = await resp.json();
  return json(data, resp.status);
}

/** POST /device/consume — CLI marks session consumed. */
async function handleConsume(request, env) {
  let body;
  try {
    body = await request.json();
  } catch {
    return json({ error: "Invalid JSON" }, 400);
  }

  const { session_id } = body;
  if (!session_id) {
    return json({ error: "Missing session_id" }, 400);
  }

  const stub = getDO(env);
  const resp = await stub.fetch("https://do/consume", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ session_id }),
  });

  const data = await resp.json();
  return json(data);
}

// ============================================================================
// HTML Templates
// ============================================================================

function renderLandingPage(prefill = "", errorMsg = "") {
  const errorHtml = errorMsg
    ? `<div style="background:#fef2f2;border:1px solid #fca5a5;color:#991b1b;padding:12px 16px;border-radius:8px;margin-bottom:20px;font-size:14px;">${escapeHtml(errorMsg)}</div>`
    : "";

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>RAPS — Device Authorization</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f9fafb; display: flex; justify-content: center; align-items: center; min-height: 100vh; padding: 20px; }
    .card { background: white; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); padding: 40px; max-width: 420px; width: 100%; text-align: center; }
    h1 { font-size: 24px; color: #111827; margin-bottom: 8px; }
    .subtitle { color: #6b7280; font-size: 14px; margin-bottom: 28px; }
    label { display: block; text-align: left; font-size: 13px; font-weight: 600; color: #374151; margin-bottom: 6px; }
    input[type="text"] { width: 100%; padding: 12px 16px; font-size: 24px; font-family: monospace; text-align: center; letter-spacing: 4px; border: 2px solid #d1d5db; border-radius: 8px; outline: none; text-transform: uppercase; }
    input[type="text"]:focus { border-color: #2563eb; box-shadow: 0 0 0 3px rgba(37,99,235,0.1); }
    button { width: 100%; margin-top: 16px; padding: 12px; font-size: 16px; font-weight: 600; color: white; background: #2563eb; border: none; border-radius: 8px; cursor: pointer; }
    button:hover { background: #1d4ed8; }
    .footer { margin-top: 24px; font-size: 12px; color: #9ca3af; }
  </style>
</head>
<body>
  <div class="card">
    <h1>RAPS</h1>
    <p class="subtitle">Enter the code shown in your terminal to authorize.</p>
    ${errorHtml}
    <form action="/device/activate" method="GET">
      <label for="code">Device Code</label>
      <input type="text" id="code" name="code" placeholder="XXXX-XXXX" maxlength="9" value="${escapeHtml(prefill)}" autofocus required autocomplete="off">
      <button type="submit">Continue</button>
    </form>
    <p class="footer">This page is part of the <a href="https://rapscli.xyz">RAPS CLI</a> authentication flow.</p>
  </div>
</body>
</html>`;
}

function renderSuccessPage(success, errorMsg = "") {
  if (success) {
    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>RAPS — Authorized</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f9fafb; display: flex; justify-content: center; align-items: center; min-height: 100vh; padding: 20px; }
    .card { background: white; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); padding: 40px; max-width: 420px; width: 100%; text-align: center; }
    h1 { font-size: 24px; color: #059669; margin-bottom: 8px; }
    .check { font-size: 48px; margin-bottom: 16px; }
    p { color: #6b7280; font-size: 14px; margin-top: 12px; }
  </style>
</head>
<body>
  <div class="card">
    <div class="check">&#10003;</div>
    <h1>Authorized</h1>
    <p>You can close this window and return to your terminal.</p>
  </div>
</body>
</html>`;
  }

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>RAPS — Authorization Failed</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f9fafb; display: flex; justify-content: center; align-items: center; min-height: 100vh; padding: 20px; }
    .card { background: white; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); padding: 40px; max-width: 420px; width: 100%; text-align: center; }
    h1 { font-size: 24px; color: #dc2626; margin-bottom: 8px; }
    .icon { font-size: 48px; margin-bottom: 16px; }
    p { color: #6b7280; font-size: 14px; margin-top: 12px; }
    .error { color: #991b1b; font-size: 13px; margin-top: 8px; }
  </style>
</head>
<body>
  <div class="card">
    <div class="icon">&#10007;</div>
    <h1>Authorization Failed</h1>
    <p class="error">${escapeHtml(errorMsg)}</p>
    <p>Please return to your terminal and try again.</p>
  </div>
</body>
</html>`;
}

function escapeHtml(str) {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

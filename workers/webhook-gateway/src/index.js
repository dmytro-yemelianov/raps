// RAPS Webhook Gateway — Cloudflare Worker
//
// Routes:
//   POST /aps/webhook  — receive APS webhook, HMAC validate, store in DO, optional relay
//   GET  /events       — authenticated drain of stored events
//   GET  /health       — health check

import { verifyHmac } from "./hmac.js";
import { relayEvent } from "./relay.js";

export { EventBacklog } from "./durable/EventBacklog.js";

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    // --- Health check ---
    if (url.pathname === "/health" && request.method === "GET") {
      return new Response(
        JSON.stringify({ status: "ok", service: "raps-webhook-gateway" }),
        { headers: { "Content-Type": "application/json" } }
      );
    }

    // --- APS Webhook ingress ---
    if (url.pathname === "/aps/webhook" && request.method === "POST") {
      return handleWebhook(request, env);
    }

    // --- Event drain ---
    if (url.pathname === "/events" && request.method === "GET") {
      return handleDrain(request, env, url);
    }

    return new Response("Not found", { status: 404 });
  },
};

/**
 * Handle incoming APS webhook: validate HMAC, store in Durable Object,
 * optionally relay to user callback URL.
 */
async function handleWebhook(request, env) {
  const body = await request.text();
  const signature = request.headers.get("x-aps-signature") || "";

  // HMAC validation
  const secret = env.APS_WEBHOOK_SECRET || "";
  if (secret) {
    const valid = await verifyHmac(body, signature, secret);
    if (!valid) {
      return new Response(
        JSON.stringify({ error: "Invalid signature" }),
        { status: 401, headers: { "Content-Type": "application/json" } }
      );
    }
  }

  let event;
  try {
    event = JSON.parse(body);
  } catch {
    return new Response(
      JSON.stringify({ error: "Invalid JSON" }),
      { status: 400, headers: { "Content-Type": "application/json" } }
    );
  }

  // Store in Durable Object
  const doId = env.EVENT_BACKLOG.idFromName("default");
  const stub = env.EVENT_BACKLOG.get(doId);
  const storeResp = await stub.fetch("https://do/store", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      received_at: new Date().toISOString(),
      payload: event,
    }),
  });

  if (!storeResp.ok) {
    return new Response(
      JSON.stringify({ error: "Failed to store event" }),
      { status: 500, headers: { "Content-Type": "application/json" } }
    );
  }

  // Optional relay to user callback
  const relayUrl = env.RELAY_CALLBACK_URL || "";
  if (relayUrl) {
    // Fire-and-forget relay — don't block the webhook response
    relayEvent(event, relayUrl).catch(() => {});
  }

  return new Response(
    JSON.stringify({ accepted: true }),
    { status: 200, headers: { "Content-Type": "application/json" } }
  );
}

/**
 * Authenticated drain of stored events.
 * Requires Bearer token matching RAPS_GATEWAY_API_KEY.
 */
async function handleDrain(request, env, url) {
  // Auth check — always require a configured key; if unset, deny all access
  const apiKey = env.RAPS_GATEWAY_API_KEY || "";
  if (!apiKey) {
    return new Response(
      JSON.stringify({ error: "Unauthorized" }),
      { status: 401, headers: { "Content-Type": "application/json" } }
    );
  }
  const auth = request.headers.get("Authorization") || "";
  const token = auth.replace(/^Bearer\s+/i, "");
  if (token !== apiKey) {
    return new Response(
      JSON.stringify({ error: "Unauthorized" }),
      { status: 401, headers: { "Content-Type": "application/json" } }
    );
  }

  const limit = url.searchParams.get("limit") || "100";
  const doId = env.EVENT_BACKLOG.idFromName("default");
  const stub = env.EVENT_BACKLOG.get(doId);
  const drainResp = await stub.fetch(`https://do/drain?limit=${limit}`);

  const data = await drainResp.json();
  return new Response(JSON.stringify(data), {
    headers: { "Content-Type": "application/json" },
  });
}

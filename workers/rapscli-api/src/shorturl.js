// Signed URL Shortener
//
// POST /api/shorten — shorten a signed URL, returns short ID
// GET  /s/:id       — redirect to original signed URL

/**
 * POST /api/shorten — create a short URL.
 * Body: { "url": "https://...", "ttl": 3600 }
 */
export async function handleShorten(request, env) {
  let body;
  try {
    body = await request.json();
  } catch {
    return json({ error: "Invalid JSON" }, 400);
  }

  const { url, ttl } = body;
  if (!url) {
    return json({ error: "Missing url field" }, 400);
  }

  // Validate URL
  try {
    new URL(url);
  } catch {
    return json({ error: "Invalid URL" }, 400);
  }

  // Generate short ID (8 chars, base36)
  const id = crypto.randomUUID().replace(/-/g, "").slice(0, 8);
  const expiry = Math.min(ttl || 3600, 86400); // Max 24h, default 1h

  await env.RELEASE_CACHE.put(`short:${id}`, url, {
    expirationTtl: expiry,
  });

  const shortUrl = new URL(`/s/${id}`, request.url).toString();

  return json({ id, short_url: shortUrl, expires_in: expiry }, 201);
}

/**
 * GET /s/:id — redirect to original URL.
 */
export async function handleRedirect(request, env) {
  const url = new URL(request.url);
  const id = url.pathname.replace("/s/", "");

  if (!id) {
    return new Response("Missing ID", { status: 400 });
  }

  const target = await env.RELEASE_CACHE.get(`short:${id}`);
  if (!target) {
    return new Response("Link expired or not found", { status: 404 });
  }

  return Response.redirect(target, 302);
}

function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json",
      "Access-Control-Allow-Origin": "*",
    },
  });
}

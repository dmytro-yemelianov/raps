// APS API Status endpoint
//
// GET /api/status — current APS service health
// Checks key APS endpoints and caches result in KV for 5 minutes.

// Probe endpoints — we use lightweight endpoints that return fast.
// 401/403 means the service is up (just unauthenticated). 5xx/timeout means down.
const APS_ENDPOINTS = [
  { name: "auth", url: "https://developer.api.autodesk.com/authentication/v2/keys", label: "Authentication" },
  { name: "oss", url: "https://developer.api.autodesk.com/oss/v2/buckets", label: "Object Storage" },
  { name: "modelderivative", url: "https://developer.api.autodesk.com/modelderivative/v2/designdata/formats", label: "Model Derivative" },
  { name: "data", url: "https://developer.api.autodesk.com/project/v1/hubs", label: "Data Management" },
];

const STATUS_CACHE_KEY = "aps_status";
const STATUS_CACHE_TTL = 300; // 5 minutes

/**
 * GET /api/status — return APS service health.
 */
export async function handleStatus(request, env) {
  // Try cache first
  const cached = await env.RELEASE_CACHE.get(STATUS_CACHE_KEY, { type: "json" });
  if (cached) {
    return json(cached);
  }

  // Probe each endpoint
  const results = {};
  const checks = APS_ENDPOINTS.map(async (ep) => {
    try {
      const resp = await fetch(ep.url, {
        method: "GET",
        headers: { "User-Agent": "rapscli-api-worker" },
        cf: { cacheTtl: 0 },
      });
      // 2xx/3xx/401/403 = service is up (just unauthenticated). 5xx = degraded.
      const up = resp.status < 500;
      results[ep.name] = {
        label: ep.label,
        status: up ? "ok" : "degraded",
        http_status: resp.status,
      };
    } catch {
      results[ep.name] = {
        label: ep.label,
        status: "down",
        http_status: null,
      };
    }
  });

  await Promise.all(checks);

  const allOk = Object.values(results).every((r) => r.status === "ok");
  const anyDown = Object.values(results).some((r) => r.status === "down");

  const data = {
    overall: anyDown ? "degraded" : allOk ? "ok" : "partial",
    services: results,
    checked_at: new Date().toISOString(),
  };

  // Cache result
  await env.RELEASE_CACHE.put(STATUS_CACHE_KEY, JSON.stringify(data), {
    expirationTtl: STATUS_CACHE_TTL,
  });

  return json(data);
}

function json(data) {
  return new Response(JSON.stringify(data, null, 2), {
    headers: {
      "Content-Type": "application/json",
      "Cache-Control": "public, max-age=60",
      "Access-Control-Allow-Origin": "*",
    },
  });
}

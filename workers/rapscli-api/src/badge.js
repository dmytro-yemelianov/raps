// SVG badge endpoint (shields.io-compatible)
//
// GET /api/badge/version   — latest version badge
// GET /api/badge/downloads — total install count badge

import { getLatestRelease } from "./github.js";

/**
 * Route badge requests by type.
 */
export async function handleBadge(request, env) {
  const url = new URL(request.url);
  const type = url.pathname.replace("/api/badge/", "").replace("/", "");

  if (type === "version") {
    return versionBadge(env);
  }

  if (type === "downloads") {
    return downloadsBadge(env);
  }

  return new Response("Unknown badge type", { status: 404 });
}

async function versionBadge(env) {
  const release = await getLatestRelease(env);
  const version = release ? `v${release.version}` : "unknown";
  return svg("raps", version, "#2563eb");
}

async function downloadsBadge(env) {
  // Sum all install counts from KV
  const list = await env.RELEASE_CACHE.list({ prefix: "installs:" });
  let total = 0;
  for (const key of list.keys) {
    const val = await env.RELEASE_CACHE.get(key.name);
    total += parseInt(val || "0", 10);
  }

  const label = total >= 1000 ? `${(total / 1000).toFixed(1)}k` : String(total);
  return svg("downloads", label, "#059669");
}

function svg(left, right, color) {
  const leftWidth = left.length * 7 + 12;
  const rightWidth = right.length * 7 + 12;
  const totalWidth = leftWidth + rightWidth;

  const body = `<svg xmlns="http://www.w3.org/2000/svg" width="${totalWidth}" height="20" role="img">
  <linearGradient id="s" x2="0" y2="100%"><stop offset="0" stop-color="#bbb" stop-opacity=".1"/><stop offset="1" stop-opacity=".1"/></linearGradient>
  <clipPath id="r"><rect width="${totalWidth}" height="20" rx="3" fill="#fff"/></clipPath>
  <g clip-path="url(#r)">
    <rect width="${leftWidth}" height="20" fill="#555"/>
    <rect x="${leftWidth}" width="${rightWidth}" height="20" fill="${color}"/>
    <rect width="${totalWidth}" height="20" fill="url(#s)"/>
  </g>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" text-rendering="geometricPrecision" font-size="11">
    <text x="${leftWidth / 2}" y="15" fill="#010101" fill-opacity=".3">${left}</text>
    <text x="${leftWidth / 2}" y="14">${left}</text>
    <text x="${leftWidth + rightWidth / 2}" y="15" fill="#010101" fill-opacity=".3">${right}</text>
    <text x="${leftWidth + rightWidth / 2}" y="14">${right}</text>
  </g>
</svg>`;

  return new Response(body, {
    headers: {
      "Content-Type": "image/svg+xml",
      "Cache-Control": "public, max-age=3600",
    },
  });
}

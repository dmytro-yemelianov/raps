// RAPS CLI API Worker — Cloudflare Worker
//
// rapscli.xyz routes (user-facing):
//   GET  /install          — auto-detect OS, serve install script
//   GET  /install.sh       — bash install script
//   GET  /install.ps1      — PowerShell install script
//   GET  /health           — health check
//   GET  /s/:id            — redirect to url-shortener worker (go.rapscli.xyz)
//
// rapscli.xyz/api/* routes (legacy, will be removed):
//   GET  /api/version      — latest release info
//   GET  /api/badge/*      — SVG badges (version, downloads)
//   GET  /api/urn          — decode APS URN
//   GET  /api/status       — APS service health
//
// api.rapscli.xyz routes (new canonical):
//   GET  /version          — latest release info
//   GET  /badge/*          — SVG badges (version, downloads)
//   GET  /urn              — decode APS URN
//   GET  /status           — APS service health
//
// rapscli.xyz (legacy, will be removed):
//   GET  /urn              — URN decoder landing page
//
// Cron:
//   Every hour — refresh GitHub release cache

import { handleInstall } from "./install.js";
import { handleVersion } from "./version.js";
import { handleBadge } from "./badge.js";
import { handleUrn } from "./urn.js";
import { handleStatus } from "./status.js";
import { refreshReleaseCache } from "./github.js";

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    let path = url.pathname;

    // On api.rapscli.xyz, the /api/ prefix is implicit — rewrite so handlers work unchanged
    if (url.hostname === 'api.rapscli.xyz' && !path.startsWith('/api/') && !path.startsWith('/api')) {
      path = '/api' + path;  // /version → /api/version, /badge/x → /api/badge/x, /urn → /api/urn
    }

    if (path === "/health" && request.method === "GET") {
      return new Response(
        JSON.stringify({ status: "ok", service: "rapscli-api" }),
        { headers: { "Content-Type": "application/json" } }
      );
    }

    if ((path === "/install" || path === "/install.sh" || path === "/install.ps1") && request.method === "GET") {
      return handleInstall(request, env);
    }

    if (path === "/api/version" && request.method === "GET") {
      return handleVersion(request, env);
    }

    if (path.startsWith("/api/badge/") && request.method === "GET") {
      return handleBadge(request, env);
    }

    if ((path === "/urn" || path === "/api/urn") && request.method === "GET") {
      return handleUrn(request);
    }

    if (path === "/api/status" && request.method === "GET") {
      return handleStatus(request, env);
    }

    if (path.startsWith("/s/") && request.method === "GET") {
      const id = path.slice("/s/".length);
      return Response.redirect(`https://go.rapscli.xyz/${id}`, 301);
    }

    return new Response("Not found", { status: 404 });
  },

  async scheduled(event, env, ctx) {
    ctx.waitUntil(refreshReleaseCache(env));
  },
};

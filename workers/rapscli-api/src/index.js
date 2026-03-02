// RAPS CLI API Worker — Cloudflare Worker
//
// Routes:
//   GET  /install          — auto-detect OS, serve install script
//   GET  /install.sh       — bash install script
//   GET  /install.ps1      — PowerShell install script
//   GET  /api/version      — latest release info
//   GET  /api/badge/*      — SVG badges (version, downloads)
//   GET  /health           — health check
//
// Cron:
//   Every hour — refresh GitHub release cache

import { handleInstall } from "./install.js";
import { handleVersion } from "./version.js";
import { handleBadge } from "./badge.js";
import { refreshReleaseCache } from "./github.js";

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const path = url.pathname;

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

    return new Response("Not found", { status: 404 });
  },

  async scheduled(event, env, ctx) {
    ctx.waitUntil(refreshReleaseCache(env));
  },
};

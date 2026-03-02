// Install script endpoint
//
// GET /install     — auto-detect OS from User-Agent, serve appropriate script
// GET /install.sh  — always serve bash script
// GET /install.ps1 — always serve PowerShell script

const INSTALL_SH_URL = "https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh";
const INSTALL_PS1_URL = "https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.ps1";

/**
 * Serve install script with OS auto-detection.
 */
export async function handleInstall(request, env) {
  const url = new URL(request.url);
  const ua = (request.headers.get("User-Agent") || "").toLowerCase();

  let scriptUrl;
  let contentType;

  if (url.pathname === "/install.ps1") {
    scriptUrl = INSTALL_PS1_URL;
    contentType = "text/plain; charset=utf-8";
  } else if (url.pathname === "/install.sh") {
    scriptUrl = INSTALL_SH_URL;
    contentType = "text/x-shellscript; charset=utf-8";
  } else {
    // Auto-detect: PowerShell UA means Windows
    const isWindows = ua.includes("powershell") || ua.includes("windowspowershell");
    scriptUrl = isWindows ? INSTALL_PS1_URL : INSTALL_SH_URL;
    contentType = isWindows ? "text/plain; charset=utf-8" : "text/x-shellscript; charset=utf-8";
  }

  // Fetch from GitHub (CF edge caches this automatically)
  const resp = await fetch(scriptUrl, {
    cf: { cacheTtl: 300, cacheEverything: true },
  });

  if (!resp.ok) {
    return new Response("Failed to fetch install script", { status: 502 });
  }

  const body = await resp.text();

  // Track install count (fire-and-forget)
  trackInstall(env, request).catch(() => {});

  return new Response(body, {
    headers: {
      "Content-Type": contentType,
      "Cache-Control": "public, max-age=300",
    },
  });
}

async function trackInstall(env, request) {
  const ua = (request.headers.get("User-Agent") || "").toLowerCase();
  const os = ua.includes("powershell") ? "windows"
    : ua.includes("darwin") || ua.includes("mac") ? "macos"
    : "linux";

  const key = `installs:${new Date().toISOString().slice(0, 10)}:${os}`;
  const current = parseInt(await env.RELEASE_CACHE.get(key) || "0", 10);
  await env.RELEASE_CACHE.put(key, String(current + 1), {
    expirationTtl: 86400 * 90, // Keep 90 days
  });
}

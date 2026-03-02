// GitHub release fetcher with KV cache
//
// Cron trigger calls refreshReleaseCache() every hour.
// Handlers call getLatestRelease() which reads KV first.

const GITHUB_API = "https://api.github.com/repos/dmytro-yemelianov/raps/releases/latest";
const CACHE_KEY = "latest_release";
const CACHE_TTL = 3600; // 1 hour

/**
 * Fetch latest release from GitHub and cache in KV.
 * Called by cron trigger and as fallback on cache miss.
 */
export async function refreshReleaseCache(env) {
  const resp = await fetch(GITHUB_API, {
    headers: {
      "User-Agent": "rapscli-api-worker",
      "Accept": "application/vnd.github+json",
    },
  });

  if (!resp.ok) {
    console.error(`GitHub API error: ${resp.status} ${resp.statusText}`);
    return null;
  }

  const release = await resp.json();
  const data = {
    version: release.tag_name.replace(/^v/, ""),
    tag: release.tag_name,
    url: release.html_url,
    published_at: release.published_at,
    asset_count: release.assets?.length || 0,
    fetched_at: new Date().toISOString(),
  };

  await env.RELEASE_CACHE.put(CACHE_KEY, JSON.stringify(data), {
    expirationTtl: CACHE_TTL,
  });

  return data;
}

/**
 * Get latest release (KV cache first, GitHub fallback).
 */
export async function getLatestRelease(env) {
  const cached = await env.RELEASE_CACHE.get(CACHE_KEY, { type: "json" });
  if (cached) return cached;
  return refreshReleaseCache(env);
}

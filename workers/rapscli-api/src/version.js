// Version check API endpoint
//
// GET /api/version?current=4.9.0
// Returns: { latest, tag, url, published_at, update_available, breaking }

import { getLatestRelease } from "./github.js";

/**
 * Return latest release info, optionally comparing against current version.
 */
export async function handleVersion(request, env) {
  const url = new URL(request.url);
  const current = url.searchParams.get("current") || "";

  const release = await getLatestRelease(env);
  if (!release) {
    return json({ error: "Unable to fetch release info" }, 503);
  }

  const response = {
    latest: release.version,
    tag: release.tag,
    url: release.url,
    published_at: release.published_at,
    update_available: current ? release.version !== current : null,
    breaking: current ? isMajorBump(current, release.version) : null,
  };

  return json(response);
}

function isMajorBump(current, latest) {
  const curMajor = parseInt(current.split(".")[0], 10);
  const latMajor = parseInt(latest.split(".")[0], 10);
  return latMajor > curMajor;
}

function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json",
      "Cache-Control": "public, max-age=300",
      "Access-Control-Allow-Origin": "*",
    },
  });
}

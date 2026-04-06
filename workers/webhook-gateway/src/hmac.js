// HMAC-SHA256 verification using Web Crypto API.
// Constant-time comparison to prevent timing attacks.

/**
 * Verify an HMAC-SHA256 signature.
 * @param {string} body - Raw request body
 * @param {string} signature - Hex-encoded HMAC signature from header
 * @param {string} secret - Shared signing secret
 * @returns {Promise<boolean>}
 */
export async function verifyHmac(body, signature, secret) {
  if (!signature || !secret) return false;

  // Strip common prefixes (e.g. "sha256=", "sha1=") before comparing
  const stripped = signature.replace(/^sha(?:1|256)=/, "");

  const encoder = new TextEncoder();
  const key = await crypto.subtle.importKey(
    "raw",
    encoder.encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"]
  );

  const sig = await crypto.subtle.sign("HMAC", key, encoder.encode(body));
  const computed = bufToHex(sig);

  return constantTimeEqual(computed, stripped);
}

/** Convert ArrayBuffer to lowercase hex string. */
function bufToHex(buf) {
  return [...new Uint8Array(buf)]
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/** Constant-time string comparison. */
function constantTimeEqual(a, b) {
  if (a.length !== b.length) return false;
  let result = 0;
  for (let i = 0; i < a.length; i++) {
    result |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return result === 0;
}

// Relay webhook events to a user-provided callback URL.

/**
 * Forward an event to the relay target.
 * @param {object} event - The webhook event payload
 * @param {string} callbackUrl - User's callback URL
 * @returns {Promise<{ok: boolean, status: number}>}
 */
export async function relayEvent(event, callbackUrl) {
  if (!callbackUrl) return { ok: true, status: 0 };

  try {
    const resp = await fetch(callbackUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(event),
    });
    return { ok: resp.ok, status: resp.status };
  } catch (err) {
    return { ok: false, status: 0, error: err.message };
  }
}

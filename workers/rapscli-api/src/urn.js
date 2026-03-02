// URN Decoder endpoint
//
// GET /api/urn?urn=BASE64_URN — decode APS URN to bucket, object key, etc.
// GET /urn — landing page with paste form

/**
 * Decode an APS URN and return structured components.
 */
export async function handleUrn(request) {
  const url = new URL(request.url);

  // Landing page
  if (url.pathname === "/urn" && !url.searchParams.has("urn")) {
    return new Response(renderUrnPage(), {
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }

  const urn = url.searchParams.get("urn") || "";
  if (!urn) {
    return json({ error: "Missing urn parameter" }, 400);
  }

  const decoded = decodeUrn(urn);
  if (!decoded) {
    return json({ error: "Invalid URN format" }, 400);
  }

  // If request accepts HTML (browser), return the page with results
  const accept = request.headers.get("Accept") || "";
  if (accept.includes("text/html") && url.pathname === "/urn") {
    return new Response(renderUrnPage(urn, decoded), {
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }

  return json(decoded);
}

function decodeUrn(urn) {
  try {
    // APS URNs are base64url-encoded
    const decoded = atob(urn.replace(/-/g, "+").replace(/_/g, "/"));

    const result = { raw: urn, decoded };

    // Parse known URN formats
    // OSS: urn:adsk.objects:os.object:BUCKET/OBJECT_KEY
    const ossMatch = decoded.match(/^urn:adsk\.objects:os\.object:([^/]+)\/(.+)$/);
    if (ossMatch) {
      result.type = "OSS Object";
      result.bucket = ossMatch[1];
      result.object_key = ossMatch[2];
      return result;
    }

    // Wipdata: urn:adsk.wipprod:dm.lineage:LINEAGE_ID
    const wipMatch = decoded.match(/^urn:adsk\.wipprod:dm\.lineage:(.+)$/);
    if (wipMatch) {
      result.type = "WIP Lineage";
      result.lineage_id = wipMatch[1];
      return result;
    }

    // Version: urn:adsk.wipprod:fs.file:vf.VERSION_ID
    const verMatch = decoded.match(/^urn:adsk\.wipprod:fs\.file:vf\.(.+)$/);
    if (verMatch) {
      result.type = "WIP File Version";
      result.version_id = verMatch[1];
      return result;
    }

    // Generic URN
    const parts = decoded.split(":");
    if (parts[0] === "urn") {
      result.type = "APS URN";
      result.namespace = parts.slice(1, -1).join(":");
      result.id = parts[parts.length - 1];
      return result;
    }

    // Not a URN but decoded successfully
    result.type = "Base64 String";
    return result;
  } catch {
    return null;
  }
}

function json(data, status = 200) {
  return new Response(JSON.stringify(data, null, 2), {
    status,
    headers: {
      "Content-Type": "application/json",
      "Access-Control-Allow-Origin": "*",
    },
  });
}

function renderUrnPage(urn = "", result = null) {
  const resultHtml = result
    ? `<pre style="background:#f1f5f9;padding:16px;border-radius:8px;overflow-x:auto;font-size:13px;margin-top:20px;">${escapeHtml(JSON.stringify(result, null, 2))}</pre>`
    : "";

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>RAPS — URN Decoder</title>
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f9fafb; display: flex; justify-content: center; align-items: center; min-height: 100vh; padding: 20px; }
    .card { background: white; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,0.1); padding: 40px; max-width: 600px; width: 100%; }
    h1 { font-size: 24px; color: #111827; margin-bottom: 8px; text-align: center; }
    .subtitle { color: #6b7280; font-size: 14px; margin-bottom: 24px; text-align: center; }
    label { display: block; font-size: 13px; font-weight: 600; color: #374151; margin-bottom: 6px; }
    textarea { width: 100%; padding: 12px; font-size: 13px; font-family: monospace; border: 2px solid #d1d5db; border-radius: 8px; outline: none; resize: vertical; min-height: 60px; }
    textarea:focus { border-color: #2563eb; box-shadow: 0 0 0 3px rgba(37,99,235,0.1); }
    button { width: 100%; margin-top: 12px; padding: 12px; font-size: 16px; font-weight: 600; color: white; background: #2563eb; border: none; border-radius: 8px; cursor: pointer; }
    button:hover { background: #1d4ed8; }
    .footer { margin-top: 24px; font-size: 12px; color: #9ca3af; text-align: center; }
  </style>
</head>
<body>
  <div class="card">
    <h1>URN Decoder</h1>
    <p class="subtitle">Paste an APS URN to decode bucket, object key, and metadata.</p>
    <form action="/urn" method="GET">
      <label for="urn">APS URN (base64)</label>
      <textarea id="urn" name="urn" placeholder="dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6..." autofocus>${escapeHtml(urn)}</textarea>
      <button type="submit">Decode</button>
    </form>
    ${resultHtml}
    <p class="footer">Part of the <a href="https://rapscli.xyz">RAPS CLI</a> toolkit.</p>
  </div>
</body>
</html>`;
}

function escapeHtml(str) {
  return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

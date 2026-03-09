import type { Context } from 'hono'
import type { Env } from './index'

export function adminUI(c: Context<{ Bindings: Env }>): Response {
  const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>URL Shortener Admin</title>
  <style>
    body { font-family: sans-serif; max-width: 800px; margin: 40px auto; padding: 0 16px; }
    h1 { font-size: 1.5rem; margin-bottom: 24px; }
    h2 { font-size: 1.1rem; margin-top: 32px; margin-bottom: 12px; }
    .token-section { background: #f5f5f5; padding: 12px 16px; border-radius: 6px; margin-bottom: 24px; display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
    .token-section label { font-weight: bold; }
    .token-section input { flex: 1; min-width: 200px; padding: 6px 8px; border: 1px solid #ccc; border-radius: 4px; font-size: 0.9rem; }
    .token-section button { padding: 6px 14px; background: #333; color: #fff; border: none; border-radius: 4px; cursor: pointer; }
    .token-section button:hover { background: #555; }
    .create-form { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 16px; }
    .create-form input { padding: 6px 8px; border: 1px solid #ccc; border-radius: 4px; font-size: 0.9rem; }
    .create-form input[name="url"] { flex: 2; min-width: 200px; }
    .create-form input[name="code"] { flex: 1; min-width: 120px; }
    .create-form button { padding: 6px 14px; background: #1a6f3c; color: #fff; border: none; border-radius: 4px; cursor: pointer; }
    .create-form button:hover { background: #25944f; }
    table { width: 100%; border-collapse: collapse; font-size: 0.9rem; }
    th { text-align: left; border-bottom: 2px solid #ddd; padding: 6px 8px; }
    td { padding: 6px 8px; border-bottom: 1px solid #eee; word-break: break-all; }
    .delete-btn { padding: 4px 10px; background: #c0392b; color: #fff; border: none; border-radius: 4px; cursor: pointer; white-space: nowrap; }
    .delete-btn:hover { background: #e74c3c; }
    .error { color: #c0392b; margin: 8px 0; font-size: 0.9rem; }
    .empty { color: #888; font-style: italic; margin: 8px 0; }
    a { color: #1a6f3c; }
  </style>
</head>
<body>
  <h1>URL Shortener Admin</h1>

  <div class="token-section">
    <label for="token">Admin Token:</label>
    <input type="password" id="token" placeholder="Enter admin token" />
    <button onclick="saveToken()">Save</button>
  </div>

  <h2>Create Short Link</h2>
  <div class="create-form">
    <input type="url" name="url" id="create-url" placeholder="https://example.com/long-url" />
    <input type="text" name="code" id="create-code" placeholder="custom-code (optional)" />
    <button onclick="createLink()">Shorten</button>
  </div>
  <div id="create-error" class="error" style="display:none"></div>

  <h2>Dynamic Links</h2>
  <div id="links-error" class="error" style="display:none"></div>
  <div id="links-container"><p class="empty">Loading…</p></div>

  <script>
    function getToken() {
      return localStorage.getItem('admin_token') || '';
    }

    function saveToken() {
      const token = document.getElementById('token').value.trim();
      localStorage.setItem('admin_token', token);
      loadLinks();
    }

    function showError(id, msg) {
      const el = document.getElementById(id);
      el.textContent = msg;
      el.style.display = msg ? 'block' : 'none';
    }

    async function loadLinks() {
      const container = document.getElementById('links-container');
      showError('links-error', '');
      container.innerHTML = '<p class="empty">Loading…</p>';

      const token = getToken();
      let resp;
      try {
        resp = await fetch('/api/links', {
          headers: { 'Authorization': 'Bearer ' + token }
        });
      } catch (e) {
        showError('links-error', 'Network error: ' + e.message);
        container.innerHTML = '';
        return;
      }

      if (!resp.ok) {
        const text = await resp.text();
        showError('links-error', 'Error ' + resp.status + ': ' + text);
        container.innerHTML = '';
        return;
      }

      const links = await resp.json();

      if (!links || links.length === 0) {
        container.innerHTML = '<p class="empty">No dynamic links yet.</p>';
        return;
      }

      const rows = links.map(link => {
        const shortUrl = window.location.origin + '/' + escapeHtml(link.code);
        return '<tr>' +
          '<td><a href="/' + escapeHtml(link.code) + '" target="_blank">' + escapeHtml(link.code) + '</a></td>' +
          '<td><a href="' + escapeHtml(link.url) + '" target="_blank">' + escapeHtml(link.url) + '</a></td>' +
          '<td><button class="delete-btn" onclick="deleteLink(' + JSON.stringify(link.code) + ')">Delete</button></td>' +
          '</tr>';
      }).join('');

      container.innerHTML = '<table>' +
        '<thead><tr><th>Code</th><th>Target URL</th><th></th></tr></thead>' +
        '<tbody>' + rows + '</tbody>' +
        '</table>';
    }

    async function createLink() {
      showError('create-error', '');
      const url = document.getElementById('create-url').value.trim();
      const code = document.getElementById('create-code').value.trim();
      const token = getToken();

      if (!url) {
        showError('create-error', 'URL is required.');
        return;
      }

      const body = { url };
      if (code) body.code = code;

      let resp;
      try {
        resp = await fetch('/api/shorten', {
          method: 'POST',
          headers: {
            'Authorization': 'Bearer ' + token,
            'Content-Type': 'application/json'
          },
          body: JSON.stringify(body)
        });
      } catch (e) {
        showError('create-error', 'Network error: ' + e.message);
        return;
      }

      if (!resp.ok) {
        const text = await resp.text();
        showError('create-error', 'Error ' + resp.status + ': ' + text);
        return;
      }

      document.getElementById('create-url').value = '';
      document.getElementById('create-code').value = '';
      loadLinks();
    }

    async function deleteLink(code) {
      showError('links-error', '');
      const token = getToken();

      let resp;
      try {
        resp = await fetch('/api/links/' + encodeURIComponent(code), {
          method: 'DELETE',
          headers: { 'Authorization': 'Bearer ' + token }
        });
      } catch (e) {
        showError('links-error', 'Network error: ' + e.message);
        return;
      }

      if (!resp.ok) {
        const text = await resp.text();
        showError('links-error', 'Error ' + resp.status + ': ' + text);
        return;
      }

      loadLinks();
    }

    function escapeHtml(str) {
      return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
    }

    // Init
    const savedToken = localStorage.getItem('admin_token');
    if (savedToken) {
      document.getElementById('token').value = savedToken;
    }
    loadLinks();
  </script>
</body>
</html>`;

  return c.html(html, 200);
}

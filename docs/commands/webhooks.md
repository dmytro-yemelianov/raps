---
layout: default
title: Webhook Commands
---

# Webhook Commands

Manage webhook subscriptions for APS events. Webhooks allow you to receive notifications when events occur in your APS applications.

## Commands

### `raps webhook list`

List all webhook subscriptions.

**Usage:**
```bash
raps webhook list
```

**Example:**
```bash
$ raps webhook list
Fetching webhooks...

Webhooks:
──────────────────────────────────────────────────────────────────────────────────────────
Status         Event                      Callback URL                        Hook ID
──────────────────────────────────────────────────────────────────────────────────────────
✓ active       dm.version.added           https://example.com/webhook          abc123xyz
✗ inactive     extraction.finished         https://api.example.com/hook        def456uvw
──────────────────────────────────────────────────────────────────────────────────────────
```

**Requirements:**
- 2-legged OAuth authentication

### `raps webhook create`

Create a new webhook subscription.

**Usage:**
```bash
raps webhook create [--url URL] [--event EVENT]
```

**Options:**
- `--url, -u`: Callback URL for webhook notifications
- `--event, -e`: Event type (e.g., `dm.version.added`)

**Example:**
```bash
$ raps webhook create --url https://example.com/webhook --event dm.version.added
Creating webhook...
✓ Webhook created successfully!
  Hook ID: abc123xyz
  Event: dm.version.added
  Status: active
  Callback: https://example.com/webhook
```

**Interactive Example:**
```bash
$ raps webhook create
Enter callback URL: https://example.com/webhook
Select event type:
  > dm.version.added - A new version of an item was added
    dm.version.modified - An item version was modified
    extraction.finished - Model extraction completed
    ...

Creating webhook...
✓ Webhook created successfully!
```

**Requirements:**
- 2-legged OAuth authentication
- Callback URL must be publicly accessible (HTTPS recommended)

### `raps webhook delete`

Delete a webhook subscription.

**Usage:**
```bash
raps webhook delete <hook-id> [--system SYSTEM] [--event EVENT]
```

**Arguments:**
- `hook-id`: Hook ID to delete

**Options:**
- `--system, -s`: System (e.g., `data` or `derivative`, default: `data`)
- `--event, -e`: Event type

**Example:**
```bash
$ raps webhook delete abc123xyz --system data --event dm.version.added
Deleting webhook...
✓ Webhook deleted successfully!
```

**Requirements:**
- 2-legged OAuth authentication

### `raps webhook events`

List all available webhook events.

**Usage:**
```bash
raps webhook events
```

**Example:**
```bash
$ raps webhook events

Available Webhook Events:
────────────────────────────────────────────────────────────
  dm.version.added - A new version of an item was added
  dm.version.modified - An item version was modified
  dm.version.deleted - An item version was deleted
  extraction.finished - Model extraction completed
  extraction.updated - Model extraction updated
────────────────────────────────────────────────────────────
```

## Available Events

### Data Management Events

- `dm.version.added` - A new version of an item was added
- `dm.version.modified` - An item version was modified
- `dm.version.deleted` - An item version was deleted
- `dm.folder.added` - A folder was added
- `dm.folder.modified` - A folder was modified
- `dm.folder.deleted` - A folder was deleted

### Model Derivative Events

- `extraction.finished` - Model extraction completed
- `extraction.updated` - Model extraction updated
- `extraction.failed` - Model extraction failed

## Webhook Payload

When an event occurs, APS sends a POST request to your callback URL with a JSON payload:

```json
{
  "hook": {
    "hookId": "abc123xyz",
    "tenant": "your-tenant-id",
    "status": "active",
    "callbackUrl": "https://example.com/webhook",
    "createdBy": "user@example.com",
    "event": "dm.version.added",
    "createdDate": "2024-01-15T10:30:00Z",
    "lastUpdatedDate": "2024-01-15T10:30:00Z"
  },
  "payload": {
    "version": {
      "id": "urn:adsk.wiprod:fs.file:co.abc123xyz",
      "type": "versions",
      "attributes": {
        "name": "building.dwg",
        "displayName": "building.dwg",
        "createTime": "2024-01-15T10:30:00Z",
        "createUserName": "john.doe@example.com",
        "lastModifiedTime": "2024-01-15T10:30:00Z",
        "lastModifiedUserName": "john.doe@example.com",
        "fileType": "dwg",
        "extension": {
          "type": "items:autodesk.bim360:File",
          "version": "1.0"
        }
      },
      "relationships": {
        "item": {
          "data": {
            "type": "items",
            "id": "urn:adsk.wiprod:fs.file:co.def456uvw"
          }
        },
        "storage": {
          "data": {
            "type": "objects",
            "id": "urn:adsk.objects:os.object:bucket/file.dwg"
          }
        }
      }
    }
  }
}
```

## Setting Up a Webhook Endpoint

Your webhook endpoint should:

1. **Accept POST requests** from APS
2. **Return 200 OK** to acknowledge receipt
3. **Verify the request** (optional but recommended)
4. **Process the event** asynchronously

**Example Node.js endpoint:**

```javascript
app.post('/webhook', (req, res) => {
  const { hook, payload } = req.body;
  
  // Verify webhook (optional)
  // Process the event
  console.log(`Event: ${hook.event}`);
  console.log(`Payload:`, payload);
  
  // Return 200 OK
  res.status(200).send('OK');
});
```

## Best Practices

1. **Use HTTPS** for callback URLs (required for production)
2. **Verify webhook signatures** to ensure requests are from APS
3. **Handle retries** - APS will retry failed webhooks
4. **Process asynchronously** - return 200 OK quickly, process later
5. **Monitor webhook status** - check `raps webhook list` regularly
6. **Test locally** - use tools like ngrok for local development

## Troubleshooting

### Webhook Not Receiving Events

1. Check webhook status: `raps webhook list`
2. Verify callback URL is publicly accessible
3. Ensure endpoint returns 200 OK
4. Check server logs for errors

### Webhook Status Shows "inactive"

1. Check if endpoint is responding
2. Verify HTTPS is used (required for production)
3. Check APS application settings
4. Delete and recreate the webhook

## Related Commands

- [Authentication]({{ '/commands/auth' | relative_url }}) - Set up authentication
- [Data Management]({{ '/commands/data-management' | relative_url }}) - Manage data that triggers events
- [Translation]({{ '/commands/translation' | relative_url }}) - Translation events


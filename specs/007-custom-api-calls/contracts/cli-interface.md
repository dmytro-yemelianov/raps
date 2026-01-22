# CLI Interface Contract: Custom API Calls

**Feature**: 007-custom-api-calls
**Date**: 2026-01-22

## Command Structure

```text
raps api <METHOD> <ENDPOINT> [OPTIONS]
```

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `get` | Execute HTTP GET request |
| `post` | Execute HTTP POST request |
| `put` | Execute HTTP PUT request |
| `patch` | Execute HTTP PATCH request |
| `delete` | Execute HTTP DELETE request |

---

## Command: `raps api get`

Execute a GET request to an APS API endpoint.

### Synopsis

```bash
raps api get <ENDPOINT> [OPTIONS]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `ENDPOINT` | Yes | API endpoint path (e.g., `/oss/v2/buckets`) |

### Options

| Option | Short | Type | Description |
|--------|-------|------|-------------|
| `--query` | `-q` | KEY=VALUE | Query parameter (repeatable) |
| `--header` | `-H` | KEY:VALUE | Custom header (repeatable) |
| `--output` | `-o` | PATH | Save response to file |
| `--verbose` | `-v` | flag | Show response headers and status |

### Examples

```bash
# Get current user profile
raps api get /userprofile/v1/users/@me

# List buckets with query parameter
raps api get /oss/v2/buckets --query limit=10 --query region=US

# Get with custom header and verbose output
raps api get /oss/v2/buckets -H "x-ads-region:US" --verbose

# Save response to file
raps api get /oss/v2/buckets -o buckets.json
```

---

## Command: `raps api post`

Execute a POST request to create a resource.

### Synopsis

```bash
raps api post <ENDPOINT> [OPTIONS]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `ENDPOINT` | Yes | API endpoint path |

### Options

| Option | Short | Type | Description |
|--------|-------|------|-------------|
| `--data` | `-d` | JSON | Inline JSON request body |
| `--data-file` | `-f` | PATH | Read request body from file |
| `--query` | `-q` | KEY=VALUE | Query parameter (repeatable) |
| `--header` | `-H` | KEY:VALUE | Custom header (repeatable) |
| `--output` | `-o` | PATH | Save response to file |
| `--verbose` | `-v` | flag | Show response headers and status |

### Examples

```bash
# Create bucket with inline JSON
raps api post /oss/v2/buckets -d '{"bucketKey":"my-bucket","policyKey":"transient"}'

# Create from file
raps api post /oss/v2/buckets --data-file bucket.json

# POST with query params
raps api post /endpoint --query param=value -d '{}'
```

---

## Command: `raps api put`

Execute a PUT request to replace a resource.

### Synopsis

```bash
raps api put <ENDPOINT> [OPTIONS]
```

### Options

Same as `raps api post`.

### Examples

```bash
# Update resource
raps api put /resource/123 -d '{"name":"updated"}'
```

---

## Command: `raps api patch`

Execute a PATCH request to update a resource.

### Synopsis

```bash
raps api patch <ENDPOINT> [OPTIONS]
```

### Options

Same as `raps api post`.

### Examples

```bash
# Partial update
raps api patch /resource/123 -d '{"status":"active"}'
```

---

## Command: `raps api delete`

Execute a DELETE request to remove a resource.

### Synopsis

```bash
raps api delete <ENDPOINT> [OPTIONS]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `ENDPOINT` | Yes | API endpoint path |

### Options

| Option | Short | Type | Description |
|--------|-------|------|-------------|
| `--query` | `-q` | KEY=VALUE | Query parameter (repeatable) |
| `--header` | `-H` | KEY:VALUE | Custom header (repeatable) |
| `--verbose` | `-v` | flag | Show response headers and status |

### Examples

```bash
# Delete bucket
raps api delete /oss/v2/buckets/my-bucket

# Delete with query param
raps api delete /resource/123 --query force=true
```

---

## Global Options

These options from the main `raps` command apply to all `api` subcommands:

| Option | Description |
|--------|-------------|
| `--output-format` | Output format: json, yaml, table, csv, plain |
| `--no-color` | Disable colored output |
| `--quiet` | Suppress non-essential output |
| `--debug` | Enable debug logging |
| `--timeout` | Request timeout in seconds |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (2xx response) |
| 1 | General error (4xx/5xx response, network error) |
| 2 | Command-line error (invalid arguments, validation failure) |
| 10 | Authentication error (401/403 or no token) |

---

## Output Formats

### Default (JSON)

```json
{
  "items": [...],
  "next": "..."
}
```

### With `--verbose`

```text
HTTP/1.1 200 OK
Content-Type: application/json
X-Request-Id: abc123

{
  "items": [...],
  "next": "..."
}
```

### Error Response

```json
{
  "status": 404,
  "error": "not_found",
  "message": "Bucket 'xyz' not found",
  "details": { ... }
}
```

---

## Validation Rules

1. **Endpoint must be relative path or allowed domain**
   - Valid: `/oss/v2/buckets`, `/userprofile/v1/users/@me`
   - Invalid: `https://evil.com/steal-token`

2. **Body options are mutually exclusive**
   - `--data` and `--data-file` cannot be used together

3. **Body only allowed for POST, PUT, PATCH**
   - `raps api get /endpoint --data '{}'` → Error

4. **JSON body must be valid JSON**
   - Invalid JSON in `--data` or `--data-file` → Error with parse message

5. **Header format must be KEY:VALUE**
   - `--header "Content-Type: application/xml"` → Valid
   - `--header "InvalidHeader"` → Error

6. **Query format must be KEY=VALUE**
   - `--query limit=10` → Valid
   - `--query invalidquery` → Error

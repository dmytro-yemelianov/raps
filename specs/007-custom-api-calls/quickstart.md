# Quickstart: Custom API Calls

**Feature**: 007-custom-api-calls

This guide helps you get started with the custom API calls feature in RAPS.

---

## Prerequisites

1. **RAPS installed** - [Installation guide](https://rapscli.xyz/install)
2. **Authenticated** - Run `raps auth login` to authenticate with your Autodesk account

---

## CLI Usage

### Basic GET Request

Retrieve your user profile:

```bash
raps api get /userprofile/v1/users/@me
```

### GET with Query Parameters

List buckets with pagination:

```bash
raps api get /oss/v2/buckets --query limit=10 --query startAt=0
```

### POST with JSON Body

Create a new bucket:

```bash
raps api post /oss/v2/buckets \
  --data '{"bucketKey":"my-new-bucket","policyKey":"transient"}'
```

### POST from File

Create a resource using a JSON file:

```bash
# Create payload file
echo '{"bucketKey":"my-bucket","policyKey":"persistent"}' > bucket.json

# Execute request
raps api post /oss/v2/buckets --data-file bucket.json
```

### Custom Headers

Add custom headers to your request:

```bash
raps api get /oss/v2/buckets \
  --header "x-ads-region:US" \
  --header "Accept:application/json"
```

### Save Response to File

Save the response body to a file:

```bash
raps api get /oss/v2/buckets -o buckets.json
```

### Verbose Output

See response headers and status:

```bash
raps api get /oss/v2/buckets --verbose
```

Output:
```text
HTTP/1.1 200 OK
Content-Type: application/json
x-request-id: abc123

{
  "items": [...]
}
```

---

## MCP Tool Usage

### Tool: api_request

The `api_request` MCP tool allows AI assistants to make custom API calls.

#### Example: GET Request

```json
{
  "method": "GET",
  "endpoint": "/oss/v2/buckets",
  "query": {
    "limit": "5"
  }
}
```

#### Example: POST Request

```json
{
  "method": "POST",
  "endpoint": "/oss/v2/buckets",
  "body": {
    "bucketKey": "ai-created-bucket",
    "policyKey": "transient"
  }
}
```

---

## Output Formats

Use the global `--output-format` flag to change output:

```bash
# JSON (default)
raps api get /oss/v2/buckets

# YAML
raps api get /oss/v2/buckets --output-format yaml

# Table
raps api get /oss/v2/buckets --output-format table

# CSV
raps api get /oss/v2/buckets --output-format csv
```

---

## Common Use Cases

### Explore Undocumented Endpoints

Test new or beta APS endpoints:

```bash
raps api get /some/beta/endpoint/v1
```

### Debug API Responses

Use verbose mode to troubleshoot:

```bash
raps api get /oss/v2/buckets/my-bucket --verbose
```

### Script Integration

Use in shell scripts with exit code checking:

```bash
#!/bin/bash
if raps api get /oss/v2/buckets/my-bucket > /dev/null 2>&1; then
    echo "Bucket exists"
else
    echo "Bucket not found"
fi
```

### Chain with jq

Process JSON responses:

```bash
raps api get /oss/v2/buckets | jq '.items[].bucketKey'
```

---

## Error Handling

### Authentication Error

```bash
$ raps api get /oss/v2/buckets
Error: Not authenticated. Run 'raps auth login' first.
```

**Solution:** Run `raps auth login` to authenticate.

### Invalid Endpoint

```bash
$ raps api get https://evil.com/endpoint
Error: Only APS API endpoints are allowed. Use a path like /oss/v2/buckets
```

**Solution:** Use relative paths to APS APIs.

### Invalid JSON

```bash
$ raps api post /endpoint --data '{invalid}'
Error: Invalid JSON: expected value at line 1 column 2
```

**Solution:** Ensure valid JSON syntax in `--data` or `--data-file`.

---

## Next Steps

- See [CLI Interface Contract](./contracts/cli-interface.md) for full command reference
- See [MCP Tool Contract](./contracts/mcp-tool.md) for MCP integration details
- See [Data Model](./data-model.md) for entity definitions

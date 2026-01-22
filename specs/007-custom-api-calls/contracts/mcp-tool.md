# MCP Tool Contract: api_request

**Feature**: 007-custom-api-calls
**Date**: 2026-01-22

## Tool Definition

### Name

`api_request`

### Description

Execute a custom HTTP request to any APS API endpoint using the current authentication. Supports all HTTP methods (GET, POST, PUT, PATCH, DELETE) with optional body, headers, and query parameters.

---

## Input Schema

```json
{
  "type": "object",
  "properties": {
    "method": {
      "type": "string",
      "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"],
      "description": "HTTP method to use"
    },
    "endpoint": {
      "type": "string",
      "description": "API endpoint path relative to APS base URL (e.g., /oss/v2/buckets)"
    },
    "body": {
      "type": "object",
      "description": "Request body as JSON object. Only valid for POST, PUT, PATCH methods."
    },
    "headers": {
      "type": "object",
      "additionalProperties": {
        "type": "string"
      },
      "description": "Custom headers to include in the request. Authorization header is set automatically."
    },
    "query": {
      "type": "object",
      "additionalProperties": {
        "type": "string"
      },
      "description": "Query parameters to append to the endpoint URL."
    }
  },
  "required": ["method", "endpoint"]
}
```

---

## Output Format

### Success Response (2xx)

```json
{
  "success": true,
  "status": 200,
  "data": { ... }
}
```

The `data` field contains the parsed JSON response from the API.

### Error Response (4xx/5xx)

```json
{
  "success": false,
  "status": 404,
  "error": "not_found",
  "message": "Bucket 'xyz' not found",
  "details": { ... }
}
```

### Validation Error

```json
{
  "success": false,
  "error": "validation_error",
  "message": "Body is not allowed for GET requests"
}
```

---

## Examples

### GET Request

**Input:**
```json
{
  "method": "GET",
  "endpoint": "/oss/v2/buckets",
  "query": {
    "limit": "10",
    "region": "US"
  }
}
```

**Output:**
```json
{
  "success": true,
  "status": 200,
  "data": {
    "items": [
      {
        "bucketKey": "my-bucket",
        "policyKey": "transient"
      }
    ]
  }
}
```

### POST Request with Body

**Input:**
```json
{
  "method": "POST",
  "endpoint": "/oss/v2/buckets",
  "body": {
    "bucketKey": "new-bucket",
    "policyKey": "persistent"
  },
  "headers": {
    "x-ads-region": "US"
  }
}
```

**Output:**
```json
{
  "success": true,
  "status": 201,
  "data": {
    "bucketKey": "new-bucket",
    "bucketOwner": "...",
    "createdDate": 1706000000000
  }
}
```

### DELETE Request

**Input:**
```json
{
  "method": "DELETE",
  "endpoint": "/oss/v2/buckets/old-bucket"
}
```

**Output:**
```json
{
  "success": true,
  "status": 200,
  "data": {}
}
```

### Error Example

**Input:**
```json
{
  "method": "GET",
  "endpoint": "/oss/v2/buckets/nonexistent"
}
```

**Output:**
```json
{
  "success": false,
  "status": 404,
  "error": "not_found",
  "message": "Bucket 'nonexistent' does not exist"
}
```

---

## Validation Rules

1. **Method is required and must be valid enum value**
   - Invalid method → `"error": "Invalid method. Must be GET, POST, PUT, PATCH, or DELETE"`

2. **Endpoint is required and must be valid APS path**
   - Empty endpoint → `"error": "Endpoint is required"`
   - External URL → `"error": "Only APS API endpoints are allowed"`

3. **Body only allowed for POST, PUT, PATCH**
   - Body with GET/DELETE → `"error": "Body is not allowed for GET/DELETE requests"`

4. **Authentication required**
   - No valid token → `"error": "Not authenticated. Run 'raps auth login' first."`

---

## Security Constraints

- Requests are restricted to APS domains only
- Authorization header is set automatically from stored token
- User-provided Authorization header is ignored
- No credential or token data is included in tool responses

---

## Integration with Existing Tools

The `api_request` tool complements existing RAPS MCP tools:

| Use Case | Recommended Tool |
|----------|-----------------|
| List buckets | `bucket_list` (typed, documented) |
| Custom bucket query | `api_request` with GET /oss/v2/buckets |
| Unsupported endpoint | `api_request` (only option) |

**When to use `api_request`:**
- Endpoint not covered by existing typed tools
- Need specific query parameters not exposed by typed tools
- Exploring new APS API features before RAPS adds support

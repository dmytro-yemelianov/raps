# Data Model: Custom API Calls

**Feature**: 007-custom-api-calls
**Date**: 2026-01-22

## Overview

This feature introduces data structures for representing custom API requests and responses. The model is intentionally simple since the feature acts as a pass-through to arbitrary APS endpoints.

---

## Entities

### 1. ApiRequest

Represents a custom API call to be executed.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| method | HttpMethod | Yes | HTTP method (GET, POST, PUT, PATCH, DELETE) |
| endpoint | String | Yes | API endpoint path (e.g., `/oss/v2/buckets`) |
| headers | Map<String, String> | No | Custom headers to include |
| query_params | Vec<(String, String)> | No | Query parameters (key-value pairs) |
| body | Option<Value> | No | Request body (JSON value) |

**Validation Rules**:
- `endpoint` must start with `/` (relative path) or be a full URL to an allowed domain
- `body` is only valid for POST, PUT, PATCH methods
- `headers` cannot override Authorization header (set automatically)

**Derived Fields**:
- `full_url`: Computed from base URL + endpoint + query params

---

### 2. HttpMethod

Enumeration of supported HTTP methods.

| Value | Description |
|-------|-------------|
| GET | Retrieve resource(s) |
| POST | Create resource |
| PUT | Replace resource |
| PATCH | Update resource |
| DELETE | Remove resource |

---

### 3. ApiResponse

Represents the response from a custom API call.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| status_code | u16 | Yes | HTTP status code |
| headers | Map<String, String> | Yes | Response headers |
| content_type | String | Yes | Content-Type header value |
| body | ResponseBody | Yes | Response body (see below) |

---

### 4. ResponseBody

Represents the response body in different formats.

| Variant | Payload | Description |
|---------|---------|-------------|
| Json | serde_json::Value | Parsed JSON response |
| Text | String | Text response (XML, HTML, plain text) |
| Binary | Vec<u8> | Binary response (images, files) |

**Content-Type Mapping**:
- `application/json` → Json
- `text/*`, `application/xml` → Text
- Everything else → Binary

---

### 5. ApiError

Error response structure for failed requests.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| status_code | u16 | Yes | HTTP status code |
| error_type | String | Yes | Error category |
| message | String | Yes | Human-readable error message |
| details | Option<Value> | No | Additional error details from API |

**Error Categories**:
- `authentication`: 401/403 responses
- `validation`: 400/422 responses (bad request, invalid input)
- `not_found`: 404 responses
- `rate_limited`: 429 responses
- `server_error`: 5xx responses
- `network`: Connection failures, timeouts

---

## Relationships

```text
┌─────────────┐
│  ApiRequest │
├─────────────┤         ┌──────────────┐
│ method      │────────>│  HttpMethod  │
│ endpoint    │         └──────────────┘
│ headers     │
│ query_params│         ┌──────────────┐
│ body        │────────>│    Value     │ (JSON)
└─────────────┘         └──────────────┘
       │
       │ executes
       ▼
┌─────────────┐
│ ApiResponse │
├─────────────┤         ┌──────────────┐
│ status_code │         │ ResponseBody │
│ headers     │────────>├──────────────┤
│ content_type│         │ Json(Value)  │
│ body        │         │ Text(String) │
└─────────────┘         │ Binary(bytes)│
       │                └──────────────┘
       │ on error
       ▼
┌─────────────┐
│  ApiError   │
├─────────────┤
│ status_code │
│ error_type  │
│ message     │
│ details     │
└─────────────┘
```

---

## State Transitions

This feature is stateless. Each request is independent with no persistent state between calls.

---

## CLI Output Structures

### SuccessOutput (for 2xx responses)

Used when formatting successful API responses for CLI output.

| Field | Type | Description |
|-------|------|-------------|
| status | u16 | HTTP status code |
| data | Value | Response body (JSON) |

### ErrorOutput (for 4xx/5xx responses)

Used when formatting error responses for CLI output.

| Field | Type | Description |
|-------|------|-------------|
| status | u16 | HTTP status code |
| error | String | Error category |
| message | String | Error message |
| details | Option<Value> | Additional details |

---

## MCP Tool Schema

### api_request Input

```json
{
  "type": "object",
  "properties": {
    "method": {
      "type": "string",
      "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"],
      "description": "HTTP method"
    },
    "endpoint": {
      "type": "string",
      "description": "API endpoint path (e.g., /oss/v2/buckets)"
    },
    "body": {
      "type": "object",
      "description": "Request body for POST/PUT/PATCH"
    },
    "headers": {
      "type": "object",
      "additionalProperties": { "type": "string" },
      "description": "Custom headers"
    },
    "query": {
      "type": "object",
      "additionalProperties": { "type": "string" },
      "description": "Query parameters"
    }
  },
  "required": ["method", "endpoint"]
}
```

### api_request Output

```json
{
  "type": "object",
  "properties": {
    "status": { "type": "integer" },
    "success": { "type": "boolean" },
    "data": { "type": "object" },
    "error": { "type": "string" }
  }
}
```

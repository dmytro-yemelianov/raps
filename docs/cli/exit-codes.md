---
layout: default
title: Exit Codes
---

# Exit Codes

RAPS CLI uses standardized exit codes to enable reliable error handling in CI/CD pipelines and shell scripts.

## Exit Code Reference

| Code | Meaning | Description |
|------|---------|-------------|
| `0` | Success | Command completed successfully |
| `2` | Invalid Arguments | Invalid arguments or validation failure |
| `3` | Auth Failure | Authentication failure (invalid credentials, expired token, etc.) |
| `4` | Not Found | Resource not found (404 errors) |
| `5` | Remote/API Error | Remote server error (5xx) or network issues |
| `6` | Internal Error | Internal CLI error or unexpected failure |

## Usage in Scripts

### Bash/Shell

```bash
#!/bin/bash

raps bucket list

case $? in
    0) echo "Success" ;;
    2) echo "Invalid arguments" ;;
    3) echo "Authentication failed" ;;
    4) echo "Bucket not found" ;;
    5) echo "API error" ;;
    6) echo "Internal error" ;;
esac
```

### PowerShell

```powershell
raps bucket list

switch ($LASTEXITCODE) {
    0 { Write-Host "Success" }
    2 { Write-Host "Invalid arguments" }
    3 { Write-Host "Authentication failed" }
    4 { Write-Host "Bucket not found" }
    5 { Write-Host "API error" }
    6 { Write-Host "Internal error" }
}
```

### CI/CD Examples

#### GitHub Actions

```yaml
- name: List buckets
  run: raps bucket list
  continue-on-error: false

- name: Check result
  if: failure()
  run: |
    case $? in
      3) echo "::error::Authentication failed" ;;
      5) echo "::warning::API error, retrying..." ;;
      *) exit $? ;;
    esac
```

#### Azure DevOps

```yaml
- script: raps bucket list
  continueOnError: false
  displayName: List buckets
```

## Error Detection

Exit codes are determined by analyzing the error chain. Common patterns:

- **Auth errors**: Contains "authentication failed", "unauthorized", "forbidden", "401", "403"
- **Not found**: Contains "not found", "404"
- **Validation errors**: Contains "invalid", "required", "missing", "cannot be empty"
- **Remote errors**: Contains "500", "502", "503", "504", "timeout", "connection", "network"

## Enhanced Error Interpretation

RAPS provides human-readable error explanations and suggestions for common API errors:

### Example Error Output

```
Error: Authentication failed - token may be expired or invalid.
  Code: Unauthorized (HTTP 401)
  Details: {"error": "invalid_token"}

Suggestions:
  → Run 'raps auth login' to refresh your token
  → Check if your credentials have expired
  → Verify your APS application is active
```

### Error Codes and Suggestions

| HTTP Status | Error Code | Explanation | Suggestions |
|-------------|------------|-------------|-------------|
| 401 | Unauthorized | Authentication failed | Login again, check credentials |
| 403 | Forbidden | Permission denied | Check scopes, verify access |
| 404 | NotFound | Resource not found | Verify resource ID/name |
| 409 | Conflict | Resource conflict | Check if resource exists |
| 429 | TooManyRequests | Rate limit exceeded | Wait and retry |
| 500+ | ServerError | Server error | Wait and retry, check status |

### Programmatic Error Handling

For machine-readable errors, use JSON output:

```bash
$ raps bucket get nonexistent --format json
{
  "error": {
    "status_code": 404,
    "code": "NotFound",
    "message": "Bucket 'nonexistent' not found",
    "suggestions": ["Verify bucket name", "List available buckets"]
  }
}
```

## Notes

- Exit code `1` is reserved for general errors (not used by RAPS CLI)
- Exit code `2` is also used by clap for argument parsing errors
- All commands follow the same exit code conventions
- Use `--format json` for machine-readable error output


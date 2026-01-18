# Quickstart: MCP Server Native Authentication

## Overview

This feature enhances the RAPS MCP server with native authentication support, providing clear guidance for credential setup and a streamlined 3-legged OAuth flow.

## New/Modified MCP Tools

| Tool | Action | Description |
|------|--------|-------------|
| `auth_status` | **Enhanced** | Now includes setup guidance, tool availability by auth type, and actionable next steps |
| `auth_test` | **Enhanced** | Now provides troubleshooting guidance on failure instead of raw errors |
| `auth_login` | **New** | Initiates 3-legged OAuth with browser or device code fallback |
| `auth_logout` | **New** | Clears stored 3-legged tokens |

## Usage Examples

### Check Authentication Status

```
Tool: auth_status
Arguments: {}
```

**Response (no credentials)**:
```
Authentication Status
=====================

2-legged OAuth: NOT CONFIGURED
  ⚠ Missing APS_CLIENT_ID environment variable

To set up 2-legged authentication:
1. Go to https://aps.autodesk.com/
2. Create or select an application
3. Copy your Client ID and Client Secret
4. Set environment variables:
   - APS_CLIENT_ID=your_client_id
   - APS_CLIENT_SECRET=your_client_secret

3-legged OAuth: NOT LOGGED IN
  → Run auth_login to authenticate for BIM 360/ACC access

Tool Availability:
  ✗ OSS (bucket_*, object_*) - requires 2-legged auth
  ✗ Derivative (translate_*) - requires 2-legged auth
  ✗ Data Management (hub_*, project_*, folder_*, item_*) - requires 3-legged auth
  ✗ ACC (issue_*, rfi_*, acc_*) - requires 3-legged auth
```

### Initiate 3-Legged Login

```
Tool: auth_login
Arguments: {}
```

**Response (browser available)**:
```
Opening browser for Autodesk login...

If the browser doesn't open, visit:
https://developer.api.autodesk.com/authentication/v2/authorize?...

After logging in, you'll be redirected back automatically.
Run auth_status to verify login was successful.
```

**Response (headless/no browser)**:
```
Device Code Authentication
==========================

1. Visit: https://autodesk.com/device
2. Enter code: ABCD-1234
3. Log in with your Autodesk account

Code expires in 15 minutes.
Run auth_status to verify login was successful.
```

### Test 2-Legged Credentials

```
Tool: auth_test
Arguments: {}
```

**Response (success)**:
```
✓ 2-legged OAuth authentication successful!

Your credentials are valid. Available tools:
  • bucket_list, bucket_create, bucket_get, bucket_delete
  • object_list, object_delete, object_signed_url, object_urn
  • translate_start, translate_status
  • admin_project_list, admin_operation_*
```

**Response (invalid credentials)**:
```
✗ 2-legged OAuth authentication failed

Error: Invalid client credentials (401 Unauthorized)

Troubleshooting:
1. Verify APS_CLIENT_ID is correct (32 characters)
2. Verify APS_CLIENT_SECRET is correct (64 characters)
3. Check that your app is not in sandbox/trial mode
4. Ensure credentials are for the correct APS region

Get credentials at: https://aps.autodesk.com/
```

## Implementation Files

| File | Purpose |
|------|---------|
| `raps-cli/src/mcp/server.rs` | Enhanced auth tools, new auth_login/logout |
| `raps-cli/src/mcp/tools.rs` | Updated tool list (14 → 16 tools in auth category) |
| `raps-cli/src/mcp/auth_guidance.rs` | Static instruction content and tool-auth mapping |

## Testing

```bash
# Run MCP auth tool tests
cargo test -p raps-cli mcp_auth

# Manual testing with MCP inspector
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"auth_status","arguments":{}},"id":1}' | cargo run -- serve
```

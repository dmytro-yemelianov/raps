---
layout: default
title: Authentication Commands
---

# Authentication Commands

RAPS supports both 2-legged (client credentials) and 3-legged (authorization code) OAuth flows for authentication.

## Commands

### `raps auth test`

Test 2-legged OAuth authentication using your Client ID and Client Secret.

**Usage:**
```bash
raps auth test
```

**Example:**
```bash
$ raps auth test
✓ Authentication successful!
  Token expires in: 3599 seconds
```

**Requirements:**
- `APS_CLIENT_ID` environment variable
- `APS_CLIENT_SECRET` environment variable

### `raps auth login`

Login with 3-legged OAuth. Supports multiple authentication methods.

**Usage:**
```bash
raps auth login [--default] [--device] [--token <token>] [--refresh-token <token>] [--expires-in <seconds>]
```

**Options:**
- `--default`: Use default scopes without prompting
- `--device`: Use device code flow (for headless/server environments)
- `--token <token>`: Provide access token directly (for CI/CD)
- `--refresh-token <token>`: Optional refresh token (used with --token)
- `--expires-in <seconds>`: Token expiry in seconds (default: 3600, used with --token)

**Examples:**

Browser-based login (default):
```bash
$ raps auth login
Opening browser for authentication...
Select scopes:
  [x] data:read - Read data (hubs, projects, folders, items)
  [x] data:write - Write data (create/update items)
  [x] data:create - Create new data
  [ ] data:search - Search for data
  [x] account:read - Read account information
  [ ] account:write - Write account information
  [x] user:read - Read user profile
  [ ] user:write - Write user profile
  [x] viewables:read - Read viewable content

✓ Login successful!
  User: john.doe@example.com
  Token expires in: 3599 seconds
```

Device code flow (headless/server):
```bash
$ raps auth login --device
Device Code Authentication
──────────────────────────────────────────────────
  User Code: ABC-123-DEF
  Verification URL: https://developer.api.autodesk.com/authentication/v2/device
  Complete URL: https://developer.api.autodesk.com/authentication/v2/device?user_code=ABC-123-DEF

Please visit the URL above and enter the user code to authorize.
Waiting for authorization (expires in 600 seconds)...
.
✓ Authorization successful!
```

Token-based login (CI/CD):
```bash
$ raps auth login --token "eyJhbGc..." --refresh-token "refresh_token_here" --expires-in 3600
⚠️  WARNING: Using token-based login. Tokens should be kept secure!
   This is intended for CI/CD environments. Never commit tokens to version control.
✓ Token validated for user: user@example.com
```

**Available Scopes:**
- `data:read` - Read data (hubs, projects, folders, items)
- `data:write` - Write data (create/update items)
- `data:create` - Create new data
- `data:search` - Search for data
- `account:read` - Read account information
- `account:write` - Write account information
- `user:read` - Read user profile
- `user:write` - Write user profile
- `viewables:read` - Read viewable content

**Default Scopes:**
When using `--default`, the following scopes are requested:
- `data:read`
- `data:write`
- `data:create`
- `account:read`
- `user:read`
- `viewables:read`

**Requirements:**
- `APS_CLIENT_ID` environment variable
- `APS_CLIENT_SECRET` environment variable
- `APS_CALLBACK_URL` environment variable (defaults to `http://localhost:8080/callback`)

### `raps auth logout`

Logout and clear stored authentication tokens.

**Usage:**
```bash
raps auth logout
```

**Example:**
```bash
$ raps auth logout
✓ Logged out successfully. Tokens cleared.
```

### `raps auth status`

Show current authentication status.

**Usage:**
```bash
raps auth status
```

**Example:**
```bash
$ raps auth status
Authentication Status
────────────────────────────────────────
  2-legged (Client Credentials): ✓ Available
  3-legged (User Login): ✓ Logged in
    Token: abcd...wxyz
    Expires in: 1h 30m
```

**Output includes:**
- 2-legged authentication availability
- 3-legged login status
- Token preview (first 4 and last 4 characters)
- Token expiry time (hours and minutes remaining)

### `raps auth whoami`

Show logged-in user profile information (requires 3-legged auth).

**Usage:**
```bash
raps auth whoami
```

**Example:**
```bash
$ raps auth whoami
User Profile:
  Name: John Doe
  Email: john.doe@example.com
  User ID: abc123xyz
  Profile Image: https://...
```

**Requirements:**
- Must be logged in with 3-legged OAuth (`raps auth login`)

## Authentication Types

### 2-Legged OAuth (Client Credentials)

Used for server-to-server operations that don't require user context.

**When to use:**
- Uploading files to OSS
- Creating buckets
- Starting translations
- Managing webhooks
- Design Automation operations

**Setup:**
```bash
# Set environment variables
export APS_CLIENT_ID="your_client_id"
export APS_CLIENT_SECRET="your_client_secret"

# Test authentication
raps auth test
```

### 3-Legged OAuth (Authorization Code)

Used for operations that require user context and access to user data.

**When to use:**
- Accessing BIM 360/ACC hubs and projects
- Browsing folders and items
- Managing issues
- Accessing user-specific data

**Setup:**
```bash
# Set environment variables
export APS_CLIENT_ID="your_client_id"
export APS_CLIENT_SECRET="your_client_secret"
export APS_CALLBACK_URL="http://localhost:8080/callback"

# Login
raps auth login
```

## Token Management

RAPS automatically:
- Stores tokens securely in platform-specific directories
- Refreshes tokens when they expire
- Uses the appropriate token type for each operation

**Token Storage Locations:**
- **Windows**: `%APPDATA%\raps\` or `%LOCALAPPDATA%\raps\`
- **macOS**: `~/Library/Application Support/raps/`
- **Linux**: `~/.local/share/raps/` or `$XDG_DATA_HOME/raps/`

**Clearing Tokens:**
```bash
raps auth logout
```

## Troubleshooting

### "Authentication failed" error

1. Verify `APS_CLIENT_ID` and `APS_CLIENT_SECRET` are set correctly
2. Check that your APS application is active in the Developer Portal
3. Ensure credentials haven't been rotated

### "Callback URL mismatch" error

1. Verify `APS_CALLBACK_URL` matches the callback URL configured in your APS application
2. Default callback URL is `http://localhost:8080/callback`

### "Token expired" error

Tokens are automatically refreshed. If you see this error:
1. Try logging out and logging back in: `raps auth logout && raps auth login`
2. Check your system clock is synchronized

## Related Commands

- [Buckets]({{ '/commands/buckets' | relative_url }}) - OSS bucket operations
- [Objects]({{ '/commands/objects' | relative_url }}) - OSS object operations
- [Data Management]({{ '/commands/data-management' | relative_url }}) - BIM 360/ACC operations (requires 3-legged auth)


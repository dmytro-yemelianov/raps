# Data Model: MCP Server Native Authentication Support

**Feature**: 001-mcp-native-auth
**Date**: 2026-01-17

## Entities

### AuthRequirement (Enum)

Defines the authentication type required for each MCP tool.

| Variant | Description |
|---------|-------------|
| `TwoLegged` | Requires 2-legged OAuth (client credentials) |
| `ThreeLegged` | Requires 3-legged OAuth (user authorization) |
| `Either` | Works with either auth type |

### AuthState (Struct)

Represents the current authentication status.

| Field | Type | Description |
|-------|------|-------------|
| `has_client_id` | `bool` | Whether APS_CLIENT_ID is configured |
| `has_client_secret` | `bool` | Whether APS_CLIENT_SECRET is configured |
| `two_legged_valid` | `bool` | Whether 2-legged auth succeeds |
| `three_legged_valid` | `bool` | Whether 3-legged token exists and is valid |
| `three_legged_expired` | `bool` | Whether 3-legged token exists but is expired |

### ToolAuthMap (HashMap)

Static mapping of tool names to their auth requirements.

```text
tool_name (String) -> AuthRequirement
```

**Categories**:
- 2-legged tools: `bucket_*`, `object_*`, `translate_*`, `admin_project_list`, `admin_operation_*`
- 3-legged tools: `hub_list`, `project_list`, `folder_*`, `item_*`, `issue_*`, `rfi_*`, `acc_*`
- Either: `auth_test`, `auth_status`, `auth_login`

### LoginResult (Enum)

Result of `auth_login` tool execution.

| Variant | Fields | Description |
|---------|--------|-------------|
| `BrowserOpened` | `auth_url: String` | Browser opened with auth URL |
| `DeviceCode` | `user_code: String, verification_url: String, expires_in: u64` | Device code for headless auth |
| `AlreadyLoggedIn` | `user_email: Option<String>` | User already has valid 3-legged token |
| `Error` | `message: String, guidance: String` | Auth initiation failed with guidance |

## Relationships

```text
┌─────────────────┐
│   RapsServer    │
└────────┬────────┘
         │ uses
         ▼
┌─────────────────┐     ┌──────────────────┐
│   AuthClient    │────▶│    AuthState     │
│  (raps-kernel)  │     │  (computed on    │
└─────────────────┘     │   each request)  │
                        └──────────────────┘
         │
         │ references
         ▼
┌─────────────────┐     ┌──────────────────┐
│  ToolAuthMap    │────▶│ AuthRequirement  │
│    (static)     │     │     (enum)       │
└─────────────────┘     └──────────────────┘
```

## State Transitions

### 3-Legged Auth Flow

```text
┌───────────────┐
│   No Token    │
└───────┬───────┘
        │ auth_login called
        ▼
┌───────────────┐     browser opened
│  Initiating   │────────────────────┐
└───────┬───────┘                    │
        │ no browser                 │
        ▼                            ▼
┌───────────────┐            ┌───────────────┐
│ Device Code   │            │    Pending    │
│   Returned    │            │   Browser     │
└───────┬───────┘            └───────┬───────┘
        │                            │
        │ user completes             │ user completes
        │ on other device            │ in browser
        ▼                            ▼
┌───────────────┐            ┌───────────────┐
│  Token Saved  │◀───────────│ Callback Recv │
└───────┬───────┘            └───────────────┘
        │
        │ token expires
        ▼
┌───────────────┐
│    Expired    │───────▶ (back to No Token or refresh)
└───────────────┘
```

## Validation Rules

1. **Client ID**: Must be non-empty string, typically 32 characters
2. **Client Secret**: Must be non-empty string, typically 64 characters
3. **Token Expiry**: Tokens considered expired 60 seconds before actual expiry (buffer)
4. **Device Code Expiry**: Typically 900 seconds (15 minutes) from issuance

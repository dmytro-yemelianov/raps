# Research: MCP Server Native Authentication Support

**Feature**: 001-mcp-native-auth
**Date**: 2026-01-17

## Research Tasks

### 1. Device Code Flow in MCP Context

**Question**: How to integrate device code flow within MCP tool constraints (synchronous tool calls)?

**Finding**:
- MCP tools are request-response based; long-polling for device code completion is not suitable
- Device code flow requires separate initiation and completion phases
- Best approach: Tool returns device code + verification URL, user completes externally, subsequent `auth_status` call verifies completion

**Decision**: Implement `auth_login` tool that returns device code info for user to complete; polling happens implicitly when user calls `auth_status` or any 3-legged tool afterward.

**Rationale**: MCP tools should be stateless and quick-returning. Blocking for device code polling (up to 5 minutes) would break user experience.

**Alternatives Considered**:
- Blocking poll in tool: Rejected - would timeout/freeze AI assistant
- Separate `auth_poll` tool: Rejected - adds unnecessary complexity; existing token refresh handles this

### 2. Browser Detection for OAuth

**Question**: How to detect if browser is available for 3-legged OAuth?

**Finding**:
- `webbrowser` crate's `open()` returns `Result` - failure indicates no browser
- Environment variable `DISPLAY` absence on Linux indicates headless
- Docker/CI environments typically lack browser support

**Decision**: Attempt browser-based flow first; if `webbrowser::open()` fails, automatically fall back to device code flow and return instructions.

**Rationale**: Best UX for interactive users while maintaining headless support.

**Alternatives Considered**:
- Always use device code: Rejected - worse UX for desktop users
- Explicit `--headless` flag: Rejected - MCP tools don't have flags; auto-detection preferred

### 3. Tool Auth Requirements Mapping

**Question**: How to maintain mapping of tools to their required auth type?

**Finding**:
- Current server has 35 tools across categories
- OSS/Derivative tools: 2-legged only
- DM/ACC/Admin tools: 3-legged required
- Some admin tools: 2-legged (account-level operations)

**Decision**: Create static `AuthRequirement` enum and mapping in `auth_guidance.rs`:
```rust
enum AuthRequirement {
    TwoLegged,
    ThreeLegged,
    Either,
}
```

**Rationale**: Compile-time mapping ensures consistency; easy to update when adding tools.

**Alternatives Considered**:
- Runtime tool introspection: Rejected - no MCP standard for this
- Documentation only: Rejected - doesn't enable programmatic guidance

### 4. Instruction Content Structure

**Question**: How to structure auth guidance content for reuse?

**Finding**:
- Instructions needed in multiple contexts: missing creds, invalid creds, tool-specific guidance
- Content should be consistent across all auth-related responses
- Autodesk developer portal URL: https://aps.autodesk.com/

**Decision**: Create constants module with structured guidance:
- `SETUP_INSTRUCTIONS`: Full onboarding guide
- `MISSING_CLIENT_ID`, `MISSING_CLIENT_SECRET`: Specific missing credential guidance
- `THREE_LEGGED_PROMPT`: Suggestion to log in for DM/ACC access
- `TOOL_AUTH_HELP`: Category-based tool availability summary

**Rationale**: Single source of truth for all auth messaging; easy to update/localize.

**Alternatives Considered**:
- Inline strings: Rejected - duplication, hard to maintain
- External config file: Rejected - over-engineering for static content

### 5. Error Message Enhancement

**Question**: How to transform raw auth errors into actionable guidance?

**Finding**:
- Current errors: "Authentication failed: {e}" - not actionable
- Common failures: missing env vars, invalid credentials, expired tokens, network errors
- Each failure type has different remediation

**Decision**: Pattern-match on error types/messages to provide specific guidance:
- Missing env var → Show setup instructions
- 401 Unauthorized → Suggest credential check
- Network error → Suggest retry, check connectivity
- Token expired → Suggest re-authentication

**Rationale**: Transforms technical errors into user-actionable steps.

**Alternatives Considered**:
- Generic "contact support": Rejected - unhelpful for self-service tool
- Error codes only: Rejected - users need plain-language guidance

## Summary

All research tasks resolved. Key architectural decisions:
1. Device code flow as fallback with non-blocking tool design
2. Auto-detect browser availability, graceful fallback
3. Static tool-auth mapping in dedicated module
4. Centralized instruction content for consistency
5. Error-type pattern matching for actionable guidance

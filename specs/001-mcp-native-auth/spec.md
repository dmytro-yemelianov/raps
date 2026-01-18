# Feature Specification: MCP Server Native Authentication Support

**Feature Branch**: `001-mcp-native-auth`
**Created**: 2026-01-17
**Status**: Draft
**Input**: User description: "MCP server should support 2 and 3-legged auth natively, providing instructions how to fill them, and suggesting to perform 3-legged auth"

## Clarifications

### Session 2026-01-17

- Q: How should the system behave when no browser is available for 3-legged OAuth? → A: Provide URL + device code flow as fallback for headless environments
- Q: What behavior when Autodesk auth servers are unavailable? → A: Use cached tokens if valid; fail gracefully with retry guidance if no cache
- Q: Should MCP server support multiple concurrent users with different credentials? → A: Single-user only - one credential set per server instance (standard MCP pattern)

## User Scenarios & Testing *(mandatory)*

### User Story 1 - First-Time Setup Guidance (Priority: P1)

An AI assistant user opens a conversation with RAPS MCP server but has not configured any authentication credentials. They should receive clear, actionable instructions on what credentials are needed and how to obtain/configure them, rather than encountering cryptic authentication errors.

**Why this priority**: Without proper onboarding guidance, users will be blocked from using any MCP tools. This is the foundational experience that enables all other functionality.

**Independent Test**: Can be tested by starting MCP server without any credentials configured and verifying helpful guidance is returned instead of raw errors.

**Acceptance Scenarios**:

1. **Given** the MCP server starts with no credentials configured, **When** the AI assistant calls `auth_status`, **Then** the response includes step-by-step instructions for obtaining APS credentials and links to the Autodesk developer portal.
2. **Given** the user has incomplete credentials (e.g., client ID but no secret), **When** the AI assistant calls `auth_status`, **Then** the response identifies which specific credential is missing and how to provide it.
3. **Given** the user has invalid credentials, **When** the AI assistant calls `auth_test`, **Then** the response explains the authentication failure in user-friendly terms and suggests troubleshooting steps.

---

### User Story 2 - Proactive 3-Legged Auth Suggestions (Priority: P1)

When a user attempts to use a tool that requires 3-legged authentication (e.g., accessing BIM 360/ACC project data) but hasn't logged in, the MCP server should proactively suggest performing 3-legged authentication and provide clear instructions on how to do so.

**Why this priority**: Many valuable MCP tools (hub/project listing, folder browsing, issues, RFIs) require 3-legged auth. Users shouldn't discover this through failures; they should be guided to authenticate.

**Independent Test**: Can be tested by calling a 3-legged tool (like `hub_list`) without being logged in and verifying the response includes authentication guidance.

**Acceptance Scenarios**:

1. **Given** the user has valid 2-legged credentials but no 3-legged token, **When** they call `hub_list`, **Then** the response explains that 3-legged auth is required and provides instructions for running the login command.
2. **Given** the user's 3-legged token has expired, **When** they call a 3-legged tool, **Then** the response indicates the token expired and suggests re-authenticating.
3. **Given** the user asks `auth_status`, **When** 3-legged auth is not configured, **Then** the response proactively suggests performing 3-legged login if they need access to BIM 360/ACC data.

---

### User Story 3 - Native 3-Legged Auth Initiation (Priority: P2)

The MCP server provides a tool that can initiate the 3-legged OAuth flow directly, guiding the user through the browser-based authentication process without requiring them to use the CLI separately.

**Why this priority**: Reduces friction by keeping users within the MCP/AI assistant workflow instead of requiring them to switch to a terminal.

**Independent Test**: Can be tested by calling the auth initiation tool and verifying it returns the authorization URL and instructions for completing the flow.

**Acceptance Scenarios**:

1. **Given** the user needs 3-legged auth, **When** they call `auth_login`, **Then** the response includes the authorization URL and clear instructions for completing authentication in their browser.
2. **Given** the user has initiated 3-legged auth, **When** authentication completes successfully, **Then** the `auth_status` tool reflects the new logged-in state.
3. **Given** the user initiates 3-legged auth but their client app doesn't have a callback URL configured, **When** the auth flow fails, **Then** the response explains the issue and how to configure the callback URL in the Autodesk developer portal.
4. **Given** the user is in a headless environment (no browser), **When** they call `auth_login`, **Then** the response provides a device code and URL for manual authentication on another device.

---

### User Story 4 - Auth Scope Awareness (Priority: P3)

When authentication is configured, the MCP server should help users understand which tools are available to them based on their current authentication state and scopes, preventing confusion about why certain operations fail.

**Why this priority**: Helps users understand the auth model and self-diagnose permission issues without external documentation.

**Independent Test**: Can be tested by checking that `auth_status` includes information about which tool categories are accessible.

**Acceptance Scenarios**:

1. **Given** the user has only 2-legged auth, **When** they call `auth_status`, **Then** the response lists which tools are available (OSS, Derivative) and which require 3-legged auth (DM, ACC).
2. **Given** the user has both 2-legged and 3-legged auth, **When** they call `auth_status`, **Then** the response indicates full access to all MCP tools.

---

### Edge Cases

- What happens when the user's APS application is in trial/sandbox mode with limited scopes?
- When the authorization server (Autodesk) is unreachable: System uses cached tokens if valid; fails gracefully with retry guidance otherwise.
- What happens if the user provides credentials for the wrong APS region?
- How does the system behave when 3-legged tokens are stored but the refresh token is invalid?
- Multiple users per instance: Out of scope - MCP server supports single-user only (one credential set per instance), following standard MCP session-per-process pattern.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide clear, actionable instructions when authentication credentials are missing or invalid.
- **FR-002**: System MUST include step-by-step guidance for obtaining APS credentials, including links to the Autodesk developer portal.
- **FR-003**: System MUST clearly indicate which authentication type (2-legged or 3-legged) each tool requires.
- **FR-004**: System MUST proactively suggest 3-legged authentication when a user attempts to use a tool that requires it without being logged in.
- **FR-005**: System MUST provide a dedicated tool to initiate the 3-legged OAuth flow, supporting both browser-based and device code flow for headless environments.
- **FR-006**: System MUST explain authentication failures in user-friendly terms with specific troubleshooting guidance.
- **FR-007**: System MUST indicate which tool categories are accessible based on current authentication state.
- **FR-008**: System MUST handle expired tokens gracefully by suggesting re-authentication rather than showing cryptic errors.
- **FR-009**: System MUST provide feedback when authentication is successful, confirming what the user can now access.
- **FR-010**: System MUST continue operating with cached tokens when Autodesk auth servers are unreachable, and fail gracefully with retry guidance when no valid cached tokens exist.

### Key Entities

- **Authentication State**: Tracks current 2-legged and 3-legged authentication status, including token validity and expiration.
- **Auth Instructions**: Guidance content for credential setup, including developer portal links and step-by-step configuration.
- **Tool Auth Requirements**: Mapping of tools to their required authentication type (2-legged, 3-legged, or either).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can understand what authentication is needed and how to configure it within the first 2 minutes of using the MCP server.
- **SC-002**: Users encountering authentication errors receive actionable guidance in 100% of cases (no raw technical errors).
- **SC-003**: Users can complete 3-legged authentication setup without leaving the AI assistant conversation flow.
- **SC-004**: Users can determine which tools are available to them based on `auth_status` output without consulting external documentation.
- **SC-005**: First-time users with no prior APS experience can successfully authenticate and use basic MCP tools within 10 minutes.

## Assumptions

- Users have access to the Autodesk developer portal to create/manage their APS applications.
- The MCP server supports both browser-based OAuth (when available) and device code flow for headless environments where no browser is accessible.
- The RAPS CLI's existing token storage mechanism will be used for persisting 3-legged tokens.
- Environment variables (APS_CLIENT_ID, APS_CLIENT_SECRET) remain the primary method for 2-legged credential configuration.
- The callback URL for 3-legged auth uses localhost (standard RAPS behavior).
- Each MCP server instance serves a single user with one credential set (standard MCP session-per-process model).

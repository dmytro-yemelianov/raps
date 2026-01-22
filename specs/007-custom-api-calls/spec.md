# Feature Specification: Custom API Calls

**Feature Branch**: `007-custom-api-calls`
**Created**: 2026-01-22
**Status**: Draft
**Input**: User description: "to cli and mcp add option to run custom api calls, using current auth, appropriate method and endpoint"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Execute Custom GET Request via CLI (Priority: P1)

A developer needs to call an APS API endpoint that RAPS doesn't directly support as a built-in command. They want to make a GET request to retrieve data from a specific endpoint using their currently authenticated session.

**Why this priority**: This is the core functionality that enables users to access any APS API endpoint without waiting for RAPS to add explicit support. GET requests are the most common API operation for retrieving data.

**Independent Test**: Can be fully tested by executing a simple GET request to a known APS endpoint (e.g., `/userprofile/v1/users/@me`) and verifying the response is returned correctly with proper authentication.

**Acceptance Scenarios**:

1. **Given** the user has authenticated with RAPS, **When** they execute `raps api get /userprofile/v1/users/@me`, **Then** the system returns the user's profile data in the configured output format
2. **Given** the user has authenticated with RAPS, **When** they execute a GET request to a valid endpoint with query parameters, **Then** the query parameters are correctly appended to the request
3. **Given** the user is not authenticated, **When** they attempt to execute a custom API call, **Then** the system displays an authentication error with guidance on how to authenticate

---

### User Story 2 - Execute Custom API Request with Request Body (Priority: P2)

A developer needs to create or update a resource via a custom API endpoint using POST, PUT, or PATCH methods with a JSON request body.

**Why this priority**: Write operations are essential for full API coverage but are used less frequently than read operations. This enables users to perform any CRUD operation on APS resources.

**Independent Test**: Can be fully tested by executing a POST request to create a resource and verifying the response indicates successful creation.

**Acceptance Scenarios**:

1. **Given** the user has authenticated, **When** they execute `raps api post /endpoint --data '{"key":"value"}'`, **Then** the system sends the POST request with the JSON body and returns the response
2. **Given** the user has authenticated, **When** they execute a PUT request with a JSON file as input, **Then** the system reads the file contents and sends them as the request body
3. **Given** the user provides malformed JSON, **When** they attempt to execute the request, **Then** the system validates the JSON and displays a clear error before sending the request

---

### User Story 3 - Execute Custom API Calls via MCP Tool (Priority: P2)

An AI assistant (or automation tool) needs to make arbitrary API calls to APS through the MCP server when built-in tools don't cover the required endpoint.

**Why this priority**: MCP integration enables AI assistants and automation tools to interact with any APS API, significantly expanding automation capabilities. This has equal priority to CLI body support as both extend core functionality.

**Independent Test**: Can be fully tested by invoking the MCP tool with a simple GET endpoint and verifying the response is returned in the expected MCP format.

**Acceptance Scenarios**:

1. **Given** the MCP server is running with valid authentication, **When** an MCP client invokes `api_request` with method GET and a valid endpoint, **Then** the tool returns the API response as structured data
2. **Given** the MCP server is running, **When** an MCP client invokes `api_request` with POST method and a body, **Then** the request is sent with the correct content-type and body
3. **Given** the API returns an error status code, **When** the MCP tool processes the response, **Then** it returns the error in a structured format that includes status code and error message

---

### User Story 4 - Custom Headers and Authentication Override (Priority: P3)

A developer needs to add custom headers to their API request or use a specific authentication method for certain endpoints that require different headers.

**Why this priority**: Most API calls work with default headers and authentication. Custom headers are needed for edge cases like specific content types, versioning headers, or accessing endpoints with special requirements.

**Independent Test**: Can be fully tested by adding a custom header to a request and verifying via the response or request logging that the header was included.

**Acceptance Scenarios**:

1. **Given** the user is authenticated, **When** they execute `raps api get /endpoint --header "X-Custom: value"`, **Then** the custom header is included in the request alongside the authentication header
2. **Given** the user specifies multiple custom headers, **When** the request is sent, **Then** all custom headers are included in the request
3. **Given** the user wants to specify a different content type, **When** they provide `--header "Content-Type: application/xml"`, **Then** the request is sent with the specified content type

---

### Edge Cases

- What happens when the endpoint returns a non-JSON response (binary, XML, HTML)?
  - The system should detect the content type and handle appropriately: display text-based responses as-is, save binary responses to a file with `--output` flag
- How does the system handle API rate limiting (429 responses)?
  - The system should respect rate limiting by displaying the retry-after information and optionally auto-retrying with the existing retry logic
- What happens when the endpoint URL is malformed?
  - The system should validate the URL format before sending and display a clear validation error
- How are very large response bodies handled?
  - The system should stream large responses and support saving to file with `--output` flag to avoid memory issues
- What happens when the user provides both `--data` and `--data-file`?
  - The system should reject this as an invalid combination with a clear error message

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST provide a CLI command group `raps api` for executing custom API calls
- **FR-002**: System MUST support HTTP methods via subcommands: `raps api get`, `raps api post`, `raps api put`, `raps api patch`, `raps api delete`
- **FR-003**: System MUST automatically use the current authenticated session's access token for all requests
- **FR-004**: System MUST allow users to specify the API endpoint path (relative to the APS base URL)
- **FR-005**: System MUST restrict requests to APS domains only (developer.api.autodesk.com and related Autodesk API hosts); external URLs are not permitted
- **FR-006**: System MUST support query parameters via `--query` or `--param` flags
- **FR-007**: System MUST support request body input via `--data` flag for inline JSON
- **FR-008**: System MUST support request body input via `--data-file` flag to read from a file
- **FR-009**: System MUST validate JSON input before sending requests
- **FR-010**: System MUST support custom headers via `--header` flag (repeatable)
- **FR-011**: System MUST display response body in the configured output format (JSON, YAML, table, CSV)
- **FR-012**: System MUST display HTTP status code and response headers when `--verbose` is enabled
- **FR-013**: System MUST provide an MCP tool `api_request` for executing custom API calls
- **FR-014**: System MUST return appropriate exit codes for CLI (0 for 2xx success, non-zero for errors)
- **FR-015**: System MUST support saving response body to a file via `--output` flag
- **FR-016**: System MUST handle non-JSON responses gracefully (display as text or save as binary)
- **FR-017**: System MUST support the existing retry logic for transient failures (5xx errors, network issues)

### Key Entities

- **API Request**: Represents a custom API call with method, endpoint, headers, query parameters, and optional body
- **API Response**: Contains status code, headers, body content, and content type
- **Authentication Context**: The current session's access token and token type used for Authorization header

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can successfully call any valid APS API endpoint within 5 seconds for typical responses
- **SC-002**: 100% of supported HTTP methods (GET, POST, PUT, PATCH, DELETE) work correctly for both CLI and MCP
- **SC-003**: Users can complete a custom API call with the same number of steps as using curl directly (endpoint + method + optional body)
- **SC-004**: Error messages clearly indicate the issue (authentication, validation, network, API error) with actionable guidance
- **SC-005**: Response output matches the globally configured format without additional user intervention
- **SC-006**: MCP tool response time is equivalent to direct CLI execution (within 10% overhead)

## Clarifications

### Session 2026-01-22

- Q: Should custom API calls allow requests to any arbitrary URL, or be restricted to APS domains? → A: APS domains only - no external URLs permitted
- Q: Should HTTP methods be specified via subcommands or a --method flag? → A: Subcommands (`raps api get`, `raps api post`, etc.)

## Assumptions

- The existing authentication system and token refresh mechanism will be reused without modification
- The base URL for APS APIs follows the existing pattern already configured in RAPS
- The existing output format flags (`--output-format`, global config) apply to custom API responses
- The existing HTTP client with retry logic will be leveraged for custom API calls
- Query parameters follow standard URL encoding conventions

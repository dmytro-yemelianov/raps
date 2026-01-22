# Tasks: Custom API Calls

**Input**: Design documents from `/specs/007-custom-api-calls/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Tests included based on plan.md (cargo test, assert_cmd integration tests)

**Organization**: Tasks grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md structure:
- CLI commands: `raps-cli/src/commands/`
- MCP server: `raps-cli/src/mcp/`
- Kernel utilities: `raps-kernel/src/`
- Tests: `raps-cli/tests/`

---

## Phase 1: Setup

**Purpose**: Module scaffolding and exports

- [ ] T001 Create empty api.rs command module in raps-cli/src/commands/api.rs
- [ ] T002 Add api module export in raps-cli/src/commands/mod.rs
- [ ] T003 Add Api variant to Commands enum in raps-cli/src/main.rs

**Checkpoint**: Project compiles with empty api command stub

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure shared by all user stories - domain validation and data types

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T004 Add ALLOWED_DOMAINS constant and is_allowed_url() validation function in raps-kernel/src/http.rs
- [ ] T005 [P] Define HttpMethod enum in raps-cli/src/commands/api.rs
- [ ] T006 [P] Define ApiRequest struct with method, endpoint, headers, query_params, body fields in raps-cli/src/commands/api.rs
- [ ] T007 [P] Define ApiResponse struct with status_code, headers, content_type, body in raps-cli/src/commands/api.rs
- [ ] T008 [P] Define ResponseBody enum (Json, Text, Binary variants) in raps-cli/src/commands/api.rs
- [ ] T009 [P] Define ApiError struct for error responses in raps-cli/src/commands/api.rs
- [ ] T010 Add unit tests for is_allowed_url() function in raps-kernel/src/http.rs

**Checkpoint**: Foundation ready - domain validation works, data types defined

---

## Phase 3: User Story 1 - Execute Custom GET Request via CLI (Priority: P1) 🎯 MVP

**Goal**: Users can execute GET requests to any APS endpoint with query parameters and output formatting

**Independent Test**: Run `raps api get /userprofile/v1/users/@me` and verify response with proper auth

### Implementation for User Story 1

- [ ] T011 [US1] Define ApiCommands enum with Get variant (endpoint, query, header, output, verbose args) in raps-cli/src/commands/api.rs
- [ ] T012 [US1] Implement parse_key_value() helper for --query KEY=VALUE parsing in raps-cli/src/commands/api.rs
- [ ] T013 [US1] Implement parse_header() helper for --header KEY:VALUE parsing in raps-cli/src/commands/api.rs
- [ ] T014 [US1] Implement build_url() to construct full URL from base + endpoint + query params in raps-cli/src/commands/api.rs
- [ ] T015 [US1] Implement execute_request() async function for HTTP GET with auth token in raps-cli/src/commands/api.rs
- [ ] T016 [US1] Implement handle_response() to detect content-type and parse response body in raps-cli/src/commands/api.rs
- [ ] T017 [US1] Implement format_output() to display response using OutputFormat (json/yaml/table/csv) in raps-cli/src/commands/api.rs
- [ ] T018 [US1] Implement verbose output showing HTTP status and headers in raps-cli/src/commands/api.rs
- [ ] T019 [US1] Implement --output flag to save response to file in raps-cli/src/commands/api.rs
- [ ] T020 [US1] Add authentication check with helpful error message when not logged in in raps-cli/src/commands/api.rs
- [ ] T021 [US1] Wire up ApiCommands::execute() dispatch in raps-cli/src/main.rs
- [ ] T022 [US1] Add integration test for `raps api get --help` in raps-cli/tests/api_tests.rs
- [ ] T023 [US1] Add integration test for GET request validation errors in raps-cli/tests/api_tests.rs

**Checkpoint**: User Story 1 complete - GET requests work with query params, output formats, auth

---

## Phase 4: User Story 2 - Execute Custom API Request with Body (Priority: P2)

**Goal**: Users can execute POST/PUT/PATCH requests with JSON body from inline or file

**Independent Test**: Run `raps api post /oss/v2/buckets --data '{"bucketKey":"test"}'` and verify request sent

### Implementation for User Story 2

- [ ] T024 [US2] Add Post, Put, Patch, Delete variants to ApiCommands enum with --data and --data-file args in raps-cli/src/commands/api.rs
- [ ] T025 [US2] Implement read_body_from_file() to load JSON from --data-file path in raps-cli/src/commands/api.rs
- [ ] T026 [US2] Implement validate_json() to parse and validate JSON body before sending in raps-cli/src/commands/api.rs
- [ ] T027 [US2] Add clap conflicts_with constraint between --data and --data-file in raps-cli/src/commands/api.rs
- [ ] T028 [US2] Implement execute_request() for POST/PUT/PATCH with Content-Type: application/json header in raps-cli/src/commands/api.rs
- [ ] T029 [US2] Implement execute_request() for DELETE method (no body) in raps-cli/src/commands/api.rs
- [ ] T030 [US2] Add validation error when body provided for GET/DELETE in raps-cli/src/commands/api.rs
- [ ] T031 [US2] Add integration test for POST with inline --data in raps-cli/tests/api_tests.rs
- [ ] T032 [US2] Add integration test for --data and --data-file mutual exclusion in raps-cli/tests/api_tests.rs

**Checkpoint**: User Story 2 complete - POST/PUT/PATCH/DELETE work with JSON bodies

---

## Phase 5: User Story 3 - Execute Custom API Calls via MCP Tool (Priority: P2)

**Goal**: MCP clients can invoke api_request tool for arbitrary APS API calls

**Independent Test**: Invoke api_request MCP tool with GET /oss/v2/buckets and verify structured response

### Implementation for User Story 3

- [ ] T033 [US3] Add "api_request" to TOOLS constant in raps-cli/src/mcp/tools.rs
- [ ] T034 [US3] Add get_tools() entry for api_request with JSON schema (method, endpoint, body, headers, query) in raps-cli/src/mcp/server.rs
- [ ] T035 [US3] Implement api_request() method on RapsServer with parameter parsing in raps-cli/src/mcp/server.rs
- [ ] T036 [US3] Add api_request dispatch case in dispatch_tool() match in raps-cli/src/mcp/server.rs
- [ ] T037 [US3] Implement MCP response formatting (success: bool, status, data/error) in raps-cli/src/mcp/server.rs
- [ ] T038 [US3] Add validation for method enum values in MCP tool in raps-cli/src/mcp/server.rs
- [ ] T039 [US3] Add domain validation for endpoint in MCP tool in raps-cli/src/mcp/server.rs
- [ ] T040 [US3] Add body-not-allowed validation for GET/DELETE in MCP tool in raps-cli/src/mcp/server.rs

**Checkpoint**: User Story 3 complete - MCP api_request tool works for all methods

---

## Phase 6: User Story 4 - Custom Headers (Priority: P3)

**Goal**: Users can add custom headers to requests via --header flag (repeatable)

**Independent Test**: Run `raps api get /endpoint --header "X-Custom:value"` and verify header included

### Implementation for User Story 4

- [ ] T041 [US4] Implement repeatable --header flag collection in clap args in raps-cli/src/commands/api.rs
- [ ] T042 [US4] Add custom headers to reqwest RequestBuilder in execute_request() in raps-cli/src/commands/api.rs
- [ ] T043 [US4] Ensure Authorization header cannot be overridden by user in raps-cli/src/commands/api.rs
- [ ] T044 [US4] Add custom headers support in MCP api_request tool in raps-cli/src/mcp/server.rs
- [ ] T045 [US4] Add integration test for multiple custom headers in raps-cli/tests/api_tests.rs

**Checkpoint**: User Story 4 complete - Custom headers work for CLI and MCP

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Error handling, edge cases, and documentation

- [ ] T046 [P] Implement exit code mapping (0=2xx, 1=4xx/5xx, 2=validation, 10=auth) in raps-cli/src/commands/api.rs
- [ ] T047 [P] Handle non-JSON responses: text display, binary save-to-file in raps-cli/src/commands/api.rs
- [ ] T048 [P] Add retry logic integration for 5xx and network errors in raps-cli/src/commands/api.rs
- [ ] T049 [P] Add rate limiting (429) handling with retry-after display in raps-cli/src/commands/api.rs
- [ ] T050 Run quickstart.md validation - verify all examples work
- [ ] T051 Run cargo clippy and fix any warnings
- [ ] T052 Run cargo fmt to ensure code formatting

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - US1 (P1): MVP - implement first
  - US2 (P2): Can start after US1 or in parallel
  - US3 (P2): Can start after US1 or in parallel (different files)
  - US4 (P3): Can start after US1 or in parallel
- **Polish (Phase 7)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: After Foundational - No dependencies on other stories
- **User Story 2 (P2)**: After Foundational - Extends US1 execute_request() but independently testable
- **User Story 3 (P2)**: After Foundational - Different files (mcp/), can parallel with US1/US2
- **User Story 4 (P3)**: After Foundational - Extends US1 header handling but independently testable

### Within Each User Story

- Data types before functions
- Helper functions before main logic
- Core implementation before error handling
- Integration tests after implementation

### Parallel Opportunities

**Phase 2 (Foundational)**:
```
T005, T006, T007, T008, T009 can run in parallel (different struct definitions)
```

**Phase 3-6 (User Stories)**:
```
US3 (MCP) can run in parallel with US1/US2 (different file: mcp/server.rs vs commands/api.rs)
```

**Phase 7 (Polish)**:
```
T046, T047, T048, T049 can run in parallel (independent error handling aspects)
```

---

## Parallel Example: Foundational Phase

```bash
# Launch data type definitions in parallel:
Task: "Define HttpMethod enum in raps-cli/src/commands/api.rs"
Task: "Define ApiRequest struct in raps-cli/src/commands/api.rs"
Task: "Define ApiResponse struct in raps-cli/src/commands/api.rs"
Task: "Define ResponseBody enum in raps-cli/src/commands/api.rs"
Task: "Define ApiError struct in raps-cli/src/commands/api.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (3 tasks)
2. Complete Phase 2: Foundational (7 tasks)
3. Complete Phase 3: User Story 1 (13 tasks)
4. **STOP and VALIDATE**: Test `raps api get` works
5. Deploy/demo if ready - users can already make GET requests!

### Incremental Delivery

1. Setup + Foundational → Foundation ready (10 tasks)
2. Add User Story 1 → MVP: GET requests work (23 tasks total)
3. Add User Story 2 → POST/PUT/PATCH/DELETE work (32 tasks total)
4. Add User Story 3 → MCP integration works (40 tasks total)
5. Add User Story 4 → Custom headers work (45 tasks total)
6. Polish → Production ready (52 tasks total)

### Parallel Team Strategy

With multiple developers after Foundational:

- **Developer A**: User Story 1 (CLI GET) + User Story 2 (CLI body methods)
- **Developer B**: User Story 3 (MCP tool) + User Story 4 (custom headers)

Different files, minimal conflicts.

---

## Notes

- [P] tasks = different files or independent code paths
- [Story] label maps task to specific user story for traceability
- Each user story is independently testable after completion
- Reuse existing raps-kernel auth and HTTP client - no new crates needed
- Domain validation (is_allowed_url) is security-critical - test thoroughly
- MCP tool shares core logic with CLI but has different I/O format

# Test Audit

Generated: 2026-03-05

## Existing test files

### raps-cli/tests/ (CLI integration tests, assert_cmd)
- admin_commands.rs — help text and arg-parsing tests for admin subcommands
- auth_commands.rs, bucket_commands.rs, ... — command-level tests for each domain
- live_api_tests.rs — real APS API tests (#[ignore], require APS_CLIENT_ID)
- mcp_auth_tests.rs — MCP tool availability tests with insta snapshots

### raps-admin/tests/integration/
- add_user_tests.rs — BulkExecutor unit tests (mock closures, no HTTP)
- dry_run_tests.rs, resume_tests.rs, remove_user_tests.rs, update_role_tests.rs, folder_rights_tests.rs

### raps-acc/tests/
- project_users_role_test.rs — HTTP round-trip tests using TestServer

### raps-kernel/src/auth/tests.rs — AuthClient unit tests with TestServer

## What is covered

| Layer | Covered |
|-------|---------|
| CLI arg parsing / help text | Partial (help text only, no full workflow) |
| Bulk executor behavior | Yes (raps-admin/tests) |
| API client serialization | Partial (role_id None/Some) |
| HTTP round-trip for add_user | Yes (project_users_role_test.rs) |
| HTTP round-trip for remove_user | No |
| HTTP round-trip for archive_project | No |
| API call trace / sequence verification | No |
| Scenario tests (full workflow) | No |
| CLI help structure snapshot | No |
| Dry-run produces zero writes | Partial (BulkExecutor level only) |
| Edge cases (invalid role, project archived) | No |

## Missing test layers

1. TraceRecorder in raps-mock (infrastructure)
2. tests/operations/ — per-operation HTTP round-trip tests
3. tests/scenarios/ — full workflow tests with API trace assertion
4. tests/cli/help_structure.rs — snapshot of command tree
5. tests/snapshots/api_traces/ — golden API call sequences
6. Edge cases: user not member, project archived, invalid role, rate limit
7. TEST_COVERAGE.md (to be generated after implementation)

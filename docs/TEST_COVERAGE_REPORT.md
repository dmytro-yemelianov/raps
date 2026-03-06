# RAPS Test Coverage Report

> Generated: 2026-03-06

## Test counts

| Category | Files | Tests |
|---|---|---|
| Smoke / binary invocation | 2 | ~20 |
| Help & snapshot (per-command) | ~30 | 150+ |
| Scenario / E2E with mock | 2 | 6 |
| Operation unit tests | 5 | ~15 |
| Integration (raps-admin crate) | 5 | 40+ |
| Live API (require credentials) | 1 | 15+ |
| Output format / consistency | 5 | 30+ |
| Inline src/ unit tests | 4 | 50+ |
| MCP auth / guidance | 1 | 25+ |
| Other (redaction, logging, exit codes) | 5 | ~15 |
| **Total** | **~51** | **400+** |

---

## Smoke / Binary Invocation

### `tests/smoke_cli.rs`
Runs the binary directly via `std::process::Command`:
- `help_command_runs` — `raps --help` exits without panic
- `config_profile_list_succeeds_without_credentials` — no-auth command works
- `bucket_info_missing_args_returns_usage_error` — missing arg → exit 2

### `tests/command_dispatch_test.rs`
Full command dispatch verification (no panic, graceful failure, no wrong routing):
- `test_all_commands_dispatch_no_panic` — fires all major subcommands
- `test_all_help_flags_work` — `--help` for every subcommand
- `test_auth_not_routed_to_completions` — regression for misrouting
- `test_config_dispatch_isolation` — state isolation regression
- Per-command no-panic: auth test/login/logout, bucket list, object list, translate status, hub list, webhook list, da engine list, plugin list, config profile list
- Exit code checks: not 101 (panic) for auth test, bucket list

---

## Help & Snapshot Tests (per-command files in `raps-cli/tests/`)

Every top-level command has a `*_commands.rs` file running `--help` against the compiled binary and validating arg requirements:

| File | Commands covered |
|---|---|
| `auth_commands.rs` | auth, auth test/login/logout/status/whoami/inspect |
| `hub_commands.rs` | hub, hub list/info |
| `bucket_commands.rs` | bucket create/list/info/delete, output format flags |
| `object_commands.rs` | object upload-batch/download/list/delete/signed-url |
| `translate_commands.rs` | translate start/status/manifest/derivatives/download/preset |
| `da_commands.rs` | da engines/appbundles/activities/appbundle-create/delete/activity-create/delete/run |
| `pipeline_commands.rs` | pipeline run/validate/sample/create, dry-run with tempfile |
| `webhook_commands.rs` | webhook list/create/delete/events/test, events works without auth |
| `issue_commands.rs` | issue list/create/update/types/comment, PROJECT_ID validation |
| `job_commands.rs` | job status/list/cancel, ID/MACHINE_ID validation |
| `admin_commands.rs` | admin user/folder/project/operation/company-list, add-to-all-projects flags |
| `swarm_commands.rs` | swarm status/metrics/queue/resume/audit/reset, worker (redis-gated) |
| `report_commands.rs` | report rfi-summary/issues-summary/submittals-summary/checklists-summary/assets-summary |
| `rfi_commands.rs` | rfi list/get/create/update, status/since filter |
| `config_commands.rs` | config profile create/list/use/delete/export/import/get/set |
| `acc_commands.rs` | acc asset/submittal/checklist, PROJECT_ID validation |
| `inspect_commands.rs` | inspect zip, no-credentials failure |
| `generate_commands.rs` | generate files, actual file creation with tempdir |
| `schema_commands.rs` | schema list/all/generate, JSON output |
| `cache_commands.rs` | cache stats/dir/clear/prune |
| `doctor_commands.rs` | doctor, JSON/YAML output format |
| `demo_commands.rs` | demo bucket-lifecycle/model-pipeline/batch-processing |
| `plugin_commands.rs` | plugin list/enable/disable/info/alias |
| `reality_commands.rs` | reality create/upload/process/status/result, PHOTOSCENE_ID |
| `folder_commands.rs` | folder create/delete/rights, no-credentials failure |
| `item_commands.rs` | item info/versions/create-from-oss/delete/rename |
| `project_commands.rs` | project list/info, HUB_ID/PROJECT_ID validation |
| `template_commands.rs` | template create/info/update/archive |
| `cli_general.rs` | version, --help, -h, unknown command, global flags, completions |

### `raps-cli/tests/cli/help_structure.rs`
Insta snapshots of full help text for admin command tree (detects regressions in help wording):
- `admin --help` snapshot
- `admin user --help` snapshot
- `admin project --help` snapshot
- `admin user add-to-all-projects --help` snapshot

---

## Scenario Tests — E2E with Mock Server

### `raps-cli/tests/scenarios/admin_add_user_all_projects.rs`
Full end-to-end: CLI invokes operation → hits mock TestServer → asserts API trace.
- `test_add_user_to_all_projects_trace_matches_snapshot` — exact API call sequence (POST per project) verified via insta snapshot
- `test_add_user_all_projects_result_counts` — success/fail/skip counts validated

### `raps-cli/tests/scenarios/admin_dry_run.rs`
- `test_dry_run_guarantees_zero_write_api_calls` — dry-run mode makes zero POST/PATCH/DELETE calls; verified by mock expectation count

### `raps-cli/tests/scenarios/admin_archive_project.rs`
- `test_admin_archive_project_trace_matches_snapshot` — PATCH call sequence snapshot

### `raps-cli/tests/scenarios/admin_remove_user.rs`
- `test_admin_remove_user_trace_matches_snapshot` — DELETE call sequence snapshot

### `raps-cli/tests/scenarios/admin_cli_scenarios.rs` (CLI-level)
- `test_admin_user_add_to_all_projects_cli` — full CLI binary invocation with mock server
- `test_admin_user_add_account_not_found_exit_code_4` — exit code 4 for Resource Not Found

---

## Operation Unit Tests (`raps-cli/tests/operations/`)

Tests individual async operation functions against a mock HTTP server (no CLI binary involved):

| File | Operation | Tests |
|---|---|---|
| `add_user_to_project.rs` | `AddProjectUserRequest` | POST endpoint, role_id serialization, optional role omission |
| `remove_user_from_project.rs` | DELETE user | DELETE endpoint, 204 No Content |
| `archive_project.rs` | PATCH project | PATCH with status=archived, UpdateProjectRequest |
| `add_user_to_all_projects.rs` | bulk add | API trace snapshot |
| `edge_cases.rs` | bulk executor | Duplicate user, empty filter edge cases |

---

## Integration Tests — raps-admin crate (`raps-admin/tests/integration/`)

Tests the `BulkExecutor` abstraction in isolation, heavy coverage of concurrency and error recovery:

| File | Tests |
|---|---|
| `add_user_tests.rs` | 10-item success, mixed results, duplicate detection, retry on transient failure, dry-run, concurrency limit (max 2), result detail structure |
| `remove_user_tests.rs` | 10-item removal, user-not-in-project skip, mixed results, dry-run, skip reason tracking, concurrency enforcement |
| `dry_run_tests.rs` | All items skipped, "dry-run mode" skip reason, progress tracking, project info preservation, zero-item edge case |
| `folder_rights_tests.rs` | Folder rights assignment via BulkExecutor |
| `update_role_tests.rs` | Role update operations |
| `resume_tests.rs` | Resume/checkpoint of partial runs |

---

## Live API Tests (`raps-cli/tests/live_api_tests.rs`)

All marked `#[ignore]`, require `APS_CLIENT_ID` + `APS_CLIENT_SECRET`:

- `test_live_auth_test`, `test_live_auth_status`
- `test_live_bucket_list` (table + JSON), `test_live_bucket_info_not_found` (404 handling)
- `test_live_object_list_bucket_not_found`
- `test_live_webhook_events`, `test_live_webhook_list`
- `test_live_da_engines/appbundles/activities`
- `test_live_translate_status_invalid_urn`
- `test_live_output_json/yaml`, `test_live_verbose_flag`, `test_live_debug_flag`, `test_live_custom_timeout`
- `test_live_bucket_workflow` — create → list → delete (only real end-to-end workflow test)

Run with: `cargo test --ignored -- live`

---

## Coverage Summary

| CLI Command | Coverage type | Mock? |
|---|---|---|
| auth | help-snapshot, live, dispatch smoke, MCP auth unit | Both |
| hub | help-snapshot, live, dispatch smoke | Both |
| bucket | help-snapshot, live, dispatch smoke | Both |
| object | help-snapshot, live, dispatch smoke | Both |
| translate | help-snapshot, live, dispatch smoke | Both |
| da | help-snapshot, live, dispatch smoke | Both |
| webhook | help-snapshot, live, dispatch smoke | Both |
| **admin** | **help-snapshot, scenario E2E, operation unit, integration (BulkExecutor)** | **Mock (TestServer)** |
| pipeline | help-snapshot, dry-run with tempfile | Partial |
| config | help-snapshot, dispatch smoke | None |
| report | help-snapshot, unit (serialization, filtering) | None |
| rfi | help-snapshot, unit (CSV parsing, serialization) | None |
| issue | help-snapshot | None |
| job | help-snapshot | None |
| swarm | help-snapshot (redis-gated) | None |
| acc | help-snapshot | None |
| inspect | help-snapshot | None |
| generate | help-snapshot, actual file creation | Partial |
| schema | help-snapshot, JSON output | None |
| cache | help-snapshot, dir/clear | None |
| doctor | help-snapshot, JSON/YAML output | None |
| demo | help-snapshot | None |
| plugin | help-snapshot, dispatch smoke | None |
| reality | help-snapshot | None |
| folder | help-snapshot | None |
| item | help-snapshot | None |
| project | help-snapshot | None |
| template | help-snapshot | None |
| **status** | **none** | — |
| **init** | **none** | — |
| mcp | MCP auth unit (auth_guidance module) | Unit only |

---

## Key Gaps

1. **`raps status` and `raps init`** — new in 5.2.0, no tests at all
2. **Single live E2E workflow** — only `test_live_bucket_workflow` does a create→use→delete cycle; no similar workflow for translate, admin, or hub
3. **Scenario coverage concentrated in admin** — only 5 scenario files, all admin operations; no scenarios for translate, pipeline, webhook, hub
4. **issue/job/swarm/acc/reality/folder/item/project/template** — help-snapshot only, no behaviour tests

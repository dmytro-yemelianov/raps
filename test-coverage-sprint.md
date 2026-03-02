# Test Coverage Sprint — Master Plan

## Phase 1: API Client Integration Tests ✅
- Completed in prior session
- Added integration tests across API client crates
- Workspace: 0 → 1199 tests

## Phase 2: Contract Snapshots + Mock Fixes + Initial raps-cli ✅
- Step 1: Contract snapshot tests (+6 tests, +5 fixtures) — raps-da, raps-webhooks
- Step 2: Mock server fixes — create_bucket, create_webhook stateless responses
- Step 3: raps-cli unit tests (+38 tests) — pipeline.rs, csv_ops.rs, swarm.rs, command_tree.rs, output/tests.rs
- Workspace: 1199 → 1293 tests

## Phase 3: raps-cli Pure Function Tests ✅
- api.rs: 13 tests (parse_key_value, parse_header, build_url, categorize_error, extract_error_message)
- auth.rs: 8 tests (mask_string, LoginPreset::scopes)
- object/mod.rs: 8 tests (format_size, truncate_str)
- bucket.rs: 6 tests (chrono_humanize)
- webhook.rs: 6 tests (truncate_str, output serialization)
- Workspace: 1293 → 1334 tests

## Phase 4: Shell, Formatter & MCP Auth Tests ✅
- shell/tests.rs: 8 tests (completer/hinter edge cases — unknown commands, flags, case sensitivity, empty/partial hints)
- output/formatter.rs: 10 tests (format_value_for_table, format_value_for_csv — null, bool, number, string, array, object)
- commands/doctor.rs: 4 tests (format_size — bytes, KB, MB, GB)
- commands/cache.rs: 4 tests (format_size — bytes, KB, MB, GB)
- mcp/auth_guidance.rs: 16 tests (AuthState, get_tool_auth_requirement, format_error_guidance, get_tool_availability_summary)
- commands/schema.rs: 4 tests (registry validation — not empty, no duplicates, categories, generators)
- raps-cli: 781 tests (255 unit + 526 integration), 0 failures

## Phase 5: MCP Definitions, Dashboard Utils & Translation Presets ✅
- mcp/definitions.rs: 8 tests (schema() helper, get_tools() registry — structure, uniqueness, descriptions, core tools)
- dashboard/util.rs: 14 tests (format_timestamp, format_size, status_color, da_status_color — behind `dashboard` feature)
- translate/presets.rs: 5 tests (default_presets — not empty, no duplicates, format validation, serialization roundtrip)
- raps-cli: 835 tests (268 unit + 526 integration + 41 dashboard), 0 failures

## Current State
- **All 5 phases complete**
- **raps-cli: 835 tests, 0 failures** (with `--features dashboard`)
- **Codecov: ~25% → improved** (exact number pending CI run)

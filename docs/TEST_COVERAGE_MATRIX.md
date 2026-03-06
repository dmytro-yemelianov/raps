# RAPS CLI — Test Coverage Matrix

> Generated: 2026-03-06
> Tool: `cargo llvm-cov --package raps-cli --summary-only`
> Overall: **25.4% line coverage** (most uncovered code requires live APS credentials)

Columns: **Lines** = line coverage %, **Fns** = function coverage %

---

## acc

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/acc/mod.rs` | 0% | 0% | help-snapshot only |
| `commands/acc/assets.rs` | 0% | 0% | help-snapshot only |
| `commands/acc/checklists.rs` | 0% | 0% | help-snapshot only |
| `commands/acc/submittals.rs` | 0% | 0% | help-snapshot only |

---

## admin

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/admin/mod.rs` | **61.7%** | 58.6% | mock scenarios + operation tests |
| `commands/admin/csv_ops.rs` | 21.2% | 52.6% | unit tests (csv parsing) |
| `commands/admin/user.rs` | 14.1% | 36.8% | mock scenarios; bulk logic in raps-admin crate |
| `commands/admin/project.rs` | 2.0% | 9.1% | help-snapshot only |
| `commands/admin/folder.rs` | 0% | 0% | help-snapshot only |
| `commands/admin/operations.rs` | **53.3%** | 81.8% | unit tests (format_status, display_bulk_result table/JSON), scenario tests (list empty table/JSON/filter/yaml, status/resume/cancel no-ops, unknown UUID) |

---

## api

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/api.rs` | 41.9% | 59.0% | arg-validation tests |

---

## auth

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/auth.rs` | 27.4% | 48.5% | help-snapshot, dispatch smoke, MCP auth unit tests |

---

## bucket

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/bucket.rs` | **57.2%** | 69.4% | help-snapshot, arg-validation, output-format tests |

---

## cache

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/cache.rs` | 37.0% | 90.0% | help-snapshot, dir/clear tests |

---

## config

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/config/mod.rs` | 36.4% | 66.7% | help-snapshot, dispatch smoke |
| `commands/config/profiles.rs` | 8.2% | 20.0% | help-snapshot only |
| `commands/config/config_ops.rs` | **79.7%** | 100% | smoke tests (CLI binary, profile create/use/set/get round-trips) |
| `commands/config/context.rs` | **75.4%** | 87.5% | scenario tests (show/set/clear round-trips, env var source, unknown key) |

---

## da (Design Automation)

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/da/engines.rs` | 21.6% | 100% | help-snapshot, dispatch smoke |
| `commands/da/mod.rs` | 7.3% | 66.7% | help-snapshot |
| `commands/da/activities.rs` | **65.1%** | 82.4% | scenario tests (list/create/delete, JSON file, missing fields); wait/alias-error paths require live API |
| `commands/da/appbundles.rs` | **49.7%** | 82.4% | scenario tests (list/create/delete round-trips) |
| `commands/da/workitems.rs` | **43.6%** | 100% | scenario tests (list, run qualified/unqualified, status, input/output args); wait polling requires live API |

---

## demo

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/demo.rs` | 0% | 0% | help-snapshot only |

---

## doctor

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/doctor.rs` | **52.8%** | 95.2% | help-snapshot, JSON/YAML output tests |

---

## folder

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/folder.rs` | 22.7% | 46.2% | help-snapshot, no-credentials failure |

---

## generate

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/generate.rs` | **95.5%** | 100% | help-snapshot + actual file generation with tempdir |

---

## hub

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/hub.rs` | 43.5% | 65.0% | help-snapshot, mock scenario (list JSON + table), dispatch smoke |

---

## init *(new in 5.2.0)*

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/init.rs` | 25.5% | 46.7% | help-snapshot, smoke, unit tests (export_line, shell_rc_filename); wizard steps require interactive terminal |

---

## inspect

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/inspect.rs` | 46.7% | 78.6% | help-snapshot, no-credentials failure |

---

## interactive shell

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/interactive.rs` | 1.8% | 12.5% | no tests (requires readline/TTY) |

---

## issue

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/issue/mod.rs` | 21.8% | 50.0% | help-snapshot, arg-validation |
| `commands/issue/transitions.rs` | 46.7% | 70.0% | help-snapshot |
| `commands/issue/attachments.rs` | 20.6% | 25.0% | help-snapshot |
| `commands/issue/comments.rs` | 12.5% | 12.5% | help-snapshot |
| `commands/issue/crud.rs` | 10.3% | 10.7% | help-snapshot only |

---

## item

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/item.rs` | 6.6% | 21.7% | help-snapshot only |

---

## job

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/job.rs` | **44.6%** | 88.9% | colorize_state unit tests + smoke tests (no-creds + fake-creds handler entry); poll/table/stop paths require live Fly.io API |

---

## object

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/object/mod.rs` | 37.3% | 93.3% | help-snapshot, arg-validation |
| `commands/object/download.rs` | 3.1% | 13.3% | help-snapshot only |
| `commands/object/upload.rs` | 0% | 0% | help-snapshot only |
| `commands/object/copy.rs` | **40.3%** | 64.7% | scenario tests (batch-copy empty bucket, batch-rename no matches, copy/rename nonexistent source error paths, arg-validation); full copy round-trip requires upload (panics in debug mode due to clap positional arg ordering) |

---

## pipeline

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/pipeline.rs` | **61.0%** | 75.0% | help-snapshot, dry-run with tempfile, validate/sample |

---

## plugin

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/plugin.rs` | 8.6% | 21.1% | help-snapshot, dispatch smoke |

---

## project

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/project.rs` | 22.0% | 66.7% | help-snapshot, arg-validation |

---

## reality (Reality Capture)

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/reality.rs` | 0% | 0% | help-snapshot only (clap parses but handler never runs) |

---

## report

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/report/mod.rs` | 13.9% | 33.3% | help-snapshot, unit tests (serialization, filtering) |
| `commands/report/extended_reports.rs` | 0% | 0% | help-snapshot only |
| `commands/report/issues_report.rs` | 0% | 0% | help-snapshot only |
| `commands/report/rfi_report.rs` | 0% | 0% | help-snapshot only |

---

## rfi

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/rfi/mod.rs` | 4.9% | 20.0% | help-snapshot, unit tests (CSV parsing, serialization) |
| `commands/rfi/crud.rs` | 0% | 0% | help-snapshot only |

---

## schema

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/schema.rs` | **96.2%** | 100% | help-snapshot, list/all/generate with JSON output validation |

---

## status *(new in 5.2.0)*

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/status.rs` | 45.3% | 65.2% | unit tests (helpers), help-snapshot, mock scenario (JSON output); render path requires live auth |

---

## swarm

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/swarm.rs` | 27.3% | 37.0% | help-snapshot only (redis feature-gated) |

---

## template

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/template.rs` | 7.3% | 44.4% | help-snapshot only |

---

## translate

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/translate/mod.rs` | 27.2% | 60.0% | help-snapshot, mock scenario (start/status/manifest no-panic) |
| `commands/translate/presets.rs` | 24.7% | 30.4% | help-snapshot |
| `commands/translate/translations.rs` | 6.7% | 35.0% | help-snapshot only |
| `commands/translate/metadata.rs` | 0% | 0% | help-snapshot only |

---

## webhook

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `commands/webhook.rs` | 23.9% | 51.2% | help-snapshot, mock scenario (list + missing-url error) |

---

## MCP server

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `mcp/auth_guidance.rs` | 88.0% | 91.7% | comprehensive unit tests (insta snapshots) |
| `mcp/definitions.rs` | 100% | 100% | schema definitions only |
| `mcp/tools.rs` | 93.5% | 100% | unit tests |
| `mcp/server.rs` | 38.3% | 40.0% | partial |
| `mcp/dispatch.rs` | 0% | 0% | no tests — all tool handlers untested |
| `mcp/tools_acc.rs` | 0% | 0% | no tests |
| `mcp/tools_admin.rs` | 0% | 0% | no tests |
| `mcp/tools_compound.rs` | 0% | 0% | no tests |
| `mcp/tools_dm.rs` | 0% | 0% | no tests |
| `mcp/tools_misc.rs` | 0% | 0% | no tests |
| `mcp/tools_oss.rs` | 0% | 0% | no tests |

---

## Support modules

| Source file | Lines | Fns | Notes |
|---|---|---|---|
| `context_banner.rs` | **67.8%** | 80.0% | unit tests (tier_from_extension, truncate, box rendering) |
| `output/formatter.rs` | 82.0% | 95.2% | insta snapshot tests (JSON/YAML/table/CSV) |
| `shell/command_tree.rs` | 100% | 100% | unit tests |
| `plugins.rs` | 62.9% | 61.3% | partial |
| `credits.rs` | 88.6% | 85.7% | partial |
| `shell/completer.rs` | 50.6% | 44.4% | unit tests |
| `shell/hinter.rs` | 46.3% | 27.3% | unit tests |
| `main.rs` | 46.0% | 74.4% | dispatch smoke tests |
| `output/mod.rs` | 32.4% | 50.0% | partial |
| `shell/highlighter.rs` | 0% | 0% | no tests |
| `shell/prompt.rs` | 0% | 0% | no tests |

---

## Overall Summary

**Total: 25.4% line coverage, 35.9% function coverage**

### By tier

| Tier | Commands | Avg line cov | Reason for gap |
|---|---|---|---|
| Well-covered (>50%) | generate, schema, bucket, pipeline, doctor, admin/mod, hub, inspect | 60–96% | arg-validation + unit tests exercise most paths |
| Partial (20–50%) | auth, api, cache, config, folder, da/engines, issue, project, status, translate/mod, webhook, swarm, context_banner | 20–50% | help + smoke tests; handlers need live API to reach full depth |
| Help-only (<20%) | acc, da/activities/appbundles/workitems, item, plugin, rfi, report sub-files, template, translate/translations | 0–15% | only clap arg-parsing is exercised |
| Zero coverage | job, reality, demo, object/upload, object/copy, mcp/dispatch, all mcp/tools_* | 0% | handlers never invoked in test suite |

### Biggest gaps (zero mock + zero live)

- `commands/job.rs` — job status/list/cancel (Fly.io machine API)
- `commands/reality.rs` — photogrammetry pipeline
- `commands/demo.rs` — demo workflows (1,126 lines, 0% covered)
- `commands/object/upload.rs` — file upload (865 lines, 0%)
- `commands/object/copy.rs` — object copy (305 lines, 0%)
- `mcp/dispatch.rs` + all `mcp/tools_*.rs` — entire MCP tool handler layer (7,126 lines, 0%)
- `commands/rfi/crud.rs` — RFI operations (634 lines, 0%)
- `commands/da/activities.rs` + `appbundles.rs` + `workitems.rs`

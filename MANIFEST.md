# RAPS Manifest — Single Source of Truth

Last updated: 2026-02-25

## Version
- **Current version:** 4.14.0
- **Rust edition:** 2024
- **Minimum Rust version:** 1.88+
- **License:** Apache-2.0

## Counts
- **Top-level commands:** 55
- **Total operations (incl. subcommands):** 95+
- **MCP tools:** 101
- **Workspace crates:** 10
- **APS APIs covered:** 15+
- **Usage modes:** 7 (CLI, Interactive Shell, TUI Dashboard, Python Bindings, GitHub Actions, Docker, MCP Server)
- **TUI Dashboard:** 7 tabs, 33 views
- **Shell scripts (use-cases):** 25 across 7 personas

## Workspace Crates
1. raps-kernel — Core auth, config, HTTP client, logging
2. raps-oss — Object Storage Service (buckets, objects)
3. raps-derivative — Model Derivative API (translations, manifests)
4. raps-dm — Data Management (hubs, projects, folders, items)
5. raps-da — Design Automation (engines, activities, workitems)
6. raps-acc — ACC/BIM 360 (issues, RFIs, assets, submittals, checklists)
7. raps-webhooks — Webhooks API (subscriptions, events)
8. raps-reality — Reality Capture (photoscenes, processing)
9. raps-admin — Account Admin (bulk user ops, folder permissions)
10. raps-cli — CLI binary, MCP server, TUI dashboard

## Command Groups (21 top-level)
auth, bucket, object, translate, hub, project, folder, item, issue, acc, admin, rfi, report, template, webhook, da, reality, pipeline, config, api, generate

## Key Features (v4.12–4.14)
- 101 MCP tools via `raps serve`
- TUI Dashboard (7 tabs, 33 views, opt-in `--features dashboard`)
- Interactive Shell (reedline, TAB completion, syntax highlighting)
- AEC GraphQL integration (faster hub/project queries, REST fallback)
- Server-side object copy and batch copy/rename
- Model Derivative metadata queries (4 commands)
- Bulk admin: user add/remove/update across projects with dry-run
- Headless env detection for auth (auto device flow)
- API health tracking with latency spinners

## Author
Dmytro Yemelianov (dmytroyemelianov@icloud.com)

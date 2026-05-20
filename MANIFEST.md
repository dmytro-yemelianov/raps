# RAPS Manifest — Single Source of Truth

Last updated: 2026-03-19

## Version
- **Current version:** 5.7.0
- **Rust edition:** 2024
- **Minimum Rust version:** 1.88+
- **License:** Apache-2.0

## Counts
- **Top-level command families:** 51
- **Total operations (incl. subcommands):** 195+
- **MCP tools:** 114
- **Workspace crates:** 10
- **APS service crates:** 8
- **APS APIs covered:** 15+
- **Usage modes:** 8 (CLI, Interactive Shell, TUI Dashboard, Python Bindings, GitHub Actions, GitLab CI, Docker, MCP Server)
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

## Command Surface
- Top-level command families are defined in `raps-cli/src/main.rs`.
- Total operations exceed 100 when subcommands are included.
- Use `raps --help` and `raps <command> --help` for the live command surface.

## Key Features (v5.0–v5.7)
- 114 MCP tools via `raps mcp`
- Device code auth flow for headless environments (`raps auth login`)
- Distributed orchestration: Redis-backed queue/cache, serverless dispatch, webhook gateway
- TUI Dashboard (7 tabs, 33 views, opt-in `--features dashboard`)
- Interactive Shell (reedline, TAB completion, syntax highlighting)
- AEC GraphQL integration (faster hub/project queries, REST fallback)
- Server-side object copy and batch copy/rename
- Model Derivative metadata queries (4 commands)
- Bulk admin: user add/remove/update/create across projects with dry-run
- Headless env detection for auth (auto device flow)
- API health tracking with latency spinners
- Company CRUD: create, get, search, update account companies
- Account user lifecycle: create, get, update-account (email-based)
- BIM 360 backward compatibility: HQ v1 API fallback across all admin endpoints
- Export/clone permissions: bulk audit user project & folder permissions to CSV
- Project archive with file export
- Safeguard: rollback and backup script generation for 32 destructive operations
- Graceful 2-legged auth fallback when 3-legged unavailable
- Smart auth expiry handling with silent token refresh

## Author
Dmytro Yemelianov (dmytroyemelianov@icloud.com)

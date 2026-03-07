# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [5.3.3] - 2026-03-07

### Added
- **`raps sync`**: Sync a local directory to an OSS bucket — parallel uploads, `--delete`, `--dry-run`, `--checksum` flags.
- **`raps watch`**: Auto-upload new/changed files from a watched directory to OSS using filesystem events (debounced, `notify` crate).
- **`raps object diff`**: Compare two OSS objects or an OSS object against a local file — colored unified diff for text, SHA-1/SHA-256 comparison for binary, `--stat` and `--checksum-only` flags.
- **`raps object download-bulk`**: Parallel bulk downloads with concurrency control (`--concurrency N`), multi-progress bars, skip-existing, and `--flat` mode.
- **`raps object inspect`**: Inspect `.zip` and `.tar.gz` archives via HTTP Range requests without downloading the full file. `--extract` to pull out a single entry.
- **`raps stats`**: Aggregate usage dashboard combining endpoint stats, command history, and throughput cache.
- **`raps workflow run`**: Compose upload → translate → poll → download in a single command.
- **`raps docs`**: Generate agent-facing documentation (AGENTS.md) from live CLI introspection; MCP subcommand; CI freshness check.
- **`raps pipeline --resume / --reset / --reset-from`**: Idempotent pipeline re-runs — skip already-completed steps, clear state, or restart from a named step.
- **Pipeline parallel execution**: `depends_on` field with topological sort (Kahn's algorithm) and cycle detection; steps without dependencies run concurrently.
- **Translation deduplication cache**: `~/.cache/raps/translation_cache.json` keyed by `urn::format`; `--force` overrides.
- **Secret scanning before upload**: Six credential patterns detected before any upload; `--allow-secrets` to bypass.
- **Transparent 401 token refresh**: Automatic token refresh on expiry with pre-check to avoid unnecessary API calls.
- **TTL response cache**: GET responses cached with per-endpoint TTLs (buckets 60s, hubs 120s, DA engines 3600s).
- **Rate-limit awareness**: Parses `X-RateLimit-*` headers and auto-throttles when quota drops below 10%.
- **Adaptive per-endpoint retry**: Tracks failure rates per endpoint and adjusts backoff multiplier (1×/2×/4×).
- **Fuzzy command suggestions**: Levenshtein distance used to suggest corrections for mistyped subcommands.
- **Command history and replay**: `raps history` and `raps replay` — all commands logged to `~/.cache/raps/history.json`.
- **Throughput-based chunk sizing**: Multipart upload chunk size (5/10/25 MB) auto-selected from measured throughput.
- **Duplicate detection**: `--skip-if-exists` flag on upload uses SHA-1 comparison against existing object.
- **Upload cost estimation**: `--cost-estimate` shows time/bandwidth estimate before upload.
- **Format auto-detection**: Translation output format inferred from input file extension.
- **Translation watch mode**: `--watch` flag on translate polls until success/failure/timeout.
- **`--proxy` flag**: Global proxy URL flag with eager validation; `RAPS_PROXY` env var support.
- **HTTP/2 multiplexing**: `http2_adaptive_window` enabled by default; `RAPS_HTTP2=0` to disable.
- **HTTP/3 QUIC transport**: Optional `h3` feature for swarm inter-agent communication via `--quic` / `RAPS_SWARM_QUIC=1`.
- **Agent-first CLI**: JSON Schema output for all types, NDJSON streaming, prompt injection defense, ID format validation.
- **Plugin system completion**: Pre/post hooks now fire around every command; alias expansion wired into interactive shell.
- **AGENTS.md**: Auto-generated agent documentation committed to repo root.

### Fixed
- HTTP client configuration errors now surface immediately instead of silently falling back to defaults.
- Pipeline idempotent state correctly persists across `--resume` runs.
- Plugin `run_pre_hooks` / `run_post_hooks` were defined but never called — now wired into every command dispatch.
- Removed dead items from `mcp/auth_guidance.rs`; simplified `AuthState` struct.

### Changed
- `new_with_http_config` across all API client crates now returns `Result<Self>` instead of silently panicking.
- Removed five crate-level `#![allow(dead_code)]` suppressions (raps-reality, raps-webhooks, raps-da, raps-acc, raps-dm) — all items were live public API.
- Shared progress bar helper extracted to `raps_kernel::progress`; duplicated setup removed from reality, admin CSV, and report commands.

### Dependencies
- `reedline` 0.45 → 0.46
- `toml` 0.8 → 1.0
- `crossterm` 0.28 → 0.29
- `thiserror` 1.0 → 2.0
- `rmcp` 0.12 → 0.17
- `deadpool-redis` 0.18 → 0.23
- `ratatui` 0.29 → 0.30
- `webbrowser` 0.8 → 1.1
- `lru` 0.12 → 0.16
- `redis` 0.27 → 1.0

## [5.3.2] - 2026-03-06

### Added
- **Admin role upsert**: Update role if user is already a member instead of failing.
- **BIM 360 Business hub support**: Admin user-add now works across ACC and BIM 360 hub types.
- **Intelligent role resolution**: `--role` flag resolves role names for both ACC and BIM 360 APIs.

### Fixed
- `raps admin add`: HTTP 409 already-member treated as skipped, not failed.
- `raps admin add`: Removed broken user-exists pre-check that caused false negatives.
- ACC API alignment: `roleId` → `roleIds` array, `projectAdministration` rules, init onboarding flow.
- HTTP 4xx/5xx errors now include API response body in error messages.
- `raps admin add`: Insight role-lock HTTP 400 treated as skipped.
- Storage: plaintext keyring file warnings include `migrate-tokens` hint.
- ANSI logo spacing and sunflower alignment tweaks.

## [5.3.0] - 2026-03-05

### Added
- **Comprehensive test coverage**: 100+ new scenario tests across rfi, admin, object copy, DA, config, job, webhook, hub, translate, and status commands using raps-mock. Coverage report published as workflow artifact.

### Changed
- raps-mock dependency switched from path to git reference (v0.3.0).

## [5.1.0] - 2026-03-02

### Added
- **`raps doctor` expanded**: 8 new self-checks — network connectivity, filesystem permissions, context variables, disk space (warn <500 MB, fail <100 MB), keyring probe, env var conflict detection, version staleness (GitHub releases API), proxy/TLS environment with credential masking.
- **Duplicate detection**: SHA-1 comparison for `--skip-if-exists` on object upload.
- **Adaptive multipart chunk sizing**: 5/10/25 MB chunks based on measured throughput; ETA display with 10s rolling window.
- **Pipeline `depends_on`**: Topological sort with cycle detection for step ordering.
- **Translation `--watch`**: Polls until success, failure, or timeout.
- **Fuzzy command corrections**: Levenshtein edit distance suggests the nearest valid subcommand.
- **Rate-limit throttling**: Auto-sleep when `X-RateLimit-Remaining` drops below 10% of limit.
- **Endpoint health tracking**: Per-endpoint failure rate with exponential backoff multiplier.
- **Format auto-detect**: Translation output format inferred from input file extension.
- **Command history**: All commands logged; `raps history` and `raps replay`.
- **Auto-profile from `.raps-project`**: Active profile set from nearest `.raps-project` file.
- **Upload cost estimation**: Time/bandwidth preview before large uploads.
- **Webhook status**: `raps webhook status` with optional reachability check.

## [5.0.0] - 2026-03-01

### Added
- **Swarm Orchestration Kernel**: Circuit breaker, retry policy, rate budget, region routing, response cache, and HTTP middleware wiring for multi-agent coordination.
- **Metrics & Audit**: API metrics collector with per-endpoint latency/error tracking and structured audit logger with JSON output.
- **Checkpoint Store**: Durable progress checkpointing for long-running swarm operations with automatic resume.
- **TUI Swarm Dashboard (F8)**: Real-time swarm status tab showing circuit breaker states, rate budgets, cache stats, API metrics, and active checkpoints.
- **Compound MCP Tools**: Bulk MCP operations (`bulk_upload`, `bulk_download`, `search_and_download`, `upload_and_translate`, `translate_and_download`) with progress reporting.
- **Swarm CLI**: `raps swarm status|reset|run` commands for swarm orchestration control.

### Performance
- **HTTP/2 Multiplexing**: Enabled `http2_adaptive_window`, connection pool tuning (`pool_idle_timeout=90s`, `pool_max_idle_per_host=10`), TCP keepalive (30s).

### Security
- ASVS L2 compliance at 100% (34/34 requirements met).

## [4.18.0] - 2026-03-01

### Added
- **Content-Addressed Download Cache**: SHA-1 keyed cache with hardlink materialization. New CLI flags: `--no-cache`, `--cache-dir`, `--refresh`, `--offline`. `raps cache stats|clear|dir` subcommands.
- **HTTP Range Inspect**: `raps inspect zip` lists zip archive contents via Range requests — only downloads the central directory, not the entire file.
- **Strict Mode for CI**: `--strict` flag (or `RAPS_STRICT=1`) implies `--non-interactive` and rejects silent defaults, making every ambiguous parameter require explicit values.
- **Unified Retry/Concurrency Flags**: `--no-retry` flag and `RAPS_CONCURRENCY` env var support for consistent control across all commands.
- **Plugin Info Command**: `raps plugin info` for detailed plugin inspection.
- **Non-Interactive Env Vars**: `RAPS_NON_INTERACTIVE=1` and `RAPS_YES=1` environment variable support.
- **Upload Management**: `raps upload status|abort|cleanup` subcommands.
- **Checklist Delete**: Complete CRUD coverage for checklists.
- **JSON Schema Output**: `--output-format=json-schema` for all CLI output types.
- **Unified Concurrency Flag**: `--concurrency` standardized across all bulk commands.
- **List Limits**: `--limit` flag on `bucket list` and `object list` commands.
- **Configurable Retry/Backoff**: `--max-retries`, `--retry-delay`, `--max-retry-delay` CLI flags and env vars.

### Changed
- **ASVS L2 Security Hardening**: Path traversal protection on all download paths, automatic log redaction via `RedactingMakeWriter`, restricted directory permissions (0o700) on log/config dirs, pipeline variable injection hardening with `shlex` parsing and metacharacter validation.

### Performance
- Replace blocking `std::fs` calls with async `tokio::fs` in async functions.
- Reuse buffers across multipart upload chunks via pool.
- Batch resumable upload state writes (every 5 parts instead of every part).

### Fixed
- Remove unused `Value` import in MCP server.

### Security
- Path traversal: `sanitize_filename()`, `validate_path_within()`, `safe_join()` applied to all download code paths.
- Log redaction: Bearer, Basic auth, Cookie, API key headers, and URL token parameters automatically redacted in all log output.
- Auth error messages redacted before inclusion in `bail!()` messages.
- Log and config directories created with mode 0o700.
- Pipeline variable substitution hardened against shell injection.
- ASVS L2 compliance matrix updated from 74% to ~82%.

## [4.17.0] - 2026-03-01

### Added
- **Pipeline v2 Engine**: Complete rewrite of pipeline execution with retry, timeout, conditionals, parallel steps, and for-each loops.
  - Expression evaluator for conditional step execution.
  - Duration parser for human-readable timeouts (`30s`, `5m`).
  - Parallel step execution with configurable concurrency.
  - `for-each` loop steps for iterating over collections.
  - 4 new MCP tools: `pipeline-validate`, `pipeline-dry-run`, `pipeline-run`, `pipeline-list-templates`.
- **Pipeline v2 MCP Integration**: Validate, dry-run, and execute pipelines via MCP.

### Changed
- **keyring 2.3 → 3.6**: Simplified dependency tree, `delete_password()` → `delete_credential()`.
- **reqwest 0.11 → 0.12**: Upgraded to hyper v1 / http v1 with zero API changes.

### Fixed
- Wrap all 18 blocking `dialoguer` prompts with `tokio::task::spawn_blocking` to prevent async runtime stalls.
- Guard 3-legged OAuth device flow against non-interactive mode with early bail.
- Fix npm publish workflow overwriting all optional deps with win32-x64.

### Security
- Add top-level `permissions:` blocks to publish and sbom workflows.
- Remove 3 RustSec advisory exceptions (RUSTSEC-2024-0388, RUSTSEC-2024-0384, RUSTSEC-2025-0134) — resolved by keyring 3.6 and reqwest 0.12.

## [4.16.0] - 2026-02-28

### Added
- **Pipeline v2 Data Model**: Restructured pipeline configuration with retry policies, timeout support, and conditional execution.

### Fixed
- CI: SBOM generation workflow fixes (filename, manifest-path, cargo-cyclonedx flags).
- CI: Publish workflow — handle already-published crates, rate limit retries.
- CI: Add continue-on-error to release announce dispatch steps.
- Add version to internal workspace deps for crates.io publishing.

## [4.15.0] - 2026-02-27

### Added
- **Plugin Signing**: Cryptographic plugin signature verification.
- **ASVS L2 Compliance**: Full compliance documentation and matrix.
- Security badges on README and website security page.

### Security
- Pin Semgrep container image and fuzz RAPS-specific code.
- Comprehensive security hardening: ASVS L2, SLSA L2, CI scanning, OpenSSF Scorecard.

## [4.14.1] - 2026-02-26

### Security
- Fix TOCTOU race condition in path validation.
- Path traversal fix for download code paths using `sanitize_filename` and `safe_join`.
- Automatic log redaction for Bearer tokens, cookies, and API keys.
- Restricted directory permissions (0o700) for log and config directories.

## [4.14.0] - 2026-02-25

### Added
- **Auto-Detect Headless Environments**: `auth login` detects missing display server and suggests alternatives.
- **Live Progress Spinners**: API health and latency tracking on all network operations.
- **Unix Pipe Support**: stdin/stdout piping across CLI commands.

### Changed
- Split 14 monolithic files into focused modules (MCP server.rs from 6312 lines → 8 modules).

### Fixed
- Validate empty auth code input and reset refreshing flag on error.
- Validate client credentials before auth operations.
- Replace broken device code flow with manual PKCE auth.
- Comprehensive MCP server review fixes (20 issues).

## [4.13.0] - 2026-02-24

### Added
- **Model Derivative Metadata Endpoints**: 4 new CLI commands for translation output inspection.
  - `raps translate metadata <URN>`: List viewable GUIDs from a completed translation.
  - `raps translate tree <URN> <GUID>`: Show the object tree hierarchy.
  - `raps translate properties <URN> <GUID>`: Retrieve all object properties.
  - `raps translate query-properties <URN> <GUID> --filter <IDs>`: Filter properties by object IDs.
- **OSS Server-Side Copy**: Copy objects between buckets without re-uploading.
  - `raps object copy <SRC> <DEST>`: Single object copy via `x-ads-copy-from` header.
  - `raps object batch-copy`: Batch copy with Semaphore(10) concurrency.
  - `raps object batch-rename`: Batch rename objects within a bucket.
- **DA Appbundle Upload**: Upload appbundles to pre-signed S3 URLs.
  - `raps da appbundle-upload <ID> --file <ZIP>`: Multipart POST to pre-signed URL from bundle creation response.
- **Demoscene-Style Credits**: `raps --version` now displays a branded ASCII art credits block with rapeseed flower logo, version info, and feature stats.

### Fixed
- REST pagination for `list_projects()`, `list_folder_contents()`, `get_item_versions()` in raps-dm (100-page safety cap).
- Model Derivative region support: `MdRegion` enum (8 regions) with `--region`/`--force` CLI flags.
- Token refresh race condition: replaced `RwLock` with `Mutex`-based `TokenCache` with coordinated refresh.
- MIME type detection in raps-reality: extension-based detection supporting 8 formats instead of hardcoded `image/jpeg`.
- Contradictory `normalize_project_id()` functions consolidated into shared `strip_project_prefix()`/`ensure_project_prefix()`.
- 2-hour polling timeout for `translate`, 4-hour for reality capture commands.
- Webhook event validation with `is_valid_event()`.
- BIM360 folder auto-detection via `b.` prefix.

## [4.12.0] - 2026-02-23

### Added
- **AEC GraphQL Integration**: `raps dm hubs` and `raps dm projects` now use the AEC Data Model GraphQL API for ACC/BIM 360 projects.
- **TUI Dashboard Expansion**: Expanded from 4 to 7 tabs with 33 views across Storage, Translation, Projects, Webhooks, Design Automation, ACC, and Admin.

### Changed
- Dashboard is now an optional compile-time feature (`--features dashboard`).

## [4.11.0] - 2026-02-22

### Fixed
- Replace 7 marketplace `.unwrap()` panics with contextual error messages.
- Implement actual confirmation prompt in `should_proceed_destructive()`.
- Add folder delete confirmation guard.
- Move shell history to config directory with graceful fallback.
- Add 30-minute timeout to DA work item polling loop.
- Replace `process::exit` with `bail!` in plugin exit and clap error paths.
- Standardize `ProjectDirs` qualifiers to `("com","autodesk","raps")`.
- Replace semaphore `.unwrap()` with cancellation-safe error handling.
- Show clear error on malformed shell input (unmatched quotes).
- Warn on invalid custom API headers instead of silent drop.
- Replace dashboard `event::read().unwrap()` with `.ok()`.
- Safe UTF-8 slicing in shell completer.
- Cap `warn_expiry_seconds` to prevent `i64` overflow.
- Replace `process::exit` with `bail!` for proper async cleanup in api.rs and admin.rs.
- Fix overflow in retry backoff with `saturating_mul`/`checked_shl`.
- Handle `Arc::try_unwrap` failure with fallback clone in raps-oss.
- Make `token_file_path`/`batch_state_path` return `Result` for CI/Docker safety.
- Wrap blocking `tiny_http::recv` in `spawn_blocking`.
- Preserve original scopes on token refresh.
- Add scope parameter to device code authorization.
- Fix UTF-8 safety in `mask_string` with char-based indexing.
- Add `MAX_PAGES` guard to `list_objects` pagination in raps-oss.

## [4.10.0] - 2026-02-22

### Fixed
- Error handling hardening and auth safety improvements across codebase (18 findings).
- Remove duplicate HTTP log lines from raps-oss.

## [4.9.0] - 2026-02-22

### Fixed
- **Profiler Accuracy**: Fix HTTP double-counting on retries; track auth and S3 upload calls; switch to `AtomicU64` for lock-free recording.
- **Structured Logging**: Configurable file filter (`RAPS_FILE_LOG`), optional JSON format (`RAPS_FILE_FORMAT=json`), 50MB size cap.
- Replace `eprintln` with `tracing::warn` across raps-oss and kernel storage.
- Add tracing dependency to raps-acc, raps-da, raps-webhooks, raps-reality, raps-admin.

## [4.8.0] - 2026-02-22

### Added
- **Reedline Shell**: Replace rustyline with reedline (nushell's line editor) fixing cursor-out-of-sync on Windows.
  - Yellow styled prompt with proper width tracking.
  - Tab completion via columnar menu.
  - Command syntax highlighting (known commands in green).
  - History hints with fish-style inline suggestions.
- **Tracing-Based Logging**: Migrate from manual `eprintln` to `tracing-subscriber` with stderr console output and daily rolling file logs.
- **Performance Profiler**: Track execution time, HTTP requests, memory usage with `--profile` flag.
- **Interactive Mode**: Optional ID arguments for folder, project, and RFI commands with fuzzy-select prompts when omitted.
- **Resilient Keychain**: Graceful fallback to file storage on all keychain error paths.

### Changed
- Standardized exit codes across admin, api, and auth commands.

## [4.7.0] - 2026-02-19

### Added
- **Auth Login Presets**: 5 new scope presets for `raps auth login -p <preset>`: `viewer`, `editor`, `storage`, `automation`, `admin`.
- **Preset Short Flag**: `-p` as short alias for `--preset`.

## [4.6.0] - 2026-02-19

### Added
- **Auth Preset Scopes**: `raps auth login --preset all` for non-interactive scope selection.
- **DA Auto-Qualification**: Bare app bundle and activity names are automatically qualified with your DA nickname.
- **DA Auto-Alias**: A "default" alias is automatically created after `appbundle-create` and `activity-create`.

### Fixed
- DA `appbundle-create` deserialization error (made `endpointUrl` and `formData` Optional).
- DA `activity-create` "Cannot parse id" error for bare names.
- Pipeline sample template using incorrect `bucket create` syntax (added `-k` flag).
- DA `workitems` `startAfterTime` format (millisecond precision).

## [4.5.0] - 2026-02-10

### Added
- **HTTP Retry**: Automatic retry on 429/5xx with exponential backoff across all service crates.
- **New Docs Pages**: api, report, and template command documentation on rapscli.xyz.
- **Company List**: `raps admin company-list` command.

### Changed
- Reduced bucket list per-region timeout from 30s to 10s for faster responses.
- Simplified report.rs and admin.rs modules.

### Fixed
- Clap `-o`/`--output` flag conflict causing panics in `api` and `translate` commands.
- Status counting bug in admin operations.

## [4.4.0] - 2026-01-20

### Added
- **14 New MCP Tools**: Bringing total to 51 tools for AI assistant integration.
- **Bulk Folder Permissions**: `admin_folder_rights` MCP tool.
- **Operation Lifecycle**: `admin_operation_resume`, `admin_operation_cancel` MCP tools.
- **Full RFI CRUD**: `rfi_create`, `rfi_update` MCP tools.
- **Full Assets CRUD**: `asset_create`, `asset_update`, `asset_delete` MCP tools.
- **Full Submittals CRUD**: `submittal_create`, `submittal_update` MCP tools.
- **Full Checklists CRUD**: `checklist_create`, `checklist_update` MCP tools.

### Changed
- Enhanced `admin_project_list` with advanced filter expressions.
- Improved auth guidance for new MCP tool categories.

## [4.3.0] - 2026-01-18

### Added
- **npm Distribution**: Install via `npm install -g @dmytro-yemelianov/raps-cli`.
  - Platform-specific packages for Windows, macOS, and Linux (x64 and arm64).
  - Automatic platform detection and binary selection.
  - Support for `npx @dmytro-yemelianov/raps-cli` without global install.
- **Python Bindings (PyO3)**: Native Python library `raps-bindings` for programmatic access.
  - `Client` class with 2-legged OAuth authentication.
  - `BucketsManager` for OSS bucket operations (list, create, get, delete).
  - `ObjectsManager` for object operations (upload, download, list, delete, signed URLs).
  - `TranslationJob` for Model Derivative translation with polling support.
  - `HubsManager` for Data Management hub listing (requires CLI 3-legged auth).
  - Custom exceptions: `RapsError`, `AuthenticationError`, `NotFoundError`, `RateLimitError`, `ValidationError`.
  - Type stubs (.pyi) for IDE autocompletion.

## [4.2.3] - 2026-01-18

### Changed
- **PyPI Wheels**: Excluded Linux ARM64 wheel due to `ring` cross-compile issues.
  - Linux ARM64 users can use `install.sh` or cargo-dist binaries.
  - Available wheels: Linux x64, macOS x64/ARM64, Windows x64.

## [4.2.2] - 2026-01-18

### Fixed
- **PyPI Wheel Build**: Fixed maturin working directory in release workflow.
  - Run maturin from `python/` directory instead of using `--manifest-path`.
  - Corrected artifact upload path.

## [4.2.1] - 2026-01-18

### Fixed
- **Install Scripts**: Fixed artifact naming to match cargo-dist convention.
  - Updated `install.sh` and `install.ps1` to use cargo-dist target triples (e.g., `x86_64-unknown-linux-gnu`).
  - Changed archive format from `.tar.gz` to `.tar.xz` for Linux/macOS.
  - Updated checksum verification to use individual `.sha256` files.

## [4.2.0] - 2026-01-18

### Added
- **Multi-Channel Distribution**: New installation methods for easier onboarding.
  - **Shell Install Script** (`install.sh`): One-liner install for Linux/macOS with automatic PATH configuration.
    - Supports bash, zsh, and fish shells
    - SHA256 checksum verification
    - Version selection via `RAPS_VERSION` environment variable
    - Uninstall support via `--uninstall` flag
  - **PowerShell Install Script** (`install.ps1`): One-liner install for Windows.
    - Automatic User PATH modification
    - Checksum verification using Get-FileHash
    - Parameters: `-Version`, `-InstallDir`, `-NoPathUpdate`, `-Uninstall`
  - **PyPI Distribution**: Install via `pip install raps`.
    - Platform wheels for Linux, macOS, and Windows (x64 and arm64)
    - Uses maturin with `bindings = "bin"` for binary bundling
    - Python 3.8+ support
- **Enhanced Release Automation**: GitHub Actions workflow extended for multi-channel publishing.
  - Automatic PyPI publishing using OIDC trusted publishing
  - Post-release install script testing on all platforms

## [4.1.0] - 2026-01-17

### Added
- **MCP Server Expansion**: Aligned MCP server with CLI v4.0 functionality (14 → 35 tools).
  - Admin Bulk Operations: `admin_project_list`, `admin_user_add`, `admin_user_remove`, `admin_user_update_role`, `admin_operation_list`, `admin_operation_status`.
  - Folder/Item Management: `folder_list`, `folder_create`, `item_info`, `item_versions`.
  - Issues: `issue_list`, `issue_get`, `issue_create`, `issue_update`.
  - RFIs: `rfi_list`, `rfi_get`.
  - ACC Extended: `acc_assets_list`, `acc_submittals_list`, `acc_checklists_list`.

### Changed
- MCP server instructions updated to reflect v4.0 capabilities.
- ACC/Admin clients created on-demand in MCP server (not cached) due to Clone trait requirements.

## [4.0.0] - 2026-01-16

### Added
- **Account Admin Bulk Management Tool**: New `raps admin` command suite for bulk user management across ACC/BIM 360 accounts.
  - `raps admin user add`: Bulk add users to multiple projects with role assignment.
  - `raps admin user remove`: Bulk remove users from projects.
  - `raps admin user update-role`: Bulk update user roles across projects.
  - `raps admin folder rights`: Bulk update folder permissions (Project Files, Plans, or custom folders).
  - `raps admin project list`: List projects with filtering by name, status, and platform.
  - `raps admin operation status`: View operation progress and results.
  - `raps admin operation resume`: Resume interrupted operations.
  - `raps admin operation cancel`: Cancel in-progress operations.
  - `raps admin operation list`: List all operations with status filtering.
- **New `raps-admin` Crate**: Orchestration layer for bulk operations with:
  - Resumable state persistence using JSON files.
  - Semaphore-based concurrency control (max 50 parallel requests).
  - Exponential backoff retry logic for rate limit handling (429 errors).
  - Progress tracking with indicatif progress bars.
  - Dry-run mode for operation preview.
  - Project filtering by regex pattern.
- **New `FolderPermissionsClient`**: ACC Folder Permissions API client in `raps-acc` crate.
- **Comprehensive Integration Tests**: 61 tests covering all bulk operations and state management.

### Changed
- Major version bump due to significant new feature addition.

## [3.11.0] - 2026-01-15

### Added
- **Global Output Format Standardization**: Consistent JSON, YAML, Table, CSV output across all commands.
  - New global `--output` flag supports `json`, `yaml`, `table`, `csv`, `plain`.
  - Automatic JSON fallback in non-interactive (piped) environments.
  - Added `serde_yaml` support for YAML output.
- **Standardized Exit Codes**: robust exit codes for CI/CD scripting:
  - `0`: Success
  - `2`: Invalid arguments
  - `3`: Authentication failure
  - `4`: Resource not found
  - `5`: Remote/API error
  - `6`: Internal error
- **Global Logging Flags**: Control verbosity and color with:
  - `--no-color`: Disable ANSI colors
  - `--quiet`: Suppress info logs
  - `--verbose`: Show request summaries
  - `--debug`: Show detailed traces with **secret redaction**.
- **Non-interactive Mode**:
  - Global `--non-interactive` flag ensures no prompts are shown.
  - Fail-fast behavior for missing required arguments in non-interactive mode.
  - Global `--yes` flag for auto-confirming destructive actions.

### Changed
- All CLI commands now use a centralized output formatter for consistency.
- Logging infrastructure now automatically redacts secrets (tokens, keys) from debug output.
- `raps-kernel` error handling updated to map `anyhow::Error` chain to standardized exit codes.

## [3.4.0] - 2026-01-02
...
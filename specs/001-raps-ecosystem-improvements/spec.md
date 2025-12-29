# Feature Specification: RAPS Ecosystem Improvements

**Feature Branch**: `001-raps-ecosystem-improvements`  
**Created**: 2025-12-29  
**Updated**: 2025-12-30  
**Status**: In Progress (~45% Complete)  
**Input**: User description: "Build and improve the solution that is implementing command-line interface for the Autodesk Platform Services (APS). CLI, MCP, GitHub Actions, Docker container, etc. modes. Refactor to microkernel architecture with Core → Community → Pro tiers."

## Executive Summary

This specification covers comprehensive improvements to the RAPS (Rust Autodesk Platform Services) ecosystem—a multi-repository project providing CLI, MCP server, GitHub Actions, Docker container, and TUI interfaces for Autodesk Platform Services automation.

### Vision: Microkernel Architecture with Tiered Products

RAPS will evolve into a **Unix-like microkernel architecture** with three product tiers:

| Tier | Description | License |
|------|-------------|---------|
| **Core (Kernel)** | Minimal trusted foundation: Auth, SSA, OSS, Derivative, Data Management | Apache 2.0 |
| **Community** | Extended features: Account Admin, ACC, DA, Reality, Webhooks, Pipelines, Plugins, MCP, TUI | Apache 2.0 |
| **Pro (Enterprise)** | Advanced features: Analytics, Audit, Compliance, Multi-tenant, SSO | Commercial |

### Repository Taxonomy

| Category | Repositories |
|----------|--------------|
| **Primary App** | `raps` (Rust CLI + microkernel crates) |
| **Documentation** | `raps-website` (main website at rapscli.xyz) |
| **Distribution Satellites** | `homebrew-tap`, `scoop-bucket`, `raps-action`, `raps-docker` |
| **Ecosystem** | `aps-tui`, `aps-wasm-demo`, `aps-sdk-openapi` |

**Current State (v3.2.0)**:
- ✅ Mature CLI with 50+ commands covering Authentication, OSS, Model Derivative, Data Management, Webhooks, Design Automation, ACC Issues/RFIs/Assets/Submittals/Checklists, Reality Capture, Pipelines, Plugins
- ✅ MCP server with 14 tools for AI assistant integration
- ✅ GitHub Action for CI/CD workflows
- ✅ Docker container for containerized deployments
- ✅ **Microkernel architecture implemented** - 6 service crates extracted
- ✅ **Tiered product strategy** - Core/Community/Pro feature flags working
- ✅ **Build performance infrastructure** - sccache, nextest, mold/lld-link in CI
- ⚠️ TUI (Terminal UI) in early development
- ⚠️ Some kernel modules need refinement (middleware.rs, auth flow separation)

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - High-Performance File Operations (Priority: P1)

As a DevOps engineer automating CAD file workflows, I need to upload large files (100MB+) quickly so that my CI/CD pipelines complete within acceptable time limits.

**Why this priority**: File operations are the most common use case and directly impact user productivity. Performance issues here affect every workflow.

**Independent Test**: Upload a 100MB test file with parallel chunks and measure throughput improvement vs baseline.

**Acceptance Scenarios**:

1. **Given** a 100MB file to upload, **When** I run `raps object upload --parallel bucket file.dwg`, **Then** upload completes 3-5x faster than sequential upload using parallel chunk uploads.
2. **Given** a multipart upload in progress, **When** the network fails mid-transfer, **Then** I can resume with `--resume` and only incomplete chunks are re-uploaded without restarting.
3. **Given** the `--concurrency 10` flag, **When** performing batch operations, **Then** exactly 10 operations run in parallel with proper semaphore control.

---

### User Story 2 - Consistent Cross-Interface Experience (Priority: P1)

As a developer using RAPS across CLI, MCP, and GitHub Actions, I need identical behavior and output formats so that my automation scripts work regardless of interface.

**Why this priority**: Inconsistency between interfaces creates confusion and doubles maintenance burden for automation scripts.

**Independent Test**: Execute the same logical operation (e.g., list buckets) via CLI, MCP tool, and GitHub Action; verify identical JSON output schema.

**Acceptance Scenarios**:

1. **Given** `raps bucket list --output json` via CLI, **When** I call `bucket_list` via MCP, **Then** the JSON output schema is identical.
2. **Given** error code 3 (auth failure) from CLI, **When** the same error occurs in GitHub Action, **Then** `exit-code` output is 3 with matching error message format.
3. **Given** a destructive operation in MCP, **When** `--yes` equivalent is not provided, **Then** operation fails with confirmation requirement (matching CLI behavior).

---

### User Story 3 - Robust CI/CD Integration (Priority: P2)

As a CI/CD pipeline author, I need strict non-interactive mode and clear exit codes so that my workflows never hang waiting for input and failures are detectable.

**Why this priority**: CI/CD reliability is critical for enterprise adoption; hanging pipelines waste compute resources.

**Independent Test**: Run all commands with `--non-interactive` flag in GitHub Actions; verify none prompt for input.

**Acceptance Scenarios**:

1. **Given** `--non-interactive` mode, **When** a command requires missing input, **Then** it fails immediately with exit code 2 and clear error message.
2. **Given** a bucket creation in non-interactive mode, **When** `--key` is not provided, **Then** error says "Required argument --key missing in non-interactive mode".
3. **Given** rate limiting from APS API, **When** retry exhausts, **Then** exit code 5 with JSON output containing retry count and last error.

---

### User Story 4 - MCP Server Reliability (Priority: P2)

As an AI assistant (Claude/Cursor) user, I need the MCP server to handle rapid sequential requests without resource exhaustion so that long AI-assisted sessions remain responsive.

**Why this priority**: MCP is a differentiating feature; poor performance here undermines the AI integration value proposition.

**Independent Test**: Send 100 sequential MCP tool calls in 60 seconds; verify no memory growth or connection failures.

**Acceptance Scenarios**:

1. **Given** MCP server running for 1 hour with continuous requests, **When** I check memory usage, **Then** it remains under 100MB with no growth trend.
2. **Given** an MCP tool call with invalid parameters, **When** processed, **Then** error response follows JSON-RPC spec with actionable message.
3. **Given** concurrent MCP requests, **When** rate limits are hit, **Then** server queues requests with exponential backoff.

---

### User Story 5 - Docker Container for Air-Gapped Environments (Priority: P2)

As a security-conscious organization, I need a self-contained Docker image that works in restricted networks with all dependencies bundled.

**Why this priority**: Enterprise adoption often requires containerized deployments with security scanning.

**Independent Test**: Run Docker container in network-isolated environment; verify `--help` works without external downloads.

**Acceptance Scenarios**:

1. **Given** the Docker image, **When** I run `docker run dmytroyemelianov/raps bucket list`, **Then** it executes without downloading additional dependencies.
2. **Given** environment variables for credentials, **When** container starts, **Then** credentials are NOT written to any files inside the container.
3. **Given** multi-arch build (amd64/arm64), **When** running on Apple Silicon or AWS Graviton, **Then** native binary is used without emulation.

---

### User Story 6 - GitHub Action for Workflow Automation (Priority: P3)

As a GitHub workflow author, I need a reliable action that installs RAPS quickly and passes outputs correctly for downstream steps.

**Why this priority**: GitHub Actions integration enables the largest CI/CD platform ecosystem.

**Independent Test**: Create workflow that installs RAPS, runs command, and uses output in subsequent step.

**Acceptance Scenarios**:

1. **Given** `version: latest` input, **When** action runs, **Then** it installs the most recent release within 30 seconds.
2. **Given** a failed command, **When** workflow continues, **Then** `exit-code` output is available for conditional logic.
3. **Given** Windows runner, **When** action runs, **Then** Windows binary is downloaded and executed correctly.

---

### User Story 7 - TUI for Interactive Exploration (Priority: P3)

As a developer exploring APS resources, I need a visual terminal interface to browse hubs, projects, and folders without memorizing command syntax.

**Why this priority**: TUI lowers the barrier to entry for new users discovering APS capabilities.

**Independent Test**: Launch TUI, navigate to a bucket, upload a file, and view translation status—all with keyboard.

**Acceptance Scenarios**:

1. **Given** TUI launched with valid credentials, **When** I press Tab, **Then** I can cycle through panels (Buckets, Objects, Translations).
2. **Given** a long list of objects, **When** I type characters, **Then** list filters incrementally (fuzzy search).
3. **Given** an upload operation in TUI, **When** in progress, **Then** visual progress bar shows completion percentage.

---

### User Story 8 - Microkernel Architecture (Priority: P0) 🏗️ ARCHITECTURE

As a maintainer, I need the codebase refactored into a microkernel architecture so that the core is minimal, auditable, and highly testable while features can evolve independently.

**Why this priority**: Architecture foundation must be established before new features; enables security auditing and isolated testing.

**Independent Test**: Build with `--features core` produces minimal binary; kernel crate has 100% test coverage.

**Acceptance Scenarios**:

1. **Given** `cargo build --features core`, **When** compiled, **Then** binary includes only Auth, OSS, Derivative, and Data Management (no ACC, DA, etc.).
2. **Given** `raps-kernel` crate, **When** I run `cargo test -p raps-kernel`, **Then** all tests pass with >90% coverage.
3. **Given** a bug in `raps-community` ACC module, **When** it panics, **Then** kernel functionality remains unaffected.
4. **Given** `raps-kernel` source, **When** audited, **Then** LOC < 3000 with zero `unsafe` blocks.

---

### User Story 9 - Tiered Product Builds (Priority: P1)

As a product manager, I need to build different product tiers (Core, Community, Pro) from the same codebase so that we can offer free and commercial versions.

**Why this priority**: Enables sustainable business model while keeping core functionality open source.

**Independent Test**: Build all three tiers; verify feature availability matches specification.

**Acceptance Scenarios**:

1. **Given** `cargo build --features core`, **When** I run `raps acc issue list`, **Then** command fails with "Feature requires Community tier".
2. **Given** `cargo build --features community`, **When** I run `raps analytics dashboard`, **Then** command fails with "Feature requires Pro tier".
3. **Given** `cargo build --features pro`, **When** I run any command, **Then** all features are available.
4. **Given** Community build, **When** I check `raps --version`, **Then** output shows "RAPS Community v3.2.0".

---

### User Story 10 - Secure Service Account (SSA) Authentication (Priority: P1)

As a DevOps engineer running automated pipelines, I need to authenticate using Secure Service Accounts (SSA) so that I can obtain 3-legged tokens for service-to-service communication without user interaction.

**Why this priority**: SSA is the recommended authentication method for CI/CD and service automation, providing 3LO capabilities without user login flows. This is critical for enterprise automation.

**Independent Test**: Create SSA robot account, generate key, exchange JWT for access token, make authenticated API call.

**Acceptance Scenarios**:

1. **Given** valid APS credentials and 2-legged token with `application:service_account:write` scope, **When** I run `raps ssa create --name "pipeline-bot"`, **Then** a service account is created and email/ID are returned.
2. **Given** an existing SSA account, **When** I run `raps ssa key create --account-id <id>`, **Then** a private key PEM is returned (only shown once).
3. **Given** SSA credentials (client_id, secret, service_account_id, key_id, private_key), **When** I run `raps auth ssa-token --scope "data:read data:write"`, **Then** a 3-legged access token is returned.
4. **Given** SSA environment variables configured, **When** I run any authenticated command, **Then** SSA JWT assertion flow is used automatically.
5. **Given** SSA key rotation needed, **When** I run `raps ssa key rotate --account-id <id>`, **Then** old key is disabled and new key is generated.

---

### User Story 11 - Account Administration (Priority: P2)

As an ACC/BIM 360 administrator, I need to manage accounts, projects, users, and companies programmatically so that I can automate onboarding and access management.

**Why this priority**: Account administration is fundamental for enterprise ACC deployments and user lifecycle management.

**Independent Test**: List projects in an account, create a new project, assign users, verify access.

**Acceptance Scenarios**:

1. **Given** admin credentials for an ACC account, **When** I run `raps admin projects list --account-id <id>`, **Then** all projects with their status are listed.
2. **Given** project creation permissions, **When** I run `raps admin projects create --name "New Project"`, **Then** project is created and ID is returned.
3. **Given** project admin access, **When** I run `raps admin users assign --project-id <id> --email user@company.com`, **Then** user is assigned to the project.
4. **Given** company directory access, **When** I run `raps admin companies list --account-id <id>`, **Then** all partner companies are listed.

---

### User Story 12 - Data Management Navigation (Priority: P2)

As a developer integrating with ACC/BIM 360, I need to navigate hubs, projects, folders, and items so that I can locate and manage files in the ACC/BIM 360 hierarchy.

**Why this priority**: Data Management API is the foundation for file access in ACC/BIM 360; required for any file-based automation.

**Independent Test**: List hubs, navigate to project, browse folders, get item versions.

**Acceptance Scenarios**:

1. **Given** authenticated user with hub access, **When** I run `raps dm hubs list`, **Then** all accessible hubs (BIM 360/ACC accounts) are listed.
2. **Given** a hub ID, **When** I run `raps dm projects list --hub-id <id>`, **Then** all projects in that hub are listed.
3. **Given** a project ID, **When** I run `raps dm folders list --project-id <id> --folder-id <top-folder>`, **Then** folder contents (items and subfolders) are listed.
4. **Given** an item ID, **When** I run `raps dm versions list --project-id <id> --item-id <id>`, **Then** all versions of that item are listed with tip version marked.
5. **Given** a version URN, **When** passed to Model Derivative, **Then** translation can be initiated using the derivatives URN from tip version.

---

### User Story 13 - Usage Analytics Dashboard (Priority: P3) 🏢 PRO TIER

As an enterprise administrator, I need visibility into API usage patterns across my organization so that I can optimize costs, identify bottlenecks, and plan capacity.

**Why this priority**: Analytics is a key differentiator for Pro tier; enables data-driven decisions for enterprise customers.

**Independent Test**: Generate usage report for past 30 days; verify metrics accuracy against APS logs.

**Acceptance Scenarios**:

1. **Given** Pro tier license, **When** I run `raps analytics dashboard`, **Then** a summary of API calls by endpoint, user, and time period is displayed.
2. **Given** usage data for past month, **When** I run `raps analytics report --period 30d --format csv`, **Then** detailed usage report is exported.
3. **Given** real-time mode, **When** I run `raps analytics watch`, **Then** live API call metrics are streamed to terminal.
4. **Given** cost thresholds configured, **When** usage exceeds threshold, **Then** alert is generated via configured webhook.

---

### User Story 14 - Audit Logging (Priority: P3) 🏢 PRO TIER

As a compliance officer, I need immutable audit logs of all RAPS operations so that I can meet regulatory requirements and investigate security incidents.

**Why this priority**: Audit trails are mandatory for regulated industries (finance, healthcare, government).

**Independent Test**: Perform operations; verify all actions logged with timestamps, user IDs, and parameters.

**Acceptance Scenarios**:

1. **Given** audit logging enabled, **When** any RAPS command executes, **Then** log entry is created with timestamp, user, command, parameters (secrets redacted), and outcome.
2. **Given** audit log storage, **When** I run `raps audit search --user admin@company.com --since 7d`, **Then** matching entries are returned.
3. **Given** tamper detection, **When** audit logs are modified externally, **Then** integrity check fails and alert is raised.
4. **Given** export requirements, **When** I run `raps audit export --format siem`, **Then** logs are exported in SIEM-compatible format (CEF/LEEF).

---

### User Story 15 - Compliance Policies (Priority: P3) 🏢 PRO TIER

As a security administrator, I need to enforce organizational policies on RAPS usage so that users cannot perform unauthorized operations.

**Why this priority**: Policy enforcement is critical for enterprises with strict security postures.

**Independent Test**: Define policy blocking bucket deletion; verify operation is blocked with policy violation message.

**Acceptance Scenarios**:

1. **Given** policy file configured, **When** user attempts blocked operation, **Then** operation fails with "Policy violation: [policy name]".
2. **Given** role-based policies, **When** user with "viewer" role attempts upload, **Then** operation fails with "Insufficient permissions".
3. **Given** data classification rules, **When** uploading to restricted bucket, **Then** file is scanned and blocked if classification violated.
4. **Given** policy changes, **When** I run `raps compliance policy reload`, **Then** new policies take effect immediately.

---

### User Story 16 - Multi-Tenant Management (Priority: P4) 🏢 PRO TIER

As an MSP (Managed Service Provider), I need to manage multiple customer ACC accounts from a single RAPS installation so that I can efficiently serve multiple clients.

**Why this priority**: Multi-tenancy enables MSP business model; key for professional services firms.

**Independent Test**: Configure 3 customer tenants; switch between them and verify isolated credentials.

**Acceptance Scenarios**:

1. **Given** multiple tenants configured, **When** I run `raps tenant list`, **Then** all configured tenants with status are displayed.
2. **Given** tenant "acme-corp", **When** I run `raps tenant switch acme-corp`, **Then** subsequent commands use acme-corp credentials.
3. **Given** tenant isolation, **When** operating as tenant A, **Then** tenant B resources are not accessible.
4. **Given** tenant-specific config, **When** I run `raps tenant config set acme-corp region=EMEA`, **Then** acme-corp defaults to EMEA region.

---

### User Story 17 - Enterprise SSO Integration (Priority: P4) 🏢 PRO TIER

As an IT administrator, I need RAPS to authenticate via our corporate identity provider so that users don't need separate APS credentials.

**Why this priority**: SSO reduces credential sprawl and enables centralized access control.

**Independent Test**: Configure SAML/OIDC provider; authenticate via SSO and verify token exchange.

**Acceptance Scenarios**:

1. **Given** OIDC provider configured, **When** I run `raps auth login --sso`, **Then** browser opens to corporate login page.
2. **Given** successful SSO auth, **When** identity token received, **Then** it's exchanged for APS access token automatically.
3. **Given** SSO session expired, **When** command requires auth, **Then** user is prompted to re-authenticate via SSO.
4. **Given** SSO disabled for user, **When** attempting SSO login, **Then** error message suggests contacting IT admin.

---

### Edge Cases

- What happens when APS API is down? → Graceful degradation with cached data display and clear offline indicator.
- How does system handle token expiry mid-operation? → Auto-refresh if possible; prompt re-auth if refresh fails.
- What if user provides conflicting flags (`--json` and `--table`)? → Last flag wins with warning; document precedence.
- How does parallel upload handle partial failure? → Retry failed chunks; report partial success with list of failed chunks.
- What if Pro license expires? → Gracefully degrade to Community tier with warning; no data loss.
- How do tier-gated commands behave? → Clear error message indicating required tier; suggest upgrade path.
- What if SSA private key is compromised? → Support `raps ssa key disable` and immediate rotation without account deletion.
- How does SSA JWT expiry work? → JWT assertion valid for max 5 minutes; token valid for 60 minutes; refresh token valid for 15 days.
- What if SSA service account is disabled? → All JWT assertions fail; keys cannot be managed; account can be re-enabled.
- How are hub IDs formatted? → Account ID prefixed with `b.` (e.g., account `c8b0c73d-3ae9` → hub `b.c8b0c73d-3ae9`).
- What if Data Management item has no derivatives? → Tip version derivatives URN is null; must trigger translation first.
- How are region headers handled? → Support both `x-ads-region` header and `region` query param; header takes precedence.

**Pro Tier Edge Cases:**
- What if analytics storage fills up? → Rotate oldest data; alert admin before reaching capacity.
- How are audit logs backed up? → Support configurable backup to S3/Azure Blob; encrypt at rest.
- What if policy file has syntax errors? → Fail-safe to deny-all; log detailed error for admin.
- How does multi-tenant handle orphaned tenants? → `raps tenant prune` removes tenants unused for configurable period.
- What if SSO provider is down? → Fall back to cached tokens if valid; prompt for alternative auth if expired.
- How are Pro tier license keys validated? → Offline validation with periodic online refresh; grace period for network issues.

---

## Requirements *(mandatory)*

### Functional Requirements

#### Microkernel Architecture (raps-kernel)

- **FR-001**: System MUST be split into microkernel (`raps-kernel`) + service crates (`raps-oss`, `raps-derivative`, `raps-dm`)
- **FR-002**: Kernel MUST contain only: Auth, HTTP client, Config, Storage, Types, Error, Logging
- **FR-003**: Kernel MUST compile with `#![deny(warnings)]`, `#![deny(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`
- **FR-004**: Kernel LOC MUST be <3000 lines (excluding tests)
- **FR-005**: Kernel test coverage MUST be >90% on critical paths
- **FR-006**: All service crates MUST depend only on `raps-kernel`, not on each other

#### Tiered Product Strategy

- **FR-007**: System MUST support feature flags: `core`, `community` (default), `pro`
- **FR-008**: Core tier MUST include the following APS APIs:
  - **Authentication** - OAuth2 v2 (2-legged, 3-legged, refresh tokens)
  - **SSA** (Secure Service Accounts) - Robot accounts, keys, JWT assertion exchange
  - **OSS** (Object Storage Service) - Buckets, objects, S3 signed URLs
  - **Model Derivative** - Translations, manifests, thumbnails, metadata
  - **Data Management** - Hubs, projects, folders, items, versions
  - Config, Completions
- **FR-009**: Community tier MUST include Core + the following APS APIs:
  - **Account Admin** - Projects, users, companies, business units
  - **Construction.Issues** (ACC Issues) - Issues, comments, types, attributes
  - **Design Automation** - AppBundles, activities, work items
  - **Reality Capture** - Photo scenes, photogrammetry
  - **Webhooks** - Event subscriptions and notifications
  - Pipelines, Plugins, MCP, TUI
- **FR-010**: Pro tier MUST include Community + Analytics, Audit, Compliance, Multi-tenant, SSO
- **FR-011**: Tier-gated commands MUST fail gracefully with clear upgrade guidance
- **FR-012**: Version output MUST include tier name (e.g., "RAPS Community v3.2.0")

#### Pro Tier Features — Analytics, Audit, Compliance, Multi-tenant, SSO

##### Analytics (FR-PRO-ANA-*)
- **FR-PRO-ANA-001**: System MUST track API call metrics: endpoint, latency, status, user, timestamp
- **FR-PRO-ANA-002**: System MUST provide dashboard command showing usage summary by time period
- **FR-PRO-ANA-003**: System MUST support CSV/JSON export of usage reports
- **FR-PRO-ANA-004**: System MUST support real-time metrics streaming via `--watch` flag
- **FR-PRO-ANA-005**: System MUST support configurable usage alerts with webhook notifications

##### Audit (FR-PRO-AUD-*)
- **FR-PRO-AUD-001**: System MUST log all command executions with: timestamp, user, command, parameters (secrets redacted), exit code
- **FR-PRO-AUD-002**: System MUST store audit logs in append-only storage with integrity verification
- **FR-PRO-AUD-003**: System MUST support audit log search by user, date range, command, and outcome
- **FR-PRO-AUD-004**: System MUST support SIEM-compatible export formats (CEF, LEEF, JSON)
- **FR-PRO-AUD-005**: System MUST detect and alert on audit log tampering attempts

##### Compliance (FR-PRO-CMP-*)
- **FR-PRO-CMP-001**: System MUST support policy files defining allowed/blocked operations per role
- **FR-PRO-CMP-002**: System MUST enforce policies before command execution with clear violation messages
- **FR-PRO-CMP-003**: System MUST support role-based access control (RBAC) with predefined roles (admin, operator, viewer)
- **FR-PRO-CMP-004**: System MUST support data classification rules for upload operations
- **FR-PRO-CMP-005**: System MUST support hot-reload of policy changes without restart

##### Multi-tenant (FR-PRO-MTN-*)
- **FR-PRO-MTN-001**: System MUST support multiple tenant configurations in a single installation
- **FR-PRO-MTN-002**: System MUST provide tenant switching via `raps tenant switch <name>`
- **FR-PRO-MTN-003**: System MUST enforce tenant isolation (credentials, config, cache)
- **FR-PRO-MTN-004**: System MUST support per-tenant configuration overrides (region, endpoints, defaults)
- **FR-PRO-MTN-005**: System MUST support tenant listing with status and last-used timestamp

##### SSO (FR-PRO-SSO-*)
- **FR-PRO-SSO-001**: System MUST support OIDC-based SSO authentication flow
- **FR-PRO-SSO-002**: System MUST support SAML 2.0 SSO as alternative to OIDC
- **FR-PRO-SSO-003**: System MUST exchange SSO identity tokens for APS access tokens
- **FR-PRO-SSO-004**: System MUST support SSO session refresh without user interaction
- **FR-PRO-SSO-005**: System MUST fall back to standard auth when SSO is unavailable

#### Build Performance & Tooling

- **FR-013**: Workspace MUST include `.cargo/config.toml` with `lld-link` linker for Windows targets
- **FR-014**: Workspace MUST include `.cargo/config.toml` with `mold` linker for Linux targets in CI
- **FR-015**: CI pipelines MUST use `sccache` for compilation caching to accelerate builds
- **FR-016**: Workspace Cargo.toml MUST set `debug = 0` for dev and test profiles to reduce PDB overhead
- **FR-017**: README MUST document local setup for lld-link/mold, sccache, and cargo-nextest (optional but recommended)
- **FR-017a**: CI pipelines MUST use `cargo-nextest` for parallel test execution instead of `cargo test`
- **FR-017b**: CI pipelines MUST run `cargo build --timings` and upload HTML report as artifact for build diagnostics

#### Authentication & SSA (Secure Service Accounts) — APS Authentication API

- **FR-AUTH-001**: System MUST support all OAuth2 v2 grant types:
  - `client_credentials` (2-legged) for app-only access
  - `authorization_code` (3-legged) for user-delegated access
  - `refresh_token` for token renewal
  - `urn:ietf:params:oauth:grant-type:jwt-bearer` for SSA assertion exchange
- **FR-AUTH-002**: System MUST support token introspection via `/authentication/v2/introspect`
- **FR-AUTH-003**: System MUST support JWKS retrieval via `/authentication/v2/keys` for JWT validation
- **FR-SSA-001**: System MUST support SSA service account creation via `POST /authentication/v2/service-accounts`
- **FR-SSA-002**: System MUST support SSA key creation via `POST /authentication/v2/service-accounts/{id}/keys`
- **FR-SSA-003**: System MUST support JWT assertion generation (RS256) using SSA private key
- **FR-SSA-004**: System MUST support SSA token exchange with scopes: `data:read`, `data:write`, `bucket:create`, etc.
- **FR-SSA-005**: System MUST securely handle SSA private keys (environment variables, never logged)

#### Account Admin — APS Account Admin API

- **FR-ADMIN-001**: System MUST support listing projects via `GET /construction/admin/v1/accounts/{accountId}/projects`
- **FR-ADMIN-002**: System MUST support creating projects via `POST /construction/admin/v1/accounts/{accountId}/projects`
- **FR-ADMIN-003**: System MUST support user management via `/construction/admin/v1/projects/{projectId}/users`
- **FR-ADMIN-004**: System MUST support company management via `/hq/v1/accounts/{account_id}/companies`
- **FR-ADMIN-005**: System MUST support business units via `/hq/v1/accounts/{account_id}/business_units_structure`

#### Data Management — APS Data Management API

- **FR-DM-001**: System MUST support hub navigation via `GET /project/v1/hubs`
- **FR-DM-002**: System MUST support project navigation via `GET /project/v1/hubs/{hub_id}/projects`
- **FR-DM-003**: System MUST support folder contents via `GET /data/v1/projects/{project_id}/folders/{folder_id}/contents`
- **FR-DM-004**: System MUST support item versions via `GET /data/v1/projects/{project_id}/items/{item_id}/versions`
- **FR-DM-005**: System MUST support tip version retrieval via `GET /data/v1/projects/{project_id}/items/{item_id}/tip`
- **FR-DM-006**: System MUST support storage creation for uploads via `POST /data/v1/projects/{project_id}/storage`
- **FR-DM-007**: System MUST extract derivatives URN from tip version for Viewer SDK integration

#### CLI Performance & Architecture

- **FR-018**: System MUST support parallel chunk uploads for files >5MB with configurable concurrency via `--concurrency` flag
- **FR-019**: System MUST implement buffer reuse for chunk uploads to reduce memory allocations
- **FR-020**: System MUST wrap all blocking calls (dialoguer, tiny_http) with `tokio::task::spawn_blocking`
- **FR-021**: System MUST use config-based URLs for all API endpoints (eliminate hardcoded URLs)
- **FR-022**: System MUST apply retry logic consistently across ALL API operations with exponential backoff
- **FR-023**: System MUST formalize JSON output schemas and maintain backward compatibility
- **FR-024**: System MUST support streaming pagination for large result sets with `--limit` flag
- **FR-025**: System MUST pass all commands through non-interactive mode audit

#### MCP Server (raps serve)

- **FR-MCP-001**: MCP server MUST cache APS client instances to reduce lock contention
- **FR-MCP-002**: MCP server MUST implement request queuing with backpressure for rate-limited scenarios
- **FR-MCP-003**: MCP server MUST expose all bucket/object operations currently available in CLI
- **FR-MCP-004**: MCP server MUST include `tool_call_id` in all responses for request tracing
- **FR-MCP-005**: MCP server MUST expose SSA tools: `ssa_create`, `ssa_list`, `ssa_key_create`, `ssa_token`
- **FR-MCP-006**: MCP server MUST expose Data Management tools: `dm_hubs_list`, `dm_projects_list`, `dm_folders_list`, `dm_versions_list`, `dm_tip_version`

#### GitHub Action (raps-action)

- **FR-ACT-001**: Action MUST support Windows runners (currently Linux/macOS only)
- **FR-ACT-002**: Action MUST cache RAPS binary across workflow runs using GitHub cache
- **FR-ACT-003**: Action MUST expose structured JSON output as output variable
- **FR-ACT-004**: Action MUST validate version input and fail fast on invalid version

#### Docker Container (raps-docker)

- **FR-DOC-001**: Container MUST be multi-arch (linux/amd64, linux/arm64)
- **FR-DOC-002**: Container MUST NOT persist credentials to filesystem
- **FR-DOC-003**: Container MUST include health check endpoint via `raps auth test`
- **FR-DOC-004**: Container MUST pin to specific RAPS version (currently hardcoded v2.0.0)

#### TUI (aps-tui)

- **FR-TUI-001**: TUI MUST share API client code with raps crate (extract to shared library)
- **FR-TUI-002**: TUI MUST support keyboard navigation for all operations
- **FR-TUI-003**: TUI MUST display authentication status in header bar
- **FR-TUI-004**: TUI MUST support vim-style keybindings (j/k navigation, / search)

### Key Entities

- **ApsClient**: Unified HTTP client with retry, timeout, and credential management
- **OutputSchema**: Formal JSON schema definitions for all command outputs
- **McpToolRegistry**: Dynamic tool registration for MCP server extensibility
- **UploadSession**: Resumable upload state with parallel chunk tracking
- **TuiState**: Centralized TUI application state with navigation stack

#### APS API Entities (from APS OpenAPI specs)

- **ServiceAccount (SSA)**: Robot identity with email, serviceAccountId, and keys for automated authentication
- **ServiceAccountKey**: RSA key pair (kid, privateKey PEM) for JWT assertion signing
- **Hub**: BIM 360/ACC account container (id prefixed with `b.`)
- **Project**: ACC project within a hub (id prefixed with `b.`)
- **Folder**: Container for items within a project
- **Item**: File resource with one or more versions
- **Version**: Specific revision of an item with derivatives URN for Viewer SDK
- **Bucket**: OSS storage container with retention policy (transient, temporary, persistent)
- **Object**: File stored in OSS bucket with objectId/objectKey
- **Manifest**: Translation job status and derivative URNs for Model Derivative
- **Issue**: ACC issue with type, subtype, status, assignee (Construction.Issues API)
- **Webhook**: Event subscription with callback URL and filter criteria

---

## Clarifications

### Session 2025-12-29

- Q: Should the spec include explicit build-time performance targets for the raps-kernel crate? → A: Yes, add `cargo check -p raps-kernel` < 5s, full workspace check < 30s
- Q: Should the spec require specific build tooling in `.cargo/config.toml` and CI? → A: Require lld-link + sccache in CI, document for local (balanced approach)
- Q: Should the spec include advanced cross-platform build optimizations beyond Windows lld-link? → A: Add mold linker for Linux CI builds (proven, significant speedup)
- Q: Should the spec require cargo-nextest for test execution in CI? → A: Require cargo-nextest in CI for parallel test execution
- Q: Should CI capture and report build timing metrics? → A: Capture `--timings` HTML report as CI artifact (visibility)

### Session 2025-12-30 (APS Alignment + Clarifications)

- Q: How should FR numbers be organized to avoid conflicts across sections? → A: Use unique prefixes per component (FR-MCP-*, FR-ACT-*, FR-DOC-*, FR-TUI-*)
- Q: Should MCP server expose tools for SSA, Account Admin, and Data Management? → A: Expose SSA and Data Management tools; Account Admin via CLI only (less frequent operations)
- Q: Should Pro tier features be defined now or deferred? → A: Define now with complete user stories (US-13 to US-17) and functional requirements (FR-PRO-*)

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Large file upload (100MB) completes in <30s on 100Mbps connection (currently ~2min)
- **SC-002**: Zero blocking async calls detected by `cargo clippy` with async lints enabled
- **SC-003**: 100% of commands pass non-interactive mode audit (no hanging in CI)
- **SC-004**: MCP server memory usage remains <100MB under sustained load (100 req/min for 1 hour)
- **SC-005**: GitHub Action installation time <30s (including download and extraction)
- **SC-006**: Docker image size <50MB (slim variant)
- **SC-007**: JSON output schemas documented and validated with JSON Schema Draft 7
- **SC-008**: TUI launches and displays auth status in <500ms
- **SC-009**: `cargo check -p raps-kernel` completes in <5s (incremental build)
- **SC-010**: `cargo check` for full workspace completes in <30s (incremental build)

#### Pro Tier Success Criteria
- **SC-PRO-001**: Analytics dashboard renders usage summary in <2s for 30-day period
- **SC-PRO-002**: Audit log queries return results in <1s for 10,000+ entries
- **SC-PRO-003**: Policy enforcement adds <10ms latency to command execution
- **SC-PRO-004**: Tenant switching completes in <500ms including credential load
- **SC-PRO-005**: SSO authentication flow completes in <5s (excluding user interaction)

### Performance Benchmarks (from Constitution)

| Metric | Target | Current | Gap |
|--------|--------|---------|-----|
| CLI startup (--help) | <100ms | ~80ms | ✅ Met |
| Auth test | <500ms | ~300ms | ✅ Met |
| 100MB upload (parallel) | <30s | ~120s | ❌ 4x gap |
| Bucket list (100 items) | <2s | ~1.5s | ✅ Met |
| MCP tool call | <200ms | ~150ms | ✅ Met |

---

## Technical Approach

### Phase 1: Performance & Architecture (Priority Issues)

1. **Parallel Multipart Upload** (Issue #70)
   - Implement `FuturesUnordered` for concurrent chunk uploads
   - Respect `--concurrency` flag for limit
   - Add progress tracking per chunk

2. **Blocking Async Fix** (Issue #73)
   - Audit all `dialoguer` and `tiny_http` usage
   - Wrap with `spawn_blocking`
   - Add clippy lint for blocking detection

3. **Unified Retry Logic** (Issue #77)
   - Extract retry logic to `http.rs` module
   - Apply to all API clients consistently
   - Configurable retry count and backoff

4. **Config-Based URLs** (Issue #76)
   - Define all APS endpoints in config
   - Support environment-based overrides (staging vs production)
   - Eliminate hardcoded URLs

### Phase 2: Cross-Interface Consistency

1. **Output Schema Formalization**
   - Define JSON Schema for each command output
   - Generate schema documentation
   - Add backward-compatibility tests

2. **MCP-CLI Parity**
   - Audit MCP tools against CLI commands
   - Add missing operations
   - Ensure error codes match

3. **GitHub Action Enhancements**
   - Add Windows runner support
   - Implement binary caching
   - Add structured JSON output

### Phase 3: TUI Development

1. **Shared API Library**
   - Extract `raps-core` crate for shared code
   - Move API clients to shared crate
   - Update both `raps` and `aps-tui` to use shared crate

2. **TUI Feature Completion**
   - Bucket/Object browsing panels
   - Upload/Download with progress
   - Translation status viewer

### Phase 4: Container & Distribution

1. **Docker Improvements**
   - Update to current RAPS version (3.1.0)
   - Add health check
   - Multi-arch manifest

2. **Distribution Channels**
   - Homebrew tap maintenance
   - Scoop bucket maintenance
   - Cargo installation verification

---

## Constitution Compliance

This specification aligns with the RAPS Ecosystem Constitution v1.0.0:

| Principle | Compliance Status |
|-----------|-------------------|
| I. Rust Idiomatic Code Quality | ✅ All changes follow clippy/rustfmt |
| II. Cross-Repository Consistency | ✅ Shared crate extraction planned |
| III. Test-First Development | ✅ Tests required before implementation |
| IV. User Experience Consistency | ✅ Output schema formalization |
| V. Performance & Resource Efficiency | ✅ Primary focus of Phase 1 |
| VI. Security & Secrets Handling | ✅ Container credential handling |

---

## Dependencies & Risks

### Dependencies

- `rmcp` crate stability for MCP server
- APS API availability for integration testing
- GitHub Actions runner availability for CI

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking changes to MCP protocol | Low | High | Pin rmcp version, test against spec |
| APS API deprecation | Low | High | Abstract API layer, version pin |
| Rust edition issues | Medium | Medium | MSRV policy, CI matrix testing |

---

## Out of Scope

- GUI desktop application (use TUI instead)
- Mobile applications
- Language bindings (Python, Node.js)—may be future consideration
- APS API proxy/gateway functionality

---

## References

### Project Documentation
- [RAPS README](../raps/README.md)
- [RAPS CHANGELOG](../raps/CHANGELOG.md)
- [Issues README](../raps/issues/README.md) - 15 tracked improvement issues
- [Constitution](../.specify/memory/constitution.md)
- [APS OpenAPI Specs](../aps-sdk-openapi/)

### APS API Documentation (llms-full.md)
- **[APS APIs Full Reference](./llms-full.md)** - Complete API documentation for LLM consumption
  - Construction.Issues API - ACC issue tracking
  - Account Admin API - Project/user management
  - Authentication API - OAuth2 v2 and SSA
  - Data Management API - Hubs, projects, folders, items
  - Model Derivative API - Translations and metadata
  - OSS API - Object storage with S3 signed URLs
  - SSA API - Secure Service Accounts for automation
  - Webhooks API - Event notifications

### External APS Documentation
- [APS Developer Portal](https://aps.autodesk.com/developer/documentation)
- [SSA Getting Started](https://developer.doc.autodesk.com/bPlouYTd/cloud-platform-ssa-docs-main-460369/)
- [Model Derivative API](https://aps.autodesk.com/en/docs/model-derivative/v2/developers_guide/overview/)
- [Data Management API](https://aps.autodesk.com/en/docs/data/v2/developers_guide/overview/)

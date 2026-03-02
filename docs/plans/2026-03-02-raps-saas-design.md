# RAPS SaaS Platform Design

**Date**: 2026-03-02
**Status**: Approved
**Author**: Dmytro Yemelianov + Claude

## Overview

Transform RAPS from a CLI tool into a full SaaS platform serving both AEC professionals (via web UI) and developers (via REST API/SDK). The platform combines managed APS infrastructure, multi-tenant governance, and AI-powered workflows via the existing MCP server.

## Approach

**API Gateway + Dashboard (A) with MCP/AI Layer (B)**: Wrap the existing 9 raps crates behind an Axum API gateway with multi-tenancy, add a React dashboard incrementally, and differentiate with AI-powered natural language APS interactions.

Key design decisions:

- **Single Rust binary**: API gateway, MCP server, and background workers in one process. Split only when scale demands it.
- **Four client types**: React SPA, REST API consumers, MCP/AI clients (Claude Desktop, Cursor), and RAPS CLI with `--cloud` mode.
- **Hybrid auth**: RAPS-managed OAuth for quick-start, BYOK (Bring Your Own Keys) for enterprises.
- **Incremental delivery**: each phase delivers standalone value.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Clients                                     │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌────────────────┐  │
│  │ React SPA │  │ REST API  │  │ MCP Client│  │ RAPS CLI       │  │
│  │ Dashboard │  │ consumers │  │ (Claude)  │  │ (cloud mode)   │  │
│  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └──────┬─────────┘  │
└────────┼──────────────┼──────────────┼───────────────┼──────────────┘
         │              │              │               │
         └──────────────┴──────┬───────┴───────────────┘
                               │
                    ┌──────────▼──────────┐
                    │   Edge / CDN        │
                    │   (Cloudflare)      │
                    └──────────┬──────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────────┐
│                    RAPS Cloud (Axum)                                 │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  Middleware Layer                                             │   │
│  │  Auth/JWT │ Rate Limit │ Metering │ Tenant Context            │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌─────────────────────┐  ┌─────────────────────────────────────┐   │
│  │  REST API Routes    │  │  MCP Server (per-tenant)            │   │
│  │  /api/v1/*          │  │  + AI orchestration layer           │   │
│  └─────────┬───────────┘  └──────────────┬──────────────────────┘   │
│            │                              │                          │
│  ┌─────────▼──────────────────────────────▼──────────────────────┐  │
│  │  Service Layer (raps crates as libraries)                     │  │
│  │  raps-kernel │ raps-derivative │ raps-oss │ raps-dm/da/acc    │  │
│  └──────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  Background Workers                                          │   │
│  │  Job Runner │ Webhook Receiver │ Cron Scheduler               │   │
│  └──────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
         │              │              │
    PostgreSQL       Redis        Autodesk APS
    (tenants,      (sessions,     (proxied API
     jobs, audit)   queues,        calls)
                    rate limits)
```

## Data Model

### Core Entities

**Tenant/Organization**: id, name, slug, plan_tier, created_at

**Users**: id, email, role (owner|admin|member|viewer), tenant_id, auth_provider, avatar_url

**APS Credentials**: id, tenant_id, label, mode (byok|oauth), client_id (encrypted), client_secret (encrypted), oauth_token (encrypted), refresh_token (encrypted), scopes[], is_default

**API Keys**: id, tenant_id, key_hash, prefix, scopes[], rate_limit, last_used_at, expires_at

**Jobs**: id, tenant_id, credential_id, kind (translate|upload|download|pipeline|bulk_admin|report|reality|webhook_setup|ai_workflow), status (queued|running|completed|failed|cancelled), input (jsonb), output (jsonb), error, started_at, completed_at, duration_ms

**Workflows**: id, tenant_id, name, trigger (cron|webhook|manual|ai), pipeline_yaml, schedule, enabled, last_run_at, next_run_at

**Audit Log**: id, tenant_id, user_id, action, resource_type, resource_id, ip_address, metadata (jsonb), created_at

**Usage Metering**: id, tenant_id, period (YYYY-MM), api_calls, translations, storage_bytes, ai_tokens, webhook_deliveries

### Multi-Tenancy

Shared database with tenant_id column on every table. PostgreSQL Row-Level Security (RLS) enforces isolation at the DB level.

### Credential Encryption

- AES-256-GCM with per-tenant data encryption keys (DEKs)
- Master key (AWS KMS or env var for dev) wraps per-tenant DEKs wraps credentials
- Never logged, never in API responses

## REST API

Base URL: `https://api.rapscli.xyz/v1`

Auth: `Authorization: Bearer <jwt>` or `X-API-Key: raps_<key>`

Standard envelope: `{ "data": {...}, "meta": { "request_id", "duration_ms" }, "pagination": { "cursor", "has_more" } }`

### Routes

```
Auth & Identity
  POST   /auth/signup
  POST   /auth/login
  POST   /auth/login/google
  POST   /auth/login/autodesk
  POST   /auth/refresh
  POST   /auth/logout

Tenant Management
  GET    /tenant
  PATCH  /tenant
  GET    /tenant/members
  POST   /tenant/members/invite
  PATCH  /tenant/members/:id
  DELETE /tenant/members/:id

APS Credentials
  GET    /credentials
  POST   /credentials
  GET    /credentials/:id
  PATCH  /credentials/:id
  DELETE /credentials/:id
  POST   /credentials/:id/test

API Keys
  GET    /api-keys
  POST   /api-keys
  DELETE /api-keys/:id

OSS (via raps-oss)
  GET    /oss/buckets
  POST   /oss/buckets
  GET    /oss/buckets/:key
  DELETE /oss/buckets/:key
  GET    /oss/buckets/:key/objects
  PUT    /oss/buckets/:key/objects
  GET    /oss/objects/:urn
  DELETE /oss/objects/:urn

Model Derivative (via raps-derivative)
  POST   /translate
  GET    /translate/:urn
  GET    /translate/:urn/derivatives
  GET    /translate/:urn/download
  DELETE /translate/:urn

Data Management (via raps-dm)
  GET    /dm/hubs
  GET    /dm/hubs/:id/projects
  GET    /dm/projects/:id/folders
  GET    /dm/folders/:id/contents
  GET    /dm/items/:id/versions

ACC (via raps-acc)
  GET    /acc/projects/:id/issues
  POST   /acc/projects/:id/issues
  PATCH  /acc/issues/:id
  GET    /acc/projects/:id/rfis
  POST   /acc/projects/:id/rfis
  (assets, submittals, checklists follow same pattern)

Design Automation (via raps-da)
  GET    /da/engines
  GET    /da/activities
  POST   /da/activities
  POST   /da/workitems
  GET    /da/workitems/:id

Jobs
  GET    /jobs
  GET    /jobs/:id
  POST   /jobs/:id/cancel
  POST   /jobs/:id/retry

Workflows (Phase 3)
  GET    /workflows
  POST   /workflows
  GET    /workflows/:id
  PATCH  /workflows/:id
  DELETE /workflows/:id
  POST   /workflows/:id/run
  GET    /workflows/:id/runs

AI Chat (Phase 3)
  POST   /ai/chat
  GET    /ai/chat/history
  POST   /ai/chat/approve

Usage
  GET    /usage
  GET    /usage/history
```

### Design Principles

- Thin proxy: routes map 1:1 to raps crate methods
- Credential selection via optional `X-Credential-Id` header
- Async jobs for long operations (translate, pipeline, etc.) return 202 + job ID
- WebSocket at `/ws` for real-time job progress and AI chat streaming
- Cursor pagination everywhere
- Rate limiting per-tenant with headers: X-RateLimit-Limit, Remaining, Reset

## AI / MCP Layer

### AI Chat Flow

1. **Intent Parser** (LLM call): user message -> structured action plan identifying needed raps crates and operations
2. **Action Plan Generator**: produces concrete API call sequence with estimates
3. **Confirmation Gate**: presents plan to user for approval (all write operations require explicit approval)
4. **Tool Executor**: runs the plan using raps crates, streaming progress back

### Hosted MCP Endpoint

SSE endpoint: `https://mcp.rapscli.xyz/tenant/:slug/sse`

Authenticated with Bearer token. External clients (Claude Desktop, Cursor) connect directly.

### MCP Tools

Existing 14 tools plus new SaaS tools: folder_browse, item_search, issue_list, issue_create, rfi_list, workflow_create, workflow_run, report_generate, bulk_translate, project_health.

### "Project Health" Feature

AI aggregates issues, RFIs, model status, and overdue items into a natural language project summary with suggested actions.

### AI Safety Controls

- Confirmation gate for all write operations
- Scope limits: AI uses same RBAC as user
- Audit trail: every AI action logged with prompt and plan
- Per-tenant daily AI token budget with warnings
- Dry-run mode available
- Prompt injection defense: tool outputs sanitized

### Workflow Engine

Reuses existing raps pipeline YAML engine with trigger wrappers:

- Cron triggers (tokio-cron-scheduler)
- Webhook triggers (APS events)
- AI-composed triggers (natural language -> YAML)
- Manual triggers

## Frontend

### Tech Stack

- React 19 + Vite
- TanStack Router (type-safe, file-based)
- TanStack Query (server state)
- shadcn/ui + Tailwind CSS
- React Hook Form + Zod (forms/validation)
- Recharts (usage dashboards)
- Native WebSocket (real-time)
- Zustand (auth state)
- Deployed on Cloudflare Pages

### Key Pages

- **Dashboard**: job stats, recent activity, usage meter, active workflows
- **AI Chat**: streaming messages, action approval cards, chat history
- **Storage Browser**: bucket list -> object list -> upload/download
- **Translations**: start form, status polling, derivative downloads
- **Projects**: hub -> project -> folder tree browser
- **ACC**: issues, RFIs, assets, submittals (tabbed per project)
- **Workflows**: list, create/edit (visual + YAML), execution history
- **Jobs**: filterable history table, detail view with logs
- **Settings**: credentials, API keys, team management, usage

### CLI Cloud Mode

Existing RAPS CLI gains `--cloud` flag routing all APS calls through `api.rapscli.xyz/v1/` instead of directly to APS. Same command logic, different transport.

## Deployment & Infrastructure

### Phase 1 Stack (MVP)

| Component | Service | Cost |
|-----------|---------|------|
| Backend | Fly.io (1-2 machines) | ~$5/mo |
| Database | Neon Postgres (serverless) | Free tier |
| Cache | Upstash Redis (serverless) | Free tier |
| Frontend | Cloudflare Pages | Free |
| CDN/WAF | Cloudflare | Free |
| Logging | Axiom (via Fly.io drain) | Free tier |
| Metrics | Grafana Cloud | Free tier |
| Errors | Sentry | Free tier |

**Monthly cost at launch: ~$0-15/mo**

### Growth Path

- **0-100 users**: Fly.io 1-2 machines, Neon free, Upstash free
- **100-1K users**: Fly.io 3-5 machines (2 regions), Neon Pro, Upstash Pro, S3 for file staging
- **1K-10K users**: AWS ECS, RDS Postgres, ElastiCache, CloudFront
- **10K+ users**: Kubernetes (EKS), RDS Multi-AZ + read replicas, dedicated worker pools

### Security

- Cloudflare WAF (DDoS, bot protection)
- TLS everywhere (Fly.io auto-certs)
- JWT with short expiry (15min) + refresh tokens
- argon2 password hashing
- CSRF protection, CSP headers
- sqlx compile-time query checking (SQL injection prevention)
- APS credentials: AES-256-GCM with key hierarchy
- Audit log: immutable, append-only
- GDPR-ready (deletion/export endpoints)

## Crate Structure

```
raps/
├── raps-kernel/          # existing (shared by CLI and cloud)
├── raps-oss/             # existing
├── raps-derivative/      # existing
├── raps-dm/              # existing
├── raps-da/              # existing
├── raps-acc/             # existing
├── raps-webhooks/        # existing
├── raps-reality/         # existing
├── raps-admin/           # existing
├── raps-cli/             # existing (unchanged)
├── raps-cloud/           # NEW — SaaS Axum server
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── db/               # sqlx models, migrations, queries
│   │   ├── middleware/       # auth, tenant, rate_limit, metering
│   │   ├── routes/           # REST API handlers
│   │   ├── services/         # business logic
│   │   ├── jobs/             # background job runner
│   │   ├── ai/              # LLM orchestrator
│   │   ├── mcp/             # hosted MCP server
│   │   └── crypto.rs        # credential encryption
│   └── migrations/
└── raps-dashboard/       # NEW — React SPA
```

## Implementation Roadmap

### Phase 1: Foundation (weeks 1-6)

API gateway live with core APS operations via REST.

- **Week 1-2**: Project scaffold, database schema, JWT auth, tenant middleware, credential CRUD (encrypted), health endpoint
- **Week 3-4**: APS proxy layer (OSS + translate + DM routes), rate limiting, metering, error mapping
- **Week 5-6**: Async job system, job runner, CI/CD pipeline, staging + production deploy

### Phase 2: Dashboard (weeks 7-12)

Web UI for non-technical users.

- **Week 7-8**: Frontend scaffold, auth pages, layout shell, credentials settings page
- **Week 9-10**: Dashboard home, storage browser, translation page, jobs page, team settings
- **Week 11-12**: Projects page (folder tree), WebSocket integration, responsive design, polish

### Phase 3: AI Layer (weeks 13-20)

Chat interface, hosted MCP, workflow engine.

- **Week 13-14**: AI orchestrator backend (LLM integration, intent parser, confirmation gate, streaming)
- **Week 15-16**: Chat UI (streaming, action cards, history, suggested prompts)
- **Week 17-18**: Hosted MCP server, expanded tool set, Claude Desktop/Cursor guides
- **Week 19-20**: Workflow engine (CRUD, cron/webhook/AI triggers, visual builder, execution history)

### Phase 4: Platform & Growth (weeks 21-30)

Enterprise features, billing, ecosystem.

- **Week 21-23**: Full RBAC, audit log viewer, cross-project reporting, ACC modules in UI, Project Health AI feature
- **Week 24-26**: Stripe billing, plan tiers, API key management, developer docs, TypeScript + Python SDK
- **Week 27-30**: Pipeline template gallery, CLI --cloud mode, social login, onboarding wizard, email notifications, landing + pricing pages

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| APS rate limits hit by multiple tenants | Per-tenant queuing, backpressure, spread calls |
| LLM costs spiral | Token budgets, cache common queries, Haiku for parsing + Sonnet for planning |
| Credential breach | Encryption at rest, key hierarchy, audit logs, pen test before launch |
| Solo dev bottleneck | Phase 1-2 solo-doable; hire for Phase 3 (AI) or Phase 2 (frontend) |
| APS API changes | raps crates abstract APS; pin API versions; integration tests against raps-mock |
| Low adoption | Validate after Phase 2 with AEC community before investing in Phase 3-4 |

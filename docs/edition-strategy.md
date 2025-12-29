---
layout: default
title: Edition Strategy (Core / Community / Pro)
---

# Edition Strategy: Core, Community, and Pro

This plan separates RAPS into purpose-built editions and repositories while keeping a shared foundation. It also enumerates Autodesk Platform Services (APS) APIs that are still missing or need deeper coverage so we can align future work with Autodesk documentation.

## Repository Split

| Repository | Purpose | Contents |
|------------|---------|----------|
| **raps-core** | Shared, MIT-licensed foundation | HTTP client, auth flows, config/profile loader, base APS clients (OSS, Model Derivative, Data Management, Issues, Webhooks, Design Automation, Reality Capture), typed errors, telemetry hooks, and common utilities. |
| **raps-community** | Open-source CLI | CLI UX, docs site, examples, community plugins, and the features mapped to the Community edition (below). Depends on `raps-core` crates. |
| **raps-pro** | Commercial/enterprise | Advanced and scaled workflows, license gating, usage analytics, and connectors that rely on premium Autodesk APIs. Built on `raps-core`; may vendor community CLI commands where alignment is required. |

## Edition Feature Split

| Area | Core Library | Community Edition | Pro Edition |
|------|--------------|-------------------|-------------|
| Auth & Profiles | ✅ All auth flows, profile parsing, token inspection | ✅ CLI commands | ✅ Same as Community |
| OSS Basics | ✅ Buckets/objects CRUD, signed URLs | ✅ Single-file upload/download, resumable uploads | ✅ **Batch/parallel transfers**, retention policies, lifecycle helpers |
| Model Derivative | ✅ Job submission/status, manifest retrieval | ✅ CLI commands + presets | ✅ **Batch translations**, priority queues, callback orchestration |
| Data Management (DM) | ✅ Hubs/projects/folders/items | ✅ Read/write + item binding | ✅ **Bulk folder/item operations**, delta sync for hubs/projects |
| ACC Issues/RFIs/Assets/Submittals/Checklists | ✅ Client structs | ✅ CLI CRUD | ✅ **Bulk issue imports/exports**, templated creation, reporting bundles |
| Design Automation | ✅ Engines, activities, work items | ⬜ Basic CLI parity (finish remaining endpoints) | ✅ **Work item batching**, render farms, cold-start mitigation |
| Webhooks | ✅ Subscription CRUD | ✅ CLI | ✅ **Bulk subscription management**, routing/transform rules |
| Pipelines & Plugins | ✅ Pipeline runner engine | ✅ CLI pipelines & community plugins | ✅ **Managed pipeline runs**, schedules, org-level secrets |
| MCP Server | ✅ Protocol adapter traits | ✅ AI tools exposed as today (single-tenant, local policy only) | ✅ **Org-scoped tools**, per-tool RBAC, audited tool runs, rate limits, secret management, policy bundles |
| Support/Docs | ✅ Inline docs | ✅ Community docs | ✅ Pro-only runbooks, SSO, support SLAs |

> **Batch processing is Pro-only**: Parallel uploads/downloads, bulk translations, and mass issue/subscription operations live in `raps-pro`, with the community edition keeping single-item or small-batch workflows.

## Missing/Specific APS APIs to Add

The following Autodesk APIs are either absent or only partially covered. Align implementations with the official APS documentation before shipping.

| Service | Gap | Target Edition | Notes |
|---------|-----|----------------|-------|
| **Design Automation v3** | Full app bundle lifecycle (aliases/versions), engine parameters, activity versioning parity | Community for parity; Pro for batching | Finish parity in community; keep large-scale batching/queue tuning in Pro. |
| **Model Properties / Metadata API** | Property database download and querying for SVF2 | Community | Enables property queries alongside manifests; foundation for downstream analytics. |
| **Data Exchange API** | Exchange creation/export/import flows | Community | Bridges ACC/DM with other tools; exposes exchange IDs and lifecycle. |
| **ACC Cost Management API** | Budget, cost items, contracts, change orders | Pro | Enterprise-only workflows; include reporting exports and approval routing. |
| **ACC Transmittals API** | Package creation, distribution lists, download tracking | Pro | Align with commercial/legal workflows; pair with audit exports. |
| **ACC Model Coordination / Clash API** | Coordination spaces, clash tests, issue linking | Community (core flows); Pro (batch reruns) | Supports clash-to-issue pipelines; Pro handles batch reruns and analytics. |
| **ACC Forms & Meetings APIs** | Forms CRUD, meeting minutes, attendance | Community | Completes ACC project-management surface area. |
| **ACC Data Connector** | Data extraction jobs and dataset downloads | Pro | Fits batch/analytics theme; integrate with pipeline runner. |
| **ACC Checklists API** | Templates, assignments, inspections | Community | Complements Issues/Assets for field ops; surface in CLI. |
| **BIM 360/ACC Locations API** | Location tree CRUD for field management | Community | Needed for richer issue/checklist context. |
| **Reality Capture** | Webhook-style callbacks for photoscene completion | Community | Extend existing client with callback registration/testing. |

## Migration Steps

1. **Extract `raps-core`**: Move shared clients and configuration into a reusable crate; keep feature flags to isolate services.
2. **Fork CLI into `raps-community`**: Point to `raps-core`, remove Pro-only flags, and document open-source support scope.
3. **Stand up `raps-pro`**: Add license enforcement, telemetry toggles, and Pro-only commands (batch, cost, data connector, large-scale DA/translation flows).
4. **CI/CD alignment**: Separate pipelines per repo; shared lint/test workflows in `raps-core` reused by Community/Pro.
5. **Docs & Support**: Publish edition matrix on the docs site; add upgrade paths and API availability notes per edition.

This plan keeps a single shared foundation while clearly separating community-ready tooling from enterprise-scale features tied to Autodesk's premium APIs.

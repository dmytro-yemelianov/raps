# GDPR Privacy Compliance Statement

**Last Updated:** 2026-03-01
**RAPS Version:** 4.14.1
**Regulation:** EU General Data Protection Regulation (2016/679)

## 1. Overview

RAPS is a local-first Rust CLI tool for interacting with Autodesk Platform Services (APS). It runs entirely on the user's machine with no telemetry, no analytics, and no phone-home behavior. RAPS does not operate as a data controller in the GDPR sense; it is a local utility that the user (or their organization) directs to interact with Autodesk APIs on their behalf.

This document serves as a transparency notice under GDPR Articles 12-14.

## 2. Data Inventory

| # | Data Category | Examples | Storage Location | Retention | Persistence |
|---|---------------|----------|------------------|-----------|-------------|
| 1 | OAuth credentials | `client_id`, `client_secret`, access/refresh tokens | OS keyring (DPAPI / Keychain / SecretService) or local file with `0o600` permissions | Until user revokes or deletes | Persistent |
| 2 | User PII (ACC admin) | Email addresses, display names for bulk user management | In-memory only | Duration of CLI invocation | Transient |
| 3 | CAD files/objects | Design files uploaded/downloaded via APS OSS API | Streamed to/from user-specified paths | User-controlled | Pass-through |
| 4 | Project metadata | Hub names, project names, folder structures | Displayed to stdout/stderr | Duration of CLI invocation | Transient |
| 5 | Log files | CLI operations, errors, HTTP status codes | `~/.local/share/raps/logs/` (mode `0o700`) | 7-day rotation, 50 MB cap | Persistent (auto-pruned) |

### Log File Details

- **Location:** `~/.local/share/raps/logs/`
- **Directory permissions:** `0o700` (owner-only access)
- **Rotation:** Daily, 7-day retention, 50 MB maximum
- **Secret redaction:** All log output passes through `RedactingMakeWriter`, which strips OAuth tokens, client secrets, and other sensitive values before they reach disk

## 3. Data Flow

```
                           HTTPS / TLS 1.2+ (rustls)
                          ┌──────────────────────────────┐
                          │                              │
 ┌──────────┐   CLI cmd   │  ┌───────┐    APS REST API  │  ┌──────────────────┐
 │          │────────────>│  │       │─────────────────>│  │                  │
 │   User   │             │  │ RAPS  │                  │  │  Autodesk APS    │
 │          │<────────────│  │       │<─────────────────│  │  Cloud Services  │
 └──────────┘   stdout    │  └───┬───┘                  │  └──────────────────┘
                          │      │                      │
                          └──────│──────────────────────┘
                                 │          User's Machine
                     ┌───────────┼───────────────┐
                     │           │               │
              ┌──────▼──────┐ ┌──▼────┐ ┌────────▼────────┐
              │  OS Keyring │ │ Logs  │ │ Config files    │
              │  (secrets)  │ │ (red- │ │ (~/.config/raps)│
              │             │ │ acted)│ │                 │
              └─────────────┘ └───────┘ └─────────────────┘
```

**Key properties:**

- All network traffic uses HTTPS with TLS 1.2+ enforced via `rustls` (no OpenSSL)
- No data is sent to any third party; the only remote endpoint is Autodesk APS
- MCP server mode communicates via stdio only; it does not open network listeners
- RAPS itself does not store user data in any cloud service

## 4. GDPR Article Mapping

| Article | Topic | RAPS Posture | Evidence |
|---------|-------|--------------|----------|
| Art. 5 | Data processing principles | Data minimization: only processes data the user explicitly requests. No surplus collection, no derived data, no profiling. | No telemetry code in codebase |
| Art. 6 | Lawful basis for processing | User's explicit CLI invocation constitutes consent (Art. 6(1)(a)) or performance of a contract (Art. 6(1)(b)) depending on context. | CLI is user-initiated only |
| Art. 12-14 | Transparency | This document. Also: `--help` on every command, open-source codebase. | This file; `docs/` directory |
| Art. 15 | Right of access | All data stored locally on user's filesystem. User has full access via standard OS tools. | Keyring + `~/.config/raps/` + `~/.local/share/raps/` |
| Art. 17 | Right to erasure | Complete erasure procedure documented below. No remote data held by RAPS. | See [Section 5](#5-data-erasure-procedure) |
| Art. 25 | Data protection by design | No telemetry, local-first architecture, automatic secret redaction in logs, OS keyring for credential storage. | `RedactingMakeWriter`, `storage.rs` |
| Art. 32 | Security of processing | TLS 1.2+ only, OS keyring, `0o600`/`0o700` file permissions, PKCE for OAuth, ASVS L2 at 94% compliance. | `docs/security/asvs-l2-compliance-matrix.md` |
| Art. 33-34 | Breach notification | Security incident response documented with 48-hour initial response SLA. | `SECURITY.md` |
| Art. 35 | Data Protection Impact Assessment | Low risk: local CLI tool, no profiling, no automated decision-making, no large-scale processing of special categories. | Architecture is inherently low-risk |
| Art. 44-49 | International data transfers | RAPS sends data to Autodesk APS endpoints as directed by the user. Autodesk maintains its own GDPR compliance program. RAPS does not independently transfer data to third countries. | No third-party integrations |

## 5. Data Erasure Procedure

To completely remove all RAPS data from a system:

```bash
# 1. Remove configuration and local data
rm -rf ~/.config/raps/
rm -rf ~/.local/share/raps/

# 2. Clear OS keyring entries
#    Linux (secret-tool):
secret-tool clear service raps

#    macOS (security CLI):
security delete-generic-password -s raps

#    Windows (PowerShell):
cmdkey /delete:raps
```

After these steps, no RAPS-related data remains on the machine. Data previously sent to Autodesk APS (files, user records) is governed by Autodesk's own data retention policies.

## 6. Data Sub-Processor

| Sub-Processor | Purpose | Data Shared | GDPR Basis |
|---------------|---------|-------------|------------|
| Autodesk (APS) | Cloud platform APIs for BIM/CAD/ACC operations | OAuth tokens, file content, user PII (ACC admin), project metadata | User-directed API calls; Autodesk's own DPA applies |

RAPS has no other sub-processors. No analytics providers, no crash reporters, no CDN, no third-party services.

## 7. Data Protection Impact Assessment Summary

| Factor | Assessment |
|--------|------------|
| Nature of processing | Local CLI tool executing user-requested API calls |
| Scope | Single user or scripted automation on one machine |
| Context | Developer/administrator tooling for APS |
| Purpose | Interact with Autodesk Platform Services |
| Risk to data subjects | Low — no profiling, no automated decisions, no large-scale processing |
| Mitigations in place | TLS-only, OS keyring, file permissions, secret redaction, no telemetry |
| DPIA required? | No — does not meet Art. 35(3) thresholds |

## 8. Contact

For security issues, see `SECURITY.md` in the repository root.
For privacy questions related to data held by Autodesk, consult [Autodesk's Privacy Statement](https://www.autodesk.com/company/legal-notices-trademarks/privacy-statement).

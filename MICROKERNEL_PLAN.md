# Microkernel Architecture Refactoring Plan

## Overview

Refactor RAPS from a monolithic single-crate design to a Cargo workspace with a microkernel architecture. The kernel contains core shared functionality (auth, config, HTTP, storage), while service-specific functionality lives in separate crates.

## Target Architecture

```
raps/                           # Workspace root
├── Cargo.toml                  # Workspace manifest
├── raps-kernel/                # Core shared functionality
├── raps-oss/                   # Object Storage Service
├── raps-derivative/            # Model Derivative
├── raps-dm/                    # Data Management (hubs, projects, folders, items)
├── raps-da/                    # Design Automation
├── raps-acc/                   # ACC/BIM 360 (issues, RFI, assets, submittals)
├── raps-webhooks/              # Webhooks
├── raps-reality/               # Reality Capture
└── raps-cli/                   # Main CLI binary (current src/)
```

## Dependency Graph

```
raps-cli (main binary)
    ├── raps-kernel (required)
    ├── raps-oss (optional feature)
    ├── raps-derivative (optional feature)
    ├── raps-dm (optional feature)
    ├── raps-da (optional feature)
    ├── raps-acc (optional feature)
    ├── raps-webhooks (optional feature)
    └── raps-reality (optional feature)

All service crates depend on: raps-kernel
```

## Crate Contents

### raps-kernel (Core)
- `error.rs` - Exit codes, error handling
- `logging.rs` - Global verbosity flags
- `interactive.rs` - Interactive mode flags
- `http.rs` - HTTP client config, retry logic
- `output.rs` - Multi-format output
- `config.rs` - Profile/credential management
- `storage.rs` - Token storage abstraction
- `auth.rs` - OAuth authentication

### Service Crates
| Crate | API Client |
|-------|------------|
| `raps-oss` | `OssClient` |
| `raps-derivative` | `DerivativeClient` |
| `raps-dm` | `DataManagementClient` |
| `raps-da` | `DesignAutomationClient` |
| `raps-acc` | `IssuesClient`, `RfiClient`, `AccClient` |
| `raps-webhooks` | `WebhooksClient` |
| `raps-reality` | `RealityCaptureClient` |

### raps-cli (Binary)
- `main.rs` - Entry point
- `commands/*` - Command implementations
- `shell.rs` - Interactive shell
- `plugins.rs` - Plugin system
- `mcp/*` - MCP server

## Benefits

1. **Modularity**: Each service is independently versioned and testable
2. **Compile Times**: Only rebuild changed crates
3. **Optional Features**: Users can disable unused services
4. **Clear Boundaries**: Enforced separation of concerns
5. **Parallel Development**: Teams can work on different crates

# RAPS - Rust Autodesk Platform Services CLI

🌼 **RAPS** (rapeseed) is a powerful command-line tool for interacting with Autodesk Platform Services (APS). Built with Rust for speed, reliability, and security.

## Monorepo Structure

This repository contains the complete RAPS monorepo workspace with all core components:

```
raps/
├── Cargo.toml              # Workspace configuration
├── Cargo.lock              # Locked dependencies
├── .cargo/                 # Build configuration (fast linkers)
│
├── raps-kernel/            # Microkernel foundation (<3000 LOC)
├── raps-oss/               # Object Storage Service
├── raps-derivative/        # Model Derivative Service
├── raps-dm/                # Data Management Service
├── raps-ssa/               # Secure Service Accounts
├── raps-community/         # Community tier features
├── raps-pro/               # Pro tier features
└── raps/                   # CLI binary crate
    ├── src/                # CLI source code
    ├── docs/               # Documentation
    └── tests/              # Integration tests
```

## Quick Start

```bash
# Build entire workspace
cargo build --workspace

# Build CLI only
cargo build -p raps

# Run CLI
cargo run -p raps -- --help

# Check entire workspace
cargo check --workspace

# Run tests
cargo nextest run --workspace
```

## Workspace Commands

### Build Individual Crates

```bash
# Build kernel only
cargo build -p raps-kernel

# Build OSS service only
cargo build -p raps-oss

# Build CLI only
cargo build -p raps
```

### Testing

```bash
# Test entire workspace
cargo nextest run --workspace

# Test individual crate
cargo test -p raps-kernel

# Test CLI
cargo test -p raps
```

### Documentation

```bash
# Generate docs for entire workspace
cargo doc --workspace --open

# Generate docs for specific crate
cargo doc -p raps-kernel --open
```

## Architecture

RAPS follows a **microkernel architecture** with strict separation of concerns:

- **Kernel** (`raps-kernel`): Minimal trusted foundation (Auth, HTTP, Config, Storage, Types, Error, Logging)
- **Services** (`raps-oss`, `raps-derivative`, `raps-dm`, `raps-ssa`): Independent APS API clients
- **Tiers** (`raps-community`, `raps-pro`): Feature collections for different product tiers
- **CLI** (`raps`): User-facing binary that depends on kernel, services, and tiers

### Dependency Rules

- Kernel: No dependencies on services or tiers
- Services: Depend only on kernel
- Tiers: Depend only on kernel and services (not on each other)
- CLI: Depends on kernel, services, and tiers

## Product Tiers

- **Core**: Essential foundation (Auth, OSS, Derivative, DM) - Apache 2.0
- **Community**: Extended features (ACC, DA, Reality, Webhooks, MCP, TUI) - Apache 2.0
- **Pro**: Enterprise features (Analytics, Audit, Compliance, SSO) - Commercial

## Performance Targets

- Kernel incremental check: <5s
- Workspace incremental check: <30s
- CLI startup: <100ms
- Memory usage: <256MB peak

## Development

See `raps/CONTRIBUTING.md` for development guidelines and `raps/docs/` for detailed documentation.

## License

- Core & Community: Apache 2.0
- Pro: Commercial

---

**Repository**: https://github.com/dmytro-yemelianov/raps  
**Website**: https://rapscli.xyz  
**Documentation**: https://docs.rapscli.xyz

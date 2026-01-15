<!--
Sync Impact Report:
- Version change: New (1.0.0)
- List of modified principles: Initial Definition
- Added sections: Core Principles (I-V), Development Workflow, Governance
- Templates requiring updates:
  - ✅ .specify/templates/tasks-template.md (Enforce mandatory testing)
  - ⚠ .specify/templates/plan-template.md (Generic ref is fine, no change needed)
  - ⚠ .specify/templates/spec-template.md (No change needed)
- Follow-up TODOs: None
-->
# RAPS Constitution

## Core Principles

### I. Rust-Native & Modular Workspace
Functionality must be organized into distinct, self-contained `raps-*` crates (e.g., `raps-cli`, `raps-oss`) within the Rust workspace. Each crate must define clear public APIs and be independently testable. Leverage Rust's type system to enforce correctness at compile time.

### II. Automation-First Design
The CLI is a tool for both humans and machines. Every command must support:
- Standardized exit codes (0 for success, non-zero for specific failures).
- Structured output formats (JSON/YAML) via `--output` for parsing.
- A non-interactive mode (or default behavior) that prevents blocking on user input in CI/CD environments.

### III. Secure by Default
Security is paramount.
- Credentials must be stored securely (using OS keychain integrations where feasible).
- Secrets (tokens, keys) must NEVER be logged in cleartext; redacting in debug logs is mandatory.
- Support robust authentication flows (Device Code, 3-legged) suitable for various environments.

### IV. Comprehensive Observability
Users and operators need visibility.
- HTTP interactions must log method, URL, and status code consistently.
- Debug/Verbose modes must provide actionable context without exposing secrets.
- Long-running operations (uploads, processing) must provide feedback (e.g., progress bars) in interactive modes.

### V. Quality & Reliability
Code quality is non-negotiable.
- New features MUST be accompanied by unit and/or integration tests.
- All code must pass strict CI gates: `cargo fmt`, `cargo clippy` (no warnings), and `cargo test`.
- Documentation (README, help text) must be updated concurrently with code changes.

## Development Workflow

### Branching & PRs
- Direct commits to `main` are prohibited.
- All changes must follow the standard branching model (`feature/*`, `fix/*`, etc.) and be merged via Pull Request.
- PRs require passing CI checks and at least one approval.

### Release & Versioning
- Versioning follows Semantic Versioning (Major.Minor.Patch).
- Releases are automated via tags.
- SHA256 checksums must be generated for all release artifacts.

## Governance

This Constitution acts as the supreme design document for the RAPS project.
- **Amendments**: Changes to these principles require a specific "Constitution Amendment" Pull Request with clear justification.
- **Compliance**: All features, refactors, and architectural decisions must align with these principles.
- **Supremacy**: If a conflict arises between this document and code/other docs, this document prevails (or must be amended).

**Version**: 1.0.0 | **Ratified**: 2026-01-15 | **Last Amended**: 2026-01-15
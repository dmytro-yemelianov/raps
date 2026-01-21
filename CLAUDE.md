# raps Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-01-16

## Active Technologies
- Rust 1.88, Edition 2024 + rmcp 0.12 (MCP server), raps-kernel (auth), raps-oss (object storage), raps-dm (data management), raps-acc (ACC modules), raps-admin (bulk user ops) (001-mcp-project-bulk-ops)
- N/A (delegates to APS APIs via existing workspace crates) (001-mcp-project-bulk-ops)

- Rust 1.88+ (edition 2024) + clap, reqwest, tokio, serde, indicatif, directories, keyring (existing workspace dependencies) (001-account-admin-management)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test; cargo clippy

## Code Style

Rust 1.88+ (edition 2024): Follow standard conventions

## Recent Changes
- 001-mcp-project-bulk-ops: Added Rust 1.88, Edition 2024 + rmcp 0.12 (MCP server), raps-kernel (auth), raps-oss (object storage), raps-dm (data management), raps-acc (ACC modules), raps-admin (bulk user ops)

- 001-account-admin-management: Added Rust 1.88+ (edition 2024) + clap, reqwest, tokio, serde, indicatif, directories, keyring (existing workspace dependencies)

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->

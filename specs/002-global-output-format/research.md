# Research: Global Output Format Standardization

**Date**: 2026-01-15
**Feature**: Global Output Format

## Decisions

### 1. Output Formatting Architecture
**Decision**: Centralized `OutputFormatter` struct/module in `raps-cli`.
**Rationale**:
- **Separation of Concerns**: Domain crates (`raps-oss`, etc.) should return pure data (`Result<T, E>`). The CLI crate is responsible for the "View" (rendering).
- **Consistency**: A single entry point for printing ensures all commands behave identically regarding `--output`.
- **Implementation**: A generic function `print_output<T: Serialize>(data: &T, format: OutputFormat)` will handle the switch logic (JSON, YAML, Table, etc.).

**Alternatives Considered**:
- *Trait implementation on every struct*: Too much boilerplate; violates DRY.
- *Middleware*: `clap` doesn't strictly have "middleware" for output in the same way web frameworks do.

### 2. Global Flag Propagation
**Decision**: Use `clap`'s `global = true` attribute on the `--output` arg in the top-level `Cli` struct.
**Rationale**:
- `clap` automatically propagates global args to subcommands.
- We can access the global arg from the top-level match or pass the context down.

### 3. CI/CD (Non-Interactive) Detection
**Decision**: Use Rust's standard `std::io::IsTerminal` trait.
**Rationale**:
- **Standard**: Stable since Rust 1.70 (we are on 1.88).
- **Reliable**: Correctly detects TTY vs Pipes/Redirection across platforms.
- **Logic**: If `!stdout().is_terminal()`, default format becomes `JSON` (unless overridden by explicit flag).

### 4. YAML Serialization
**Decision**: Use `serde_yaml`.
**Rationale**:
- Industry standard for Rust.
- Already in workspace dependencies.
- Compatible with `serde::Serialize` which we already use for JSON.

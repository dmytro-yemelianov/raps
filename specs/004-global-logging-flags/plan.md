# Implementation Plan: Global Logging Flags

**Branch**: `004-global-logging-flags` | **Date**: 2026-01-15 | **Spec**: [specs/004-global-logging-flags/spec.md](../spec.md)
**Input**: Feature specification from `/specs/004-global-logging-flags/spec.md`

## Summary

Implement global logging flags (`--no-color`, `--quiet`, `--verbose`, `--debug`) and secret redaction. The logic already exists in `raps-kernel::logging`, but needs to be fully wired up: `redact_secrets` is unused, and commands need to respect `--quiet` consistently.

## Technical Context

**Language/Version**: Rust 1.88
**Primary Dependencies**: `raps-kernel`, `clap`, `colored`, `regex`
**Project Type**: Rust Workspace (CLI)

## Constitution Check

- **IV. Comprehensive Observability**: ✅ Explicit logging control.
- **III. Secure by Default**: ✅ Secret redaction.

## Project Structure

### Source Code

```text
raps-kernel/src/
└── logging.rs           # Ensure redact_secrets is used

raps-cli/src/
└── main.rs              # Ensure flags are passed to logging::init
```

**Structure Decision**: Keep logic in `raps-kernel`, ensure usage in `raps-cli`.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| None | | |
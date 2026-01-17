# Implementation Plan: Multi-Channel RAPS Distribution

**Branch**: `006-raps-distribution` | **Date**: 2026-01-17 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/006-raps-distribution/spec.md`

## Summary

Implement multiple distribution channels for RAPS CLI to expand user reach and simplify installation. Phase 1 delivers:
1. **Bash install script** - One-liner install for Linux/macOS users
2. **PowerShell install script** - One-liner install for Windows users
3. **PyPI CLI distribution** - `pip install raps` via maturin-built wheels
4. **GitHub Actions workflow** - Automated publishing on release

Technical approach: Shell scripts download pre-built binaries from GitHub releases. Python package uses maturin with `bindings = "bin"` mode to bundle the Rust binary into platform-specific wheels.

## Technical Context

**Language/Version**: Rust 1.88+ (existing RAPS codebase), Bash (POSIX-compatible), PowerShell 5.1+, Python 3.8+ (for maturin)
**Primary Dependencies**: maturin (Python wheel building), curl/wget (install scripts), GitHub Actions
**Storage**: N/A (no persistent storage for distribution)
**Testing**: GitHub Actions matrix testing on ubuntu-latest, macos-latest, windows-latest
**Target Platform**: Windows (x64, arm64), macOS (x64, arm64), Linux (x64, arm64)
**Project Type**: Single project with distribution tooling
**Performance Goals**: Install completes in under 60 seconds
**Constraints**: No external dependencies beyond GitHub releases, PyPI; scripts must work offline after download
**Scale/Scope**: 6 platform/arch combinations, 3 distribution channels (scripts, pip, release workflow)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Compliance | Notes |
|-----------|------------|-------|
| I. Rust-Native & Modular | ✓ PASS | Install scripts are standalone; maturin wraps existing raps binary without modifying Rust code |
| II. Automation-First | ✓ PASS | GitHub Actions automates all publishing; scripts support non-interactive execution |
| III. Secure by Default | ✓ PASS | Scripts verify checksums; download only from GitHub releases (HTTPS); no credential exposure |
| IV. Comprehensive Observability | ✓ PASS | Install scripts provide progress feedback; colorized output; clear error messages |
| V. Quality & Reliability | ✓ PASS | Install scripts tested on all platforms via CI; maturin wheels tested post-publish |

**Gate Status**: PASSED - No violations requiring justification.

## Project Structure

### Documentation (this feature)

```text
specs/006-raps-distribution/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
└── tasks.md             # Phase 2 output (created by /speckit.tasks)
```

### Source Code (repository root)

```text
# Distribution scripts (new)
install.sh               # Bash install script for Linux/macOS
install.ps1              # PowerShell install script for Windows
uninstall.sh             # Bash uninstall script
uninstall.ps1            # PowerShell uninstall script

# Python packaging (new)
python/
├── pyproject.toml       # maturin configuration
├── src/
│   └── raps/
│       ├── __init__.py  # Package init
│       └── __main__.py  # Entry point wrapper
└── README.md            # PyPI package readme

# GitHub Actions (new/modified)
.github/workflows/
├── release.yml          # Unified release workflow (modified)
├── test-install.yml     # Install script testing (new)
└── ci.yml               # Existing CI (unchanged)

# Existing structure (unchanged)
raps-cli/
raps-kernel/
raps-oss/
... (other crates)
```

**Structure Decision**: Distribution tooling added at repository root level (install scripts) and in `python/` subdirectory for maturin packaging. No changes to existing Rust workspace structure. GitHub Actions workflows extended for release automation.

## Complexity Tracking

> No violations to justify - all Constitution checks passed.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| N/A | N/A | N/A |

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [3.11.0] - 2026-01-15

### Added
- **Global Output Format Standardization**: Consistent JSON, YAML, Table, CSV output across all commands.
  - New global `--output` flag supports `json`, `yaml`, `table`, `csv`, `plain`.
  - Automatic JSON fallback in non-interactive (piped) environments.
  - Added `serde_yaml` support for YAML output.
- **Standardized Exit Codes**: robust exit codes for CI/CD scripting:
  - `0`: Success
  - `2`: Invalid arguments
  - `3`: Authentication failure
  - `4`: Resource not found
  - `5`: Remote/API error
  - `6`: Internal error
- **Global Logging Flags**: Control verbosity and color with:
  - `--no-color`: Disable ANSI colors
  - `--quiet`: Suppress info logs
  - `--verbose`: Show request summaries
  - `--debug`: Show detailed traces with **secret redaction**.
- **Non-interactive Mode**:
  - Global `--non-interactive` flag ensures no prompts are shown.
  - Fail-fast behavior for missing required arguments in non-interactive mode.
  - Global `--yes` flag for auto-confirming destructive actions.

### Changed
- All CLI commands now use a centralized output formatter for consistency.
- Logging infrastructure now automatically redacts secrets (tokens, keys) from debug output.
- `raps-kernel` error handling updated to map `anyhow::Error` chain to standardized exit codes.

## [3.4.0] - 2026-01-02
...
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2025-12-25

### Added
- SHA256 checksum generation scripts for release artifacts (PowerShell and bash)
- Checksum verification documentation for Windows, macOS, and Linux
- SBOM (Software Bill of Materials) generation scripts supporting CycloneDX format
- Comprehensive SBOM documentation and usage guide
- CODE_OF_CONDUCT.md following Contributor Covenant 2.1
- Enhanced `.gitignore` with patterns for logs, temporary files, caches, and build artifacts

### Changed
- Release process now includes checksums.txt for all artifacts
- Repository cleanup: enhanced `.gitignore` to prevent accidental commits of build artifacts

### Fixed
- Repository artifacts properly excluded from version control

## [0.5.0] - 2025-12-25

### Added
- `--timeout` CLI flag for HTTP request timeouts (default: 120s)
- `--concurrency` CLI flag for bulk operations (default: 5)
- Parallel processing with semaphore-based concurrency control in batch operations
- Config precedence documentation (CLI flags > env vars > profile > defaults)
- OS keychain integration for secure token storage (opt-in via `RAPS_USE_KEYCHAIN` env var)
- TokenStorage abstraction supporting both file and keychain backends

### Changed
- All API clients now accept `HttpClientConfig` for consistent timeout configuration
- Batch processing demo now uses parallel processing with configurable concurrency limits
- Config precedence updated to include CLI flags as highest priority

### Fixed
- Batch processing now properly respects concurrency limits
- All API clients use shared HTTP configuration from CLI flags

## [0.4.0] - 2025-12-24

### Added
- Profile management system (`raps config profile create/list/use/delete/current`)
- Config get/set commands with profile support
- Device code authentication flow (`raps auth login --device`) for headless/server environments
- Token-based login (`raps auth login --token`) for CI/CD scenarios with security warnings
- HTTP retry logic with exponential backoff and jitter for 429 and 5xx errors
- Configurable HTTP client timeouts (default: 120s request, 30s connect)
- YAML output format support (`--output yaml`)
- Global logging flags (`--no-color`, `--quiet`, `--verbose`, `--debug`)
- Secret redaction in debug output
- Global non-interactive mode (`--non-interactive` and `--yes` flags)
- Standardized exit codes (0=success, 2=invalid args, 3=auth failure, 4=not found, 5=remote error, 6=internal error)
- Token expiry information in `raps auth status`
- CHANGELOG.md following Keep a Changelog format
- GitHub issue templates (bug report, feature request)
- Checksum generation scripts for release verification

### Changed
- Config loading now supports profile precedence: environment variables > active profile > defaults
- All API clients now use configured HTTP timeouts
- Improved error messages with better context for exit code detection

### Fixed
- Commands now properly respect non-interactive mode and fail with clear errors when required parameters are missing
- All clippy warnings resolved

## [0.3.0] - 2024-XX-XX

### Added
- Initial release with core APS CLI functionality
- Authentication (2-legged and 3-legged OAuth)
- Object Storage Service (OSS) bucket and object management
- Model Derivative file translation
- Data Management (hubs, projects, folders, items)
- Webhooks management
- Design Automation support
- ACC/BIM 360 Issues management
- Reality Capture photogrammetry support
- Interactive command prompts
- JSON and CSV output formats
- Shell completions (bash, zsh, fish, PowerShell, elvish)

[Unreleased]: https://github.com/dmytro-yemelianov/raps/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v0.6.0
[0.5.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v0.5.0
[0.4.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v0.4.0
[0.3.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v0.3.0


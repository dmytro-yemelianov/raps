# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

### Changed
- Config loading now supports profile precedence: environment variables > active profile > defaults
- All API clients now use configured HTTP timeouts
- Improved error messages with better context for exit code detection

### Fixed
- Commands now properly respect non-interactive mode and fail with clear errors when required parameters are missing

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

[Unreleased]: https://github.com/dmytro-yemelianov/raps/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v0.3.0


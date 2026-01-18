# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **Install Scripts**: Fixed artifact naming to match cargo-dist convention.
  - Updated `install.sh` and `install.ps1` to use cargo-dist target triples (e.g., `x86_64-unknown-linux-gnu`).
  - Changed archive format from `.tar.gz` to `.tar.xz` for Linux/macOS.
  - Updated checksum verification to use individual `.sha256` files.

## [4.2.0] - 2026-01-18

### Added
- **Multi-Channel Distribution**: New installation methods for easier onboarding.
  - **Shell Install Script** (`install.sh`): One-liner install for Linux/macOS with automatic PATH configuration.
    - Supports bash, zsh, and fish shells
    - SHA256 checksum verification
    - Version selection via `RAPS_VERSION` environment variable
    - Uninstall support via `--uninstall` flag
  - **PowerShell Install Script** (`install.ps1`): One-liner install for Windows.
    - Automatic User PATH modification
    - Checksum verification using Get-FileHash
    - Parameters: `-Version`, `-InstallDir`, `-NoPathUpdate`, `-Uninstall`
  - **PyPI Distribution**: Install via `pip install raps`.
    - Platform wheels for Linux, macOS, and Windows (x64 and arm64)
    - Uses maturin with `bindings = "bin"` for binary bundling
    - Python 3.8+ support
- **Enhanced Release Automation**: GitHub Actions workflow extended for multi-channel publishing.
  - Automatic PyPI publishing using OIDC trusted publishing
  - Post-release install script testing on all platforms

## [4.1.0] - 2026-01-17

### Added
- **MCP Server Expansion**: Aligned MCP server with CLI v4.0 functionality (14 → 35 tools).
  - Admin Bulk Operations: `admin_project_list`, `admin_user_add`, `admin_user_remove`, `admin_user_update_role`, `admin_operation_list`, `admin_operation_status`.
  - Folder/Item Management: `folder_list`, `folder_create`, `item_info`, `item_versions`.
  - Issues: `issue_list`, `issue_get`, `issue_create`, `issue_update`.
  - RFIs: `rfi_list`, `rfi_get`.
  - ACC Extended: `acc_assets_list`, `acc_submittals_list`, `acc_checklists_list`.

### Changed
- MCP server instructions updated to reflect v4.0 capabilities.
- ACC/Admin clients created on-demand in MCP server (not cached) due to Clone trait requirements.

## [4.0.0] - 2026-01-16

### Added
- **Account Admin Bulk Management Tool**: New `raps admin` command suite for bulk user management across ACC/BIM 360 accounts.
  - `raps admin user add`: Bulk add users to multiple projects with role assignment.
  - `raps admin user remove`: Bulk remove users from projects.
  - `raps admin user update-role`: Bulk update user roles across projects.
  - `raps admin folder rights`: Bulk update folder permissions (Project Files, Plans, or custom folders).
  - `raps admin project list`: List projects with filtering by name, status, and platform.
  - `raps admin operation status`: View operation progress and results.
  - `raps admin operation resume`: Resume interrupted operations.
  - `raps admin operation cancel`: Cancel in-progress operations.
  - `raps admin operation list`: List all operations with status filtering.
- **New `raps-admin` Crate**: Orchestration layer for bulk operations with:
  - Resumable state persistence using JSON files.
  - Semaphore-based concurrency control (max 50 parallel requests).
  - Exponential backoff retry logic for rate limit handling (429 errors).
  - Progress tracking with indicatif progress bars.
  - Dry-run mode for operation preview.
  - Project filtering by regex pattern.
- **New `FolderPermissionsClient`**: ACC Folder Permissions API client in `raps-acc` crate.
- **Comprehensive Integration Tests**: 61 tests covering all bulk operations and state management.

### Changed
- Major version bump due to significant new feature addition.

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
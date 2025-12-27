# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [4.0.0] - 2025-12-27

### Changed
- Hardened MCP server tool invocation with strict argument validation, safer defaults, and clearer errors to prevent invalid API calls.
- Cached APS client instances to reduce lock contention and repeated client construction during MCP sessions.
- Added sensible limit clamping and output format validation to MCP tools to mitigate abusive requests and clarify supported conversions.

## [3.0.0] - 2025-12-26

### Added

#### MCP Server (Model Context Protocol)
- **`raps serve`** - Start MCP server for AI assistant integration
- **14 MCP Tools** for direct APS API access from AI assistants:
  - `auth_test` - Test 2-legged OAuth credentials
  - `auth_status` - Check authentication status (2-legged and 3-legged)
  - `bucket_list` - List OSS buckets with optional region filter
  - `bucket_create` - Create new OSS bucket with retention policy
  - `bucket_get` - Get bucket details
  - `bucket_delete` - Delete empty bucket
  - `object_list` - List objects in bucket
  - `object_delete` - Delete object from bucket
  - `object_signed_url` - Generate pre-signed S3 download URL
  - `object_urn` - Get Base64-encoded URN for translation
  - `translate_start` - Start CAD file translation (svf2, obj, stl, step, iges, ifc)
  - `translate_status` - Check translation job status
  - `hub_list` - List accessible BIM 360/ACC hubs
  - `project_list` - List projects in a hub

#### Dependencies
- Added `rmcp` v0.12 - Official Rust SDK for Model Context Protocol
- Added `schemars` v0.8 - JSON Schema generation for tool definitions
- Added `tracing` and `tracing-subscriber` for MCP server logging

### Changed
- **Major Version Bump**: v3.0.0 introduces MCP server capability, a new interface paradigm for AI-assisted APS operations

### Usage

Start the MCP server:
```bash
raps serve
```

Configure in Claude Desktop (`claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "raps": {
      "command": "raps",
      "args": ["serve"],
      "env": {
        "APS_CLIENT_ID": "your_client_id",
        "APS_CLIENT_SECRET": "your_client_secret"
      }
    }
  }
}
```

Configure in Cursor (`.cursor/mcp.json`):
```json
{
  "mcpServers": {
    "raps": {
      "command": "raps",
      "args": ["serve"]
    }
  }
}
```

---

## [2.1.0] - 2025-12-26

### Added
- 🌼 **Rapeseed Branding**: RAPS now stands for "Rust Autodesk Platform Services" with the blossom (🌼) as the brand symbol.
- **Official Website**: New documentation and blog at [rapscli.xyz](https://rapscli.xyz).
- **Blossom Favicon**: Updated website favicon with golden blossom design.

### Changed
- **CLI Branding**: Updated `--version` and `--help` output to show "🌼 RAPS (rapeseed)" branding.
- **Documentation Links**: All documentation now points to [rapscli.xyz](https://rapscli.xyz).
- **Cargo.toml**: Updated homepage, documentation URLs, and description with rapeseed branding.
- **Package Managers**: Updated Homebrew and Scoop formulas with new branding and homepage.

---

## [2.0.0] - 2025-12-25

### Added
- **APS Feature Coverage**: New dedicated documentation page comparing RAPS CLI capabilities against full APS API spectrum.
- **Enhanced Documentation**: Created comprehensive guides for ACC Modules, RFIs, Plugin Management, and Synthetic Data Generation.
- **License Migration**: Transitioned from MIT to Apache 2.0 for better attribution preservation and patent protection.
- **Architecture Visualization**: New Mermaid diagrams for command architecture and authentication flows.
- **Practical Examples**: Expanded use cases for ACC Assets, RFIs, and developer tools.

### Changed
- **Repository Reorganization**: Significant cleanup and restructuring of the repository for better maintainability (moved scripts, consolidated assets, archived historical logs).
- **Workflow Migration**: Documentation deployment migrated from Jekyll to MkDocs in GitHub Actions.
- **Reference Overhaul**: Updated `README.md` and `index.md` with complete command reference and feature discovery paths.

### Fixed
- Outdated usage examples in `demo` command documentation.
- Broken environment variable examples in installation guides.

---

## [1.0.0] - 2025-12-25

### Added

#### ACC RFIs (Requests for Information)
- `raps rfi list` - List RFIs in a project
- `raps rfi get` - Get RFI details
- `raps rfi create` - Create new RFI with title, question, priority, due date
- `raps rfi update` - Update RFI status, answer, and other fields

#### ACC Assets (Full CRUD)
- `raps acc asset get` - Get specific asset details
- `raps acc asset create` - Create new asset with description, barcode, category
- `raps acc asset update` - Update asset status, description, barcode

#### ACC Submittals (Full CRUD)
- `raps acc submittal get` - Get specific submittal details
- `raps acc submittal create` - Create new submittal with title, spec section, due date
- `raps acc submittal update` - Update submittal status, title, due date

#### ACC Checklists (Full CRUD)
- `raps acc checklist get` - Get specific checklist details
- `raps acc checklist create` - Create new checklist from template
- `raps acc checklist update` - Update checklist status, location, due date

#### Plugin System
- `raps plugin list` - List discovered and configured plugins
- `raps plugin enable <name>` - Enable a plugin
- `raps plugin disable <name>` - Disable a plugin
- `raps plugin alias list` - List command aliases
- `raps plugin alias add` - Add command alias
- `raps plugin alias remove` - Remove command alias
- Plugin discovery for `raps-<name>` executables in PATH
- Pre/post command hooks support
- Command aliases for frequently used patterns

#### Documentation
- Comprehensive plugin system documentation (`docs/plugins.md`)
- Known limitations documentation (`docs/limitations.md`)
- Stability and backward compatibility policy (`docs/STABILITY.md`)
- Performance tuning documentation (timeout, concurrency, pagination)
- OS keychain integration documentation

### Changed
- **BREAKING**: First stable release - backward compatibility guaranteed from this version
- Version bumped to 1.0.0 (stable release)
- All ACC modules now have full CRUD support (previously list-only)
- Enhanced configuration documentation with pagination behavior

### Fixed
- Plugin system now fully functional (was placeholder in 0.7.0)

---

## [0.7.0] - 2025-12-25

### Added

#### Object Storage Service (OSS)
- Multipart chunked uploads for files > 5MB with automatic chunking
- Resumable uploads with `--resume` flag to continue interrupted uploads
- Batch file uploads with `--batch` and `--parallel` flags for parallel processing
- Upload state persistence for resume capability

#### Model Derivative
- Download translated derivatives with `raps translate download`
- Translation presets with `raps translate preset` (list/create/delete/use)
- Default presets: viewer (svf2), 3d-print (stl), cad-exchange (step), bim (ifc)

#### Design Automation
- Activity creation with `raps da activity create`
- Work item submission with `raps da workitem run`
- Work item result retrieval with `raps da workitem get`
- Report download capability

#### ACC Issues
- Issue comments management (`raps issue comment list/add/delete`)
- Issue attachments (`raps issue attachment list/upload/download`)
- Issue state transitions (`raps issue transition`)

#### ACC Data Management
- Bind OSS objects to ACC project folders with `raps item bind`
- Create linked items from external uploads

#### Webhooks
- Test webhook endpoints with `raps webhook test`
- Sample payload generation for endpoint validation
- HMAC signature support for testing

#### Configuration & Automation
- Profile import/export (`raps config profile import/export`)
- Token inspection with `raps auth inspect-token` (scope, expiry, warnings)
- Pipeline execution from YAML/JSON files with `raps pipeline run`
- Pipeline validation with `raps pipeline validate`
- Sample pipeline generation with `raps pipeline sample`
- Variable substitution and conditional step execution
- Continue-on-error support for robust automation

#### Documentation
- Feature overview page with Mermaid diagrams
- Command architecture visualization
- Authentication flow diagrams
- Data flow sequence diagrams
- Version history timeline
- Complete feature matrix tables

#### Error Handling
- Enhanced error interpretation with human-readable explanations
- Contextual suggestions for common API errors
- Improved exit codes for CI/CD integration

#### Architecture (Placeholders)
- Plugin/extension system architecture
- ACC modules expansion framework (Assets, Submittals, Checklists)

### Changed
- OssClient now implements Clone for parallel operations
- ProfilesData now implements Clone for export operations
- Improved multipart upload threshold (5MB)

### Fixed
- All unit tests pass (73 tests)
- Code compiles without warnings

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

[Unreleased]: https://github.com/dmytro-yemelianov/raps/compare/v4.0.0...HEAD
[4.0.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v4.0.0
[3.0.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v3.0.0
[2.1.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v2.1.0
[2.0.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v2.0.0
[1.0.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v1.0.0
[0.7.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v0.7.0
[0.6.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v0.6.0
[0.5.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v0.5.0
[0.4.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v0.4.0
[0.3.0]: https://github.com/dmytro-yemelianov/raps/releases/tag/v0.3.0


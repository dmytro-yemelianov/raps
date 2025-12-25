# RAPS

<div align="center">
  <img src="logo/output/raps-logo.webp" alt="RAPS Logo" width="200"/>

  <h3>Rust APS CLI</h3>
  <p>A comprehensive command-line interface for Autodesk Platform Services (APS)</p>

  [![Crates.io](https://img.shields.io/crates/v/raps.svg)](https://crates.io/crates/raps)
  [![Downloads](https://img.shields.io/crates/d/raps.svg)](https://crates.io/crates/raps)
  [![Documentation](https://img.shields.io/badge/docs-latest-blue.svg)](https://dmytro-yemelianov.github.io/raps/)
  [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
  [![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
  [![Build Status](https://github.com/dmytro-yemelianov/raps/workflows/CI/badge.svg)](https://github.com/dmytro-yemelianov/raps/actions)

</div>

**R**ust **APS** CLI - A comprehensive command-line interface for Autodesk Platform Services (APS), written in Rust.

## Features

### Authentication
- **2-legged OAuth** (Client Credentials) for server-to-server operations
- **3-legged OAuth** (Authorization Code) with browser login for user data access
- **Device-code authentication** (`--device`) for headless/server environments
- **Token-based login** (`--token`) for CI/CD scenarios
- **Token inspection** (`auth inspect-token`) - view scopes, expiry, and warnings
- Secure token storage with automatic refresh
- User profile information with `auth whoami`

### Object Storage Service (OSS)
- Create, list, and delete buckets (with multi-region support: US & EMEA)
- Get detailed bucket information with `bucket info`
- Upload, download, list, and delete objects
- **Resumable multipart uploads** for large files (auto-chunking for files > 5MB)
- **Batch uploads** with parallel processing (`--batch`, `--parallel`)
- **Signed S3 URLs** for direct download bypassing OSS servers
- Progress bars for file transfers

### Model Derivative
- Translate CAD files to various formats (SVF2, OBJ, STL, STEP, etc.)
- Check translation status with optional polling
- View manifest and available derivatives
- **Download derivatives** (`translate download`) - export translated models
- **Translation presets** (`translate preset`) - save and reuse common configurations

### Data Management (BIM 360/ACC)
- Browse hubs, projects, folders, and items
- Create folders
- View item versions
- **Bind OSS objects to ACC folders** (`item bind`) - link external uploads
- Requires 3-legged authentication

### Webhooks
- Create, list, and delete webhook subscriptions
- Support for data management and model derivative events
- **Test webhook endpoints** (`webhook test`) - validate with sample payloads

### Design Automation
- List available engines (AutoCAD, Revit, Inventor, 3ds Max)
- Manage app bundles and activities
- **Create activities** (`da activity create`)
- **Submit work items** (`da workitem run`) with input/output URLs
- **Get work item results** (`da workitem get`) - download reports
- Monitor work item status

### ACC Issues (Construction Cloud)
- List, create, and update issues
- View issue types (categories) and subtypes
- Filter by status
- **Issue comments** (`issue comment`) - list, add, delete
- **Issue attachments** (`issue attachment`) - upload, download
- **State transitions** (`issue transition`) - change issue status

### ACC RFIs (Requests for Information) (v1.0.0+)
- **List RFIs** (`rfi list`) - view all RFIs in a project
- **Get RFI details** (`rfi get`) - view full RFI information
- **Create RFIs** (`rfi create`) - submit new requests for information
- **Update RFIs** (`rfi update`) - answer RFIs, change status

### ACC Assets (v1.0.0+)
- **List assets** (`acc asset list`) - view project assets
- **CRUD operations** - get, create, update assets

### ACC Submittals (v1.0.0+)
- **List submittals** (`acc submittal list`) - view project submittals
- **CRUD operations** - get, create, update submittals

### ACC Checklists (v1.0.0+)
- **List checklists** (`acc checklist list`) - view project checklists
- **List templates** (`acc checklist templates`) - view available templates
- **CRUD operations** - get, create, update checklists

### Reality Capture
- Create photoscenes for photogrammetry
- Upload photos and start processing
- Monitor progress and download results (OBJ, FBX, RCS, etc.)

### Pipeline Automation
- **Execute pipelines** from YAML/JSON files (`pipeline run`)
- **Variable substitution** and conditional step execution
- **Dry-run mode** for validation
- **Continue-on-error** for robust automation
- **Sample generation** (`pipeline sample`)

### Configuration & Profiles
- **Profile management** - create, switch, delete configurations
- **Profile import/export** - backup and share configurations
- Config precedence: CLI flags > env vars > profile > defaults

### Plugin System (v1.0.0+)
- **External plugins** - extend RAPS with `raps-<name>` executables
- **Command hooks** - run pre/post command scripts
- **Command aliases** - create shortcuts for frequent operations
- **Plugin management** (`plugin list/enable/disable`)

### Development Tools (v1.0.0+)
- **Test Data Generation** (`generate`) - create synthetic OBJ, IFC, and other files for testing
- **Demo Scenarios** (`demo`) - run end-to-end scenarios like bucket lifecycle or model pipeline

## Installation

### Prerequisites

- APS account with application credentials from [APS Developer Portal](https://aps.autodesk.com/myapps)

### Install from crates.io

```bash
cargo install raps
```

### Install from Pre-built Binaries

Download the latest release for your platform from the [Releases](https://github.com/dmytro-yemelianov/raps/releases) page:

| Platform | Architecture | File |
|----------|--------------|------|
| Windows | x64 | `raps-windows-x64.zip` |
| macOS | Intel | `raps-macos-x64.tar.gz` |
| macOS | Apple Silicon | `raps-macos-arm64.tar.gz` |
| Linux | x64 | `raps-linux-x64.tar.gz` |
| Linux | ARM64 | `raps-linux-arm64.tar.gz` |

Extract and add to your PATH:

**Windows (PowerShell):**
```powershell
# Extract to a directory in your PATH
Expand-Archive raps-windows-x64.zip -DestinationPath "$env:USERPROFILE\bin"
# Add to PATH (if not already)
$env:PATH += ";$env:USERPROFILE\bin"
```

**macOS/Linux:**
```bash
# Extract
tar -xzf raps-*.tar.gz

# Move to PATH
sudo mv raps /usr/local/bin/
chmod +x /usr/local/bin/raps
```

### Build from Source

```bash
# Requires Rust 1.70 or later (https://rustup.rs/)
cd raps
cargo build --release
```

## Shell Completions

RAPS supports auto-completion for bash, zsh, fish, PowerShell, and elvish.

### PowerShell

```powershell
# Add to your PowerShell profile ($PROFILE)
raps completions powershell | Out-String | Invoke-Expression

# Or save to a file and source it
raps completions powershell > "$env:USERPROFILE\Documents\PowerShell\raps.ps1"
# Then add to $PROFILE: . "$env:USERPROFILE\Documents\PowerShell\raps.ps1"
```

### Bash

```bash
# Add to ~/.bashrc
eval "$(raps completions bash)"

# Or save to completions directory
raps completions bash > ~/.local/share/bash-completion/completions/raps
```

### Zsh

```zsh
# Add to ~/.zshrc (before compinit)
eval "$(raps completions zsh)"

# Or save to fpath directory
raps completions zsh > ~/.zfunc/_raps
# Add to ~/.zshrc: fpath=(~/.zfunc $fpath)
```

### Fish

```fish
# Save to completions directory
raps completions fish > ~/.config/fish/completions/raps.fish
```

### Elvish

```elvish
# Add to ~/.elvish/rc.elv
eval (raps completions elvish | slurp)
```

## Configuration

### Profile Management (v0.4.0+)

Manage multiple configurations for different environments:

```bash
# Create a profile
raps config profile create production

# Set profile values
raps config set client_id "your_client_id"
raps config set client_secret "your_client_secret"

# Switch between profiles
raps config profile use production

# List all profiles
raps config profile list

# Show current profile
raps config profile current
```

**Config Precedence:** CLI flags > Environment variables > Active profile > Defaults

### Environment Variables

```powershell
# Required
$env:APS_CLIENT_ID = "your_client_id"
$env:APS_CLIENT_SECRET = "your_client_secret"

# Optional
$env:APS_CALLBACK_URL = "http://localhost:8080/callback"  # For 3-legged OAuth
$env:APS_DA_NICKNAME = "your_nickname"  # For Design Automation
```

### Using .env File

Create a `.env` file in your working directory:

```env
APS_CLIENT_ID=your_client_id
APS_CLIENT_SECRET=your_client_secret
APS_CALLBACK_URL=http://localhost:8080/callback
```

## Usage

### Authentication

```bash
# Test 2-legged authentication
raps auth test

# Login with 3-legged OAuth (opens browser)
raps auth login

# Login with device code (headless/server environments)
raps auth login --device

# Login with token (CI/CD scenarios)
raps auth login --token <access_token> --refresh-token <refresh_token>

# Check authentication status (shows token expiry)
raps auth status

# Show logged-in user profile
raps auth whoami

# Logout
raps auth logout
```

### Buckets & Objects

```bash
# Create a bucket
raps bucket create

# List buckets (from all regions)
raps bucket list

# Get bucket details
raps bucket info my-bucket

# Upload a file
raps object upload my-bucket model.dwg

# Download a file
raps object download my-bucket model.dwg

# Get signed S3 download URL (direct download, expires in 2-60 minutes)
raps object signed-url my-bucket model.dwg --minutes 10
```

### Translation

```bash
# Start translation
raps translate start <urn> --format svf2

# Check status
raps translate status <urn> --wait

# View manifest
raps translate manifest <urn>
```

### Output Formats

RAPS supports multiple output formats for CI/CD integration:

```bash
# JSON output (machine-readable)
raps bucket list --output json

# YAML output
raps bucket list --output yaml

# CSV output
raps bucket list --output csv

# Table output (default, human-readable)
raps bucket list --output table

# Plain text
raps bucket list --output plain
```

### Global Flags

```bash
# Disable colors
raps bucket list --no-color

# Quiet mode (only output data)
raps bucket list --quiet

# Set HTTP request timeout (seconds, default: 120)
raps bucket list --timeout 60

# Set maximum concurrent operations for bulk commands (default: 5)
raps demo batch-processing --concurrency 10

# Verbose mode (show request summaries)
raps bucket list --verbose

# Debug mode (full trace with secret redaction)
raps bucket list --debug

# Non-interactive mode (fail on prompts)
raps bucket create --non-interactive --key my-bucket

# Auto-confirm prompts
raps bucket delete my-bucket --yes
```

### Exit Codes

RAPS uses standardized exit codes for scripting:

- `0` - Success
- `2` - Invalid arguments
- `3` - Authentication failure
- `4` - Not found
- `5` - Remote/API error
- `6` - Internal error

See [Exit Codes Documentation](docs/cli/exit-codes.md) for details.

### Data Management (requires login)

```bash
# List hubs
raps hub list

# List projects in a hub
raps project list <hub-id>

# List folder contents
raps folder list <project-id> <folder-id>

# View item versions
raps item versions <project-id> <item-id>
```

### Webhooks

```bash
# List all webhooks
raps webhook list

# Create a webhook
raps webhook create --url https://example.com/hook --event dm.version.added

# List available events
raps webhook events
```

### Design Automation

```bash
# List available engines
raps da engines

# List app bundles
raps da appbundles

# List activities
raps da activities

# Check work item status
raps da status <workitem-id> --wait
```

### Issues (ACC/BIM 360, requires login)

```bash
# List issues in a project
raps issue list <project-id>

# Create an issue
raps issue create <project-id> --title "My Issue"

# List issue types (categories)
raps issue types <project-id>
```

**Note:** The project-id should NOT include the "b." prefix used by the Data Management API.

### Reality Capture

```bash
# Create a photoscene
raps reality create --name "My Scene" --scene-type object --format obj

# Upload photos
raps reality upload <photoscene-id> photo1.jpg photo2.jpg photo3.jpg

# Start processing
raps reality process <photoscene-id>

# Check status
raps reality status <photoscene-id> --wait

# Get download link
raps reality result <photoscene-id>
```

## Command Reference

| Command | Description |
|---------|-------------|
| `auth` | Authentication (test, login, logout, status, whoami, inspect-token) |
| `bucket` | OSS bucket operations (create, list, info, delete) |
| `object` | OSS object operations (upload, download, list, delete, signed-url) |
| `translate` | Model Derivative (start, status, manifest, download, preset) |
| `hub` | List/view hubs |
| `project` | List/view projects |
| `folder` | Folder operations |
| `item` | Item operations (versions, bind) |
| `webhook` | Webhook subscriptions (create, list, delete, test) |
| `da` | Design Automation (engines, appbundles, activities, workitem) |
| `issue` | ACC/BIM 360 issues (list, create, update, comment, attachment, transition) |
| `acc` | ACC extended modules (assets, submittals, checklists) |
| `rfi` | ACC RFIs (list, get, create, update) |
| `reality` | Reality Capture photogrammetry |
| `pipeline` | Pipeline automation (run, validate, sample) |
| `plugin` | Plugin management (list, install, remove) |
| `generate` | Generate (synthetic files for testing) |
| `demo` | Run demo scenarios |
| `config` | Configuration and profile management (import, export) |
| `completions` | Generate shell completions (bash, zsh, fish, powershell, elvish) |

## API Coverage

This CLI covers the following APS APIs (validated against OpenAPI specs):

- **Authentication API v2** - OAuth 2.0 flows, user profile
- **OSS API v2** - Buckets, objects, signed S3 URLs
- **Model Derivative API v2** - Translation jobs, manifests
- **Data Management API v1** - Hubs, projects, folders, items
- **Webhooks API v1** - Event subscriptions
- **Design Automation API v3** - Engines, activities, work items
- **Construction Issues API v1** - Issues, issue types
- **ACC RFIs API v1** - Requests for Information
- **ACC Assets API v1** - Assets, categories, status
- **ACC Submittals API v1** - Submittals, spec sections
- **ACC Checklists API v1** - Checklists, templates
- **Reality Capture API v1** - Photogrammetry processing

## Release Verification

All releases include SHA256 checksums for verification. See [Checksums Documentation](docs/cli/checksums.md) for instructions on verifying downloads.

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details on:

- Development setup
- Code style guidelines
- Pull request process
- Branch protection policy

**Important**: The `main` branch is protected. All changes must be made through Pull Requests. See [Branch Protection Setup](docs/BRANCH_PROTECTION.md) for details.

## License

MIT License

## Documentation

- **[Full Documentation](https://dmytro-yemelianov.github.io/raps/)** - Complete user guide
- **[Feature Overview](https://dmytro-yemelianov.github.io/raps/features/)** - Visual diagrams and feature matrix
- **[Pipeline Guide](https://dmytro-yemelianov.github.io/raps/commands/pipeline/)** - Automation workflows
- **[Exit Codes](https://dmytro-yemelianov.github.io/raps/cli/exit-codes/)** - CI/CD integration
- **[Changelog](CHANGELOG.md)** - Version history

## Resources

- [APS Developer Portal](https://aps.autodesk.com)
- [APS Documentation](https://aps.autodesk.com/developer/documentation)
- [APS OpenAPI Specifications](https://github.com/autodesk-platform-services/aps-sdk-openapi)
- [Data Management API](https://aps.autodesk.com/en/docs/data/v2/)
- [Model Derivative API](https://aps.autodesk.com/en/docs/model-derivative/v2/)
- [Design Automation API](https://aps.autodesk.com/en/docs/design-automation/v3/)
- [Webhooks API](https://aps.autodesk.com/en/docs/webhooks/v1/)
- [Reality Capture API](https://aps.autodesk.com/en/docs/reality-capture/v1/)
- [ACC Issues API](https://aps.autodesk.com/en/docs/acc/v1/reference/http/issues-issues-GET/)

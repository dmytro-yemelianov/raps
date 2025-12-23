# RAPS

**R**ust **APS** CLI - A comprehensive command-line interface for Autodesk Platform Services (APS), written in Rust.

## Features

### Authentication
- **2-legged OAuth** (Client Credentials) for server-to-server operations
- **3-legged OAuth** (Authorization Code) with browser login for user data access
- Secure token storage with automatic refresh
- User profile information with `auth whoami`

### Object Storage Service (OSS)
- Create, list, and delete buckets (with multi-region support: US & EMEA)
- Get detailed bucket information with `bucket info`
- Upload, download, list, and delete objects
- **Signed S3 URLs** for direct download bypassing OSS servers
- Progress bars for file transfers

### Model Derivative
- Translate CAD files to various formats (SVF2, OBJ, STL, STEP, etc.)
- Check translation status with optional polling
- View manifest and available derivatives

### Data Management (BIM 360/ACC)
- Browse hubs, projects, folders, and items
- Create folders
- View item versions
- Requires 3-legged authentication

### Webhooks
- Create, list, and delete webhook subscriptions
- Support for data management and model derivative events

### Design Automation
- List available engines (AutoCAD, Revit, Inventor, 3ds Max)
- Manage app bundles and activities
- Submit and monitor work items

### ACC Issues (Construction Cloud)
- List, create, and update issues
- View issue types (categories) and subtypes
- Filter by status
- Uses the Construction Issues API v1

### Reality Capture
- Create photoscenes for photogrammetry
- Upload photos and start processing
- Monitor progress and download results (OBJ, FBX, RCS, etc.)

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

# Check authentication status
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
| `auth` | Authentication management (test, login, logout, status, whoami) |
| `bucket` | OSS bucket operations (create, list, info, delete) |
| `object` | OSS object operations (upload, download, list, delete, signed-url) |
| `translate` | Model Derivative translation |
| `hub` | List/view hubs |
| `project` | List/view projects |
| `folder` | Folder operations |
| `item` | Item/file operations |
| `webhook` | Webhook subscriptions |
| `da` | Design Automation |
| `issue` | ACC/BIM 360 issues |
| `reality` | Reality Capture photogrammetry |
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
- **Reality Capture API v1** - Photogrammetry processing

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) for details on:

- Development setup
- Code style guidelines
- Pull request process
- Branch protection policy

**Important**: The `main` branch is protected. All changes must be made through Pull Requests. See [Branch Protection Setup](docs/BRANCH_PROTECTION.md) for details.

## License

MIT License

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

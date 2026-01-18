# RAPS - Rust CLI for Autodesk Platform Services

[![PyPI version](https://badge.fury.io/py/raps.svg)](https://badge.fury.io/py/raps)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

A fast, modern command-line interface for Autodesk Platform Services (APS), built with Rust.

## Installation

```bash
pip install raps
```

## Quick Start

```bash
# Check installation
raps --version

# Get help
raps --help

# Test authentication (requires APS credentials)
raps auth test

# List buckets
raps bucket list
```

## Configuration

Set your APS credentials as environment variables:

```bash
export APS_CLIENT_ID="your-client-id"
export APS_CLIENT_SECRET="your-client-secret"
```

Or use a `.env` file in your project directory.

## Features

- **Object Storage Service (OSS)**: Manage buckets and objects
- **Model Derivative**: Translate and extract model data
- **Data Management**: Work with hubs, projects, and folders
- **Design Automation**: Run Revit, AutoCAD, and Inventor engines
- **Authentication**: Support for 2-legged, 3-legged, and device code flows
- **MCP Server**: AI assistant integration via Model Context Protocol

## Documentation

For full documentation, visit [rapscli.xyz](https://rapscli.xyz).

## Alternative Installation Methods

### Shell Script (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash
```

### PowerShell (Windows)

```powershell
irm https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.ps1 | iex
```

### Homebrew (macOS)

```bash
brew install dmytro-yemelianov/tap/raps
```

### Scoop (Windows)

```powershell
scoop bucket add raps https://github.com/dmytro-yemelianov/scoop-bucket
scoop install raps
```

## License

Apache 2.0 - See [LICENSE](https://github.com/dmytro-yemelianov/raps/blob/main/LICENSE) for details.

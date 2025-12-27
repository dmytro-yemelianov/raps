# RAPS CLI Documentation

<div align="center">
  <img src="logo/output/raps-logo.webp" 
       srcset="logo/output/raps-logo-256.webp 256w, logo/output/raps-logo-512.webp 512w, logo/output/raps-logo.webp 512w"
       sizes="(max-width: 512px) 256px, 512px"
       alt="RAPS Logo" 
       width="200"
       style="max-width: 200px; height: auto;"/>
  <h1>RAPS CLI Documentation</h1>
  <p><strong>R</strong>ust <strong>APS</strong> CLI - A comprehensive command-line interface for Autodesk Platform Services</p>
</div>

## Welcome

RAPS (Rust APS CLI) is a powerful command-line tool for interacting with Autodesk Platform Services (APS). It provides easy access to all major APS APIs including authentication, object storage, model translation, data management, webhooks, design automation, and more.

## Quick Start

1. **[Install RAPS](installation.md)** - Get started with installation instructions
2. **[Configure Credentials](configuration.md)** - Set up your APS credentials
3. **[Authenticate](commands/auth.md)** - Login and test authentication
4. **[Explore Commands](commands/buckets.md)** - Start using RAPS commands

## Features

### 🔐 Authentication
- 2-legged OAuth (Client Credentials) for server-to-server operations
- 3-legged OAuth (Authorization Code) with browser login
- Secure token storage with automatic refresh

### 📦 Object Storage Service (OSS)
- Create, list, and delete buckets (multi-region support)
- Upload, download, and manage objects
- **Resumable multipart uploads** for large files
- **Batch uploads** with parallel processing
- Generate signed S3 URLs for direct downloads

### 🔄 Model Derivative
- Translate CAD files to various formats (SVF2, OBJ, STL, STEP, etc.)
- Check translation status with polling
- View manifests and available derivatives
- **Download derivatives** (OBJ, STL, STEP, etc.)
- **Translation presets** for common workflows

### 🏗️ Data Management
- Browse hubs, projects, folders, and items
- Create folders and manage versions
- Full BIM 360/ACC integration

### 🔔 Webhooks
- Create, list, and delete webhook subscriptions
- Support for data management and model derivative events
- **Test webhook endpoints** with sample payloads

### 📋 Pipelines & Automation
- **Execute batch operations** from YAML/JSON files
- Variable substitution and conditional steps
- Continue-on-error support for robust automation
- Dry-run mode for validation

### 🔍 Token Inspection
- **Inspect token scopes and expiry**
- Validate tokens before script execution
- CI/CD-friendly expiry warnings

### ⚙️ Design Automation
- List available engines (AutoCAD, Revit, Inventor, 3ds Max)
- Manage app bundles and activities
- **Create activities** with custom commands
- **Submit and monitor work items**
- **Download work item results and reports**

### 🏗️ ACC Issues
- List, create, and update issues
- View issue types and subtypes
- Filter by status
- **Manage comments and attachments**
- **State transitions** between statuses

### 📋 ACC Extended Modules
- **Assets**: Manage project assets, tracking markers, categories
- **Submittals**: Manage submittal items and specs
- **Checklists**: Manage field checklists and templates

### ❓ RFIs
- **Manage Requests for Information**
- Create, update, answer, and track status

### 🔗 ACC Data Binding
- **Bind OSS objects to ACC project folders**
- Create linked items from external uploads

### 📸 Reality Capture
- Create photoscenes for photogrammetry
- Upload photos and start processing
- Monitor progress and download results

### 🧩 Plugins & Aliases
- **External Plugins**: Extend RAPS with `raps-<name>` executables
- **Aliases**: Create custom command shortcuts

### 🛠️ Development Tools
- **Synthetic Data Generation**: Create OBJ, IFC, JSON files for testing
- **Demo Scenarios**: Run end-to-end workflows (bucket lifecycle, model pipeline)

### 🤖 MCP Server (v3.0.0+)
- **AI Assistant Integration**: Model Context Protocol server for Claude, Cursor, and other MCP clients
- **14 MCP Tools**: Direct access to APS APIs from AI assistants
- **Natural Language Operations**: Manage buckets, objects, translations, and projects conversationally
- **Zero Code Required**: AI assistants handle the tool calls automatically

## Documentation Structure

- **[Getting Started](getting-started.md)** - Overview and prerequisites
- **[Installation](installation.md)** - Installation methods
- **[Configuration](configuration.md)** - Setting up credentials and profiles
- **[Feature Overview](features.md)** - Visual feature matrix and diagrams
- **[APS Feature Coverage](aps-coverage.md)** - Detailed APS service coverage matrix
- **[MCP Server](commands/mcp.md)** - AI assistant integration (v3.0.0+)
- **[Proxy Support](proxy-support.md)** - Configure proxy for corporate networks
- **[SBOM & Build Provenance](sbom.md)** - Software Bill of Materials
- **[Exit Codes](cli/exit-codes.md)** - CI/CD-friendly error handling
- **[Commands](commands/buckets.md)** - Complete command reference
- **[Pipelines](commands/pipeline.md)** - Batch operations and automation
- **[Examples](examples.md)** - Common use cases and workflows
- **[Troubleshooting](troubleshooting.md)** - Common issues and solutions

## Resources

- [APS Developer Portal](https://aps.autodesk.com)
- [APS Documentation](https://aps.autodesk.com/developer/documentation)
- [GitHub Repository](https://github.com/dmytro-yemelianov/raps)
- [Crates.io Package](https://crates.io/crates/raps)

## Support

For issues, questions, or contributions, please visit the [GitHub repository](https://github.com/dmytro-yemelianov/raps).


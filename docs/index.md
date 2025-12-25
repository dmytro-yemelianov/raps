# RAPS CLI Documentation

<div align="center">
  <img src="logo/output/raps-logo.png" alt="RAPS Logo" width="200"/>
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
- Generate signed S3 URLs for direct downloads

### 🔄 Model Derivative
- Translate CAD files to various formats (SVF2, OBJ, STL, STEP, etc.)
- Check translation status with polling
- View manifests and available derivatives

### 🏗️ Data Management
- Browse hubs, projects, folders, and items
- Create folders and manage versions
- Full BIM 360/ACC integration

### 🔔 Webhooks
- Create, list, and delete webhook subscriptions
- Support for data management and model derivative events

### ⚙️ Design Automation
- List available engines (AutoCAD, Revit, Inventor, 3ds Max)
- Manage app bundles and activities
- Submit and monitor work items

### 🏗️ ACC Issues
- List, create, and update issues
- View issue types and subtypes
- Filter by status

### 📸 Reality Capture
- Create photoscenes for photogrammetry
- Upload photos and start processing
- Monitor progress and download results

## Documentation Structure

- **[Getting Started](getting-started.md)** - Overview and prerequisites
- **[Installation](installation.md)** - Installation methods
- **[Configuration](configuration.md)** - Setting up credentials
- **[Proxy Support](proxy-support.md)** - Configure proxy for corporate networks
- **[SBOM & Build Provenance](sbom.md)** - Software Bill of Materials
- **[Commands](commands/buckets.md)** - Complete command reference
- **[Examples](examples.md)** - Common use cases and workflows
- **[Troubleshooting](troubleshooting.md)** - Common issues and solutions

## Resources

- [APS Developer Portal](https://aps.autodesk.com)
- [APS Documentation](https://aps.autodesk.com/developer/documentation)
- [GitHub Repository](https://github.com/dmytro-yemelianov/raps)
- [Crates.io Package](https://crates.io/crates/raps)

## Support

For issues, questions, or contributions, please visit the [GitHub repository](https://github.com/dmytro-yemelianov/raps).


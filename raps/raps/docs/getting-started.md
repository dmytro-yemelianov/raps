---
layout: default
title: Getting Started
---

# Getting Started with RAPS CLI

RAPS (Rust APS CLI) is a command-line interface for Autodesk Platform Services that makes it easy to interact with APS APIs from your terminal.

## Prerequisites

Before you begin, ensure you have:

1. **APS Account**: An account with Autodesk Platform Services
2. **Application Credentials**: Client ID and Client Secret from the [APS Developer Portal](https://aps.autodesk.com/myapps)
3. **Operating System**: Windows, macOS, or Linux

## What You'll Need

### APS Application Credentials

1. Go to [APS Developer Portal](https://aps.autodesk.com/myapps)
2. Create a new application or select an existing one
3. Note your **Client ID** and **Client Secret**
4. Configure callback URL (for 3-legged OAuth): `http://localhost:8080/callback`

### Optional: Design Automation Nickname

If you plan to use Design Automation features, you'll need:
- A **nickname** for your Design Automation app (must be unique across APS)

## Installation Methods

RAPS can be installed in several ways:

1. **[From Pre-built Binaries](installation.md#pre-built-binaries)** - Quickest method
2. **[From crates.io](installation.md#cratesio)** - Using Cargo package manager
3. **[From Source](installation.md#build-from-source)** - Build from GitHub repository

## Next Steps

1. **[Install RAPS](installation.md)** - Choose your installation method
2. **[Configure Credentials](configuration.md)** - Set up environment variables
3. **[Test Authentication](commands/auth.md#test)** - Verify your setup
4. **[Start Using Commands](commands/buckets.md)** - Begin working with APS

## First Command

After installation and configuration, test your setup:

```bash
raps auth test
```

This command tests your 2-legged OAuth credentials. If successful, you're ready to start using RAPS!

## Common Workflows

### Upload and Translate a Model

```bash
# 1. Create a bucket
raps bucket create

# 2. Upload a file
raps object upload <bucket-name> model.dwg

# 3. Translate to SVF2
raps translate start <urn> --format svf2

# 4. Check status
raps translate status <urn> --wait
```

### Browse BIM 360/ACC Projects

```bash
# 1. Login with 3-legged OAuth
raps auth login

# 2. List hubs
raps hub list

# 3. List projects
raps project list <hub-id>

# 4. Browse folders
raps folder list <project-id> <folder-id>
```

For more examples, see the [Examples](examples.md) page.


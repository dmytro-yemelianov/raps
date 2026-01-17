# Data Model: Multi-Channel RAPS Distribution

**Feature**: 006-raps-distribution
**Date**: 2026-01-17

## Overview

This feature primarily involves distribution tooling (scripts and packaging) rather than traditional data entities. The "data model" here describes configuration structures, release artifacts, and runtime state.

## Entities

### 1. Release Artifact

Represents a binary or package produced during the release process.

| Field | Type | Description |
|-------|------|-------------|
| name | string | Artifact filename (e.g., raps-linux-x64.tar.gz) |
| platform | enum | Target OS: linux, darwin, windows |
| architecture | enum | Target arch: x64, arm64 |
| version | string | Semantic version (e.g., 4.2.0) |
| checksum | string | SHA256 hash of the artifact |
| download_url | string | GitHub release download URL |

**Validation Rules**:
- version must match semver pattern: `^\d+\.\d+\.\d+$`
- checksum must be 64 hex characters (SHA256)
- platform and architecture must form valid combination

### 2. Install Configuration (Bash)

Runtime configuration for the Bash install script.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| RAPS_VERSION | string | "latest" | Version to install |
| RAPS_INSTALL_DIR | string | ~/.raps/bin | Installation directory |
| RAPS_NO_MODIFY_PATH | boolean | false | Skip PATH modification |

### 3. Install Configuration (PowerShell)

Runtime configuration for the PowerShell install script.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| Version | string | "latest" | Version to install |
| InstallDir | string | ~/.raps/bin | Installation directory |
| NoPathUpdate | switch | false | Skip PATH modification |
| Uninstall | switch | false | Remove existing installation |

### 4. Python Package Metadata

Package metadata for PyPI distribution.

| Field | Type | Value |
|-------|------|-------|
| name | string | "raps" |
| version | string | Synced from Cargo.toml |
| requires-python | string | ">=3.8" |
| license | string | "Apache-2.0" |
| classifiers | list | See below |

**Classifiers**:
- Development Status :: 4 - Beta
- Environment :: Console
- Intended Audience :: Developers
- License :: OSI Approved :: Apache Software License
- Operating System :: OS Independent
- Programming Language :: Rust
- Topic :: Software Development :: Build Tools

### 5. Platform Matrix

Mapping of platforms to build targets.

| Platform | Architecture | Rust Target | Wheel Tag |
|----------|--------------|-------------|-----------|
| Linux | x64 | x86_64-unknown-linux-gnu | manylinux_2_17_x86_64 |
| Linux | arm64 | aarch64-unknown-linux-gnu | manylinux_2_17_aarch64 |
| macOS | x64 | x86_64-apple-darwin | macosx_10_12_x86_64 |
| macOS | arm64 | aarch64-apple-darwin | macosx_11_0_arm64 |
| Windows | x64 | x86_64-pc-windows-msvc | win_amd64 |
| Windows | arm64 | aarch64-pc-windows-msvc | win_arm64 |

## State Transitions

### Install Script Flow

```
[Start] → [Detect Platform] → [Check Version] → [Download Binary]
    ↓
[Verify Checksum] → [Extract to Install Dir] → [Modify PATH]
    ↓
[Verify Installation] → [Print Success] → [End]
```

**Error States**:
- Unsupported platform → Print supported platforms, exit 1
- Download failure → Print retry instructions, exit 1
- Checksum mismatch → Print security warning, exit 1
- Install dir not writable → Suggest custom dir or permissions, exit 1
- PATH modification failed → Print manual instructions, exit 0 (partial success)

### Uninstall Script Flow

```
[Start] → [Find Installation] → [Remove Binary] → [Notify PATH Cleanup]
    ↓
[Print Success] → [End]
```

## Relationships

```
Release Artifact
    │
    ├── downloaded by → Install Script (Bash/PowerShell)
    │
    ├── bundled into → Python Wheel (via maturin)
    │
    └── referenced by → GitHub Release
```

## File Formats

### checksums.txt (GitHub Release)

```
<sha256>  raps-linux-x64.tar.gz
<sha256>  raps-linux-arm64.tar.gz
<sha256>  raps-darwin-x64.tar.gz
<sha256>  raps-darwin-arm64.tar.gz
<sha256>  raps-windows-x64.zip
<sha256>  raps-windows-arm64.zip
```

### pyproject.toml

```toml
[project]
name = "raps"
version = "0.0.0"  # Replaced by maturin from Cargo.toml
description = "Rust CLI for Autodesk Platform Services"
readme = "README.md"
license = {text = "Apache-2.0"}
requires-python = ">=3.8"
keywords = ["autodesk", "aps", "forge", "cad", "bim", "cli"]
classifiers = [
    "Development Status :: 4 - Beta",
    "Environment :: Console",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: Apache Software License",
    "Operating System :: OS Independent",
    "Programming Language :: Rust",
]

[project.urls]
Homepage = "https://rapscli.xyz"
Repository = "https://github.com/dmytro-yemelianov/raps"
Documentation = "https://rapscli.xyz/docs"

[tool.maturin]
bindings = "bin"
strip = true
```

# Contract: PyPI Package (raps)

**Feature**: 006-raps-distribution
**Date**: 2026-01-17

## Interface

### Installation

```bash
# Install latest version
pip install raps

# Install specific version
pip install raps==4.2.0

# Upgrade to latest
pip install --upgrade raps

# Install in virtual environment
python -m venv venv
source venv/bin/activate
pip install raps
```

### Usage After Install

```bash
# CLI is available directly
raps --version
raps --help
raps bucket list
raps auth test
```

## Package Structure

```
raps-4.2.0-py3-none-manylinux_2_17_x86_64.whl
├── raps/
│   ├── __init__.py      # Empty, provides package namespace
│   ├── __main__.py      # Entry point for `python -m raps`
│   └── raps (binary)    # Actual RAPS executable
└── raps-4.2.0.dist-info/
    ├── METADATA
    ├── WHEEL
    ├── entry_points.txt
    └── RECORD
```

### entry_points.txt

```
[console_scripts]
raps = raps:main
```

### __main__.py

```python
"""Entry point for running raps as a module."""
import os
import sys
import subprocess

def main():
    """Execute the raps binary with all provided arguments."""
    binary_path = os.path.join(os.path.dirname(__file__), "raps")
    if sys.platform == "win32":
        binary_path += ".exe"

    result = subprocess.run([binary_path] + sys.argv[1:])
    sys.exit(result.returncode)

if __name__ == "__main__":
    main()
```

## Package Metadata

| Field | Value |
|-------|-------|
| name | raps |
| version | Synced from Cargo.toml (e.g., 4.2.0) |
| description | Rust CLI for Autodesk Platform Services |
| author | Dmytro Yemelianov |
| license | Apache-2.0 |
| requires-python | >=3.8 |
| homepage | https://rapscli.xyz |
| repository | https://github.com/dmytro-yemelianov/raps |

### Keywords

- autodesk
- aps
- forge
- cad
- bim
- cli
- rust

### Classifiers

- Development Status :: 4 - Beta
- Environment :: Console
- Intended Audience :: Developers
- License :: OSI Approved :: Apache Software License
- Operating System :: OS Independent
- Programming Language :: Rust
- Topic :: Software Development :: Build Tools

## Platform Wheels

| Platform | Wheel Tag | Rust Target |
|----------|-----------|-------------|
| Linux x64 | manylinux_2_17_x86_64 | x86_64-unknown-linux-gnu |
| Linux ARM64 | manylinux_2_17_aarch64 | aarch64-unknown-linux-gnu |
| macOS x64 | macosx_10_12_x86_64 | x86_64-apple-darwin |
| macOS ARM64 | macosx_11_0_arm64 | aarch64-apple-darwin |
| Windows x64 | win_amd64 | x86_64-pc-windows-msvc |
| Windows ARM64 | win_arm64 | aarch64-pc-windows-msvc |

## Build Configuration (pyproject.toml)

```toml
[build-system]
requires = ["maturin>=1.4,<2.0"]
build-backend = "maturin"

[project]
name = "raps"
version = "0.0.0"  # Replaced by maturin
description = "Rust CLI for Autodesk Platform Services"
readme = "README.md"
license = {text = "Apache-2.0"}
requires-python = ">=3.8"
keywords = ["autodesk", "aps", "forge", "cad", "bim", "cli"]
authors = [
    {name = "Dmytro Yemelianov"}
]
classifiers = [
    "Development Status :: 4 - Beta",
    "Environment :: Console",
    "Intended Audience :: Developers",
    "License :: OSI Approved :: Apache Software License",
    "Operating System :: OS Independent",
    "Programming Language :: Rust",
    "Topic :: Software Development :: Build Tools",
]

[project.urls]
Homepage = "https://rapscli.xyz"
Repository = "https://github.com/dmytro-yemelianov/raps"
Documentation = "https://rapscli.xyz/docs"

[project.scripts]
raps = "raps:main"

[tool.maturin]
bindings = "bin"
strip = true
compatibility = "manylinux2014"
```

## Behavior

### pip install raps

1. pip resolves package and selects appropriate wheel for platform
2. Downloads wheel from PyPI
3. Extracts wheel to site-packages
4. Creates `raps` script in bin directory (virtualenv or user scripts)
5. `raps` command becomes available

### Error Conditions

| Condition | pip Behavior |
|-----------|--------------|
| Platform not supported | "ERROR: Could not find a version that satisfies the requirement raps" |
| Python version too old | "ERROR: raps requires Python >=3.8" |
| No network | Standard pip network error |

## Version Synchronization

Version is automatically extracted from `Cargo.toml` by maturin during build:

```
Cargo.toml: version = "4.2.0"
           ↓ (maturin build)
pyproject.toml: version = "4.2.0" (in wheel metadata)
```

This ensures CLI version and Python package version always match.

---
layout: default
title: Installation
---

# Installation

RAPS CLI can be installed using several methods. Choose the one that best fits your needs.

## Pre-built Binaries

The quickest way to get started is to download a pre-built binary for your platform.

### Download Latest Release

Visit the [Releases page](https://github.com/dmytro-yemelianov/raps/releases) and download the appropriate file for your platform:

| Platform | Architecture | File |
|----------|--------------|------|
| Windows | x64 | `raps-windows-x64.zip` |
| macOS | Intel | `raps-macos-x64.tar.gz` |
| macOS | Apple Silicon | `raps-macos-arm64.tar.gz` |
| Linux | x64 | `raps-linux-x64.tar.gz` |
| Linux | ARM64 | `raps-linux-arm64.tar.gz` |

### Windows Installation

**PowerShell:**

```powershell
# Download and extract
Expand-Archive raps-windows-x64.zip -DestinationPath "$env:USERPROFILE\bin"

# Add to PATH (if not already)
$env:PATH += ";$env:USERPROFILE\bin"

# Make permanent (add to your PowerShell profile)
[Environment]::SetEnvironmentVariable("Path", $env:Path + ";$env:USERPROFILE\bin", "User")
```

**Command Prompt:**

```cmd
# Extract to a directory in your PATH
# Then add that directory to your system PATH environment variable
```

### macOS/Linux Installation

```bash
# Extract the archive
tar -xzf raps-*.tar.gz

# Move to a directory in your PATH
sudo mv raps /usr/local/bin/

# Make executable
chmod +x /usr/local/bin/raps

# Verify installation
raps --version
```

## crates.io

If you have Rust and Cargo installed, you can install RAPS directly from crates.io:

```bash
cargo install raps
```

**Prerequisites:**
- Rust 1.70 or later ([rustup.rs](https://rustup.rs/))
- Cargo package manager (included with Rust)

## Build from Source

To build RAPS from source:

```bash
# Clone the repository
git clone https://github.com/dmytro-yemelianov/raps.git
cd raps

# Build in release mode
cargo build --release

# The binary will be at target/release/raps (or target/release/raps.exe on Windows)
```

**Prerequisites:**
- Rust 1.70 or later
- Cargo package manager
- Git

## Verify Installation

After installation, verify that RAPS is working:

```bash
raps --version
```

You should see output like:

```
raps 0.3.0
```

## Shell Completions

RAPS supports auto-completion for several shells. See the [Shell Completions](configuration#shell-completions) section for setup instructions.

## Next Steps

After installation:

1. **[Configure your credentials](configuration)**
2. **[Test authentication](commands/auth#test)**
3. **[Start using commands](commands/buckets)**


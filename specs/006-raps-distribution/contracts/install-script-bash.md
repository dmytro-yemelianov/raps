# Contract: Bash Install Script (install.sh)

**Feature**: 006-raps-distribution
**Date**: 2026-01-17

## Interface

### Invocation

```bash
# Default installation (latest version)
curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash

# Specific version
RAPS_VERSION=4.2.0 curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash

# Custom install directory
RAPS_INSTALL_DIR=/opt/raps/bin curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash

# Skip PATH modification
RAPS_NO_MODIFY_PATH=1 curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash

# Uninstall
curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash -s -- --uninstall
```

### Environment Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| RAPS_VERSION | string | "latest" | Specific version to install (e.g., "4.2.0") |
| RAPS_INSTALL_DIR | path | ~/.raps/bin | Installation directory |
| RAPS_NO_MODIFY_PATH | boolean | "" | If set (any value), skip PATH modification |

### Arguments

| Argument | Description |
|----------|-------------|
| --uninstall | Remove RAPS installation |
| --help | Show usage information |

## Behavior

### Success Flow

1. Print ASCII art banner
2. Detect OS (linux, darwin) and architecture (x86_64, aarch64)
3. Determine version (fetch latest from GitHub API if "latest")
4. Download binary archive from GitHub releases
5. Download checksums.txt
6. Verify SHA256 checksum
7. Create install directory if needed
8. Extract binary to install directory
9. Make binary executable
10. Detect shell and modify appropriate config file (unless RAPS_NO_MODIFY_PATH)
11. Verify installation with `raps --version`
12. Print success message with next steps

### Error Conditions

| Condition | Exit Code | Output |
|-----------|-----------|--------|
| Unsupported OS | 1 | "Error: Unsupported operating system: {os}. Supported: linux, darwin" |
| Unsupported architecture | 1 | "Error: Unsupported architecture: {arch}. Supported: x86_64, aarch64" |
| Download failed | 1 | "Error: Failed to download RAPS. Check your internet connection." |
| Checksum mismatch | 1 | "Error: Checksum verification failed. The download may be corrupted." |
| Install dir not writable | 1 | "Error: Cannot write to {dir}. Try: RAPS_INSTALL_DIR=/path/to/writable/dir" |
| Version not found | 1 | "Error: Version {version} not found. Available versions: ..." |
| Verification failed | 1 | "Error: Installation verification failed. Binary may be corrupted." |

### Output Format

```
     ____  ___    ____  _____
    / __ \/ _ |  / __ \/ ___/
   / /_/ / __ | / /_/ (__  )
  / _, _/ /_/ |/ ____/____/
 /_/ |_/_/ |_/_/

Installing RAPS v4.2.0 for linux-x86_64...

→ Downloading raps-linux-x64.tar.gz...
✓ Downloaded (12.3 MB)

→ Verifying checksum...
✓ Checksum verified

→ Installing to /home/user/.raps/bin...
✓ Installed

→ Updating PATH in ~/.bashrc...
✓ PATH updated

→ Verifying installation...
✓ raps 4.2.0 installed successfully!

To get started, run:
  raps --help

Note: You may need to restart your shell or run:
  source ~/.bashrc
```

## Dependencies

- curl or wget (for downloading)
- tar (for extracting .tar.gz)
- sha256sum or shasum (for checksum verification)

## Platform Matrix

| OS | Architecture | Archive Format | Binary Name |
|----|--------------|----------------|-------------|
| linux | x86_64 | .tar.gz | raps |
| linux | aarch64 | .tar.gz | raps |
| darwin | x86_64 | .tar.gz | raps |
| darwin | aarch64 | .tar.gz | raps |

# Contract: PowerShell Install Script (install.ps1)

**Feature**: 006-raps-distribution
**Date**: 2026-01-17

## Interface

### Invocation

```powershell
# Default installation (latest version)
irm https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.ps1 | iex

# Specific version
$env:RAPS_VERSION = "4.2.0"; irm https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.ps1 | iex

# Or download and run with parameters
Invoke-WebRequest -Uri https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.ps1 -OutFile install.ps1
.\install.ps1 -Version "4.2.0"
.\install.ps1 -InstallDir "C:\Tools\raps"
.\install.ps1 -NoPathUpdate
.\install.ps1 -Uninstall
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| -Version | string | "latest" | Specific version to install (e.g., "4.2.0") |
| -InstallDir | string | $HOME\.raps\bin | Installation directory |
| -NoPathUpdate | switch | false | Skip PATH environment variable modification |
| -Uninstall | switch | false | Remove RAPS installation |
| -Help | switch | false | Show usage information |

### Environment Variables (Alternative)

| Variable | Description |
|----------|-------------|
| RAPS_VERSION | Overrides -Version parameter |
| RAPS_INSTALL_DIR | Overrides -InstallDir parameter |

## Behavior

### Success Flow

1. Print ASCII art banner
2. Detect architecture (AMD64, ARM64)
3. Determine version (fetch latest from GitHub API if "latest")
4. Download binary archive from GitHub releases
5. Download checksums.txt
6. Verify SHA256 checksum
7. Create install directory if needed
8. Extract binary to install directory
9. Add install directory to User PATH (unless -NoPathUpdate)
10. Verify installation with `raps --version`
11. Print success message with next steps

### Error Conditions

| Condition | Exit Code | Output |
|-----------|-----------|--------|
| Unsupported architecture | 1 | "Error: Unsupported architecture: {arch}. Supported: AMD64, ARM64" |
| Download failed | 1 | "Error: Failed to download RAPS. Check your internet connection." |
| Checksum mismatch | 1 | "Error: Checksum verification failed. The download may be corrupted." |
| Install dir not writable | 1 | "Error: Cannot write to {dir}. Check permissions or specify -InstallDir" |
| Version not found | 1 | "Error: Version {version} not found. Check available versions at GitHub." |
| Verification failed | 1 | "Error: Installation verification failed. Binary may be corrupted." |
| PowerShell version too old | 1 | "Error: PowerShell 5.1 or later required. Current: {version}" |

### Output Format

```
     ____  ___    ____  _____
    / __ \/ _ |  / __ \/ ___/
   / /_/ / __ | / /_/ (__  )
  / _, _/ /_/ |/ ____/____/
 /_/ |_/_/ |_/_/

Installing RAPS v4.2.0 for windows-x64...

→ Downloading raps-windows-x64.zip...
✓ Downloaded (12.3 MB)

→ Verifying checksum...
✓ Checksum verified

→ Installing to C:\Users\user\.raps\bin...
✓ Installed

→ Updating User PATH...
✓ PATH updated

→ Verifying installation...
✓ raps 4.2.0 installed successfully!

To get started, run:
  raps --help

Note: You may need to restart your terminal for PATH changes to take effect.
```

### Uninstall Behavior

```
→ Removing RAPS installation...
✓ Removed binary from C:\Users\user\.raps\bin
✓ Removed C:\Users\user\.raps\bin from User PATH

RAPS has been uninstalled.
```

## Dependencies

- PowerShell 5.1+ (default on Windows 10/11)
- Invoke-WebRequest (built-in)
- Expand-Archive (built-in)
- Get-FileHash (built-in for SHA256)

## Platform Matrix

| Architecture | Archive Format | Binary Name |
|--------------|----------------|-------------|
| AMD64 (x64) | .zip | raps.exe |
| ARM64 | .zip | raps.exe |

## Security Considerations

- Downloads only from GitHub releases (HTTPS)
- Verifies SHA256 checksum before extraction
- Installs to user directory (no admin required)
- Modifies only User PATH (not system-wide)

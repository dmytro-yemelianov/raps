# Quickstart: Testing RAPS Distribution

**Feature**: 006-raps-distribution
**Date**: 2026-01-17

This guide covers how to test the distribution channels during development.

## Prerequisites

- Rust 1.88+ (for building binaries)
- Python 3.8+ (for maturin and pip testing)
- PowerShell 5.1+ (for Windows script testing)
- curl or wget (for Unix script testing)

## Local Testing

### 1. Test Bash Install Script

```bash
# Build the binary first
cargo build --release

# Create a mock release directory
mkdir -p /tmp/raps-release
cp target/release/raps /tmp/raps-release/
cd /tmp/raps-release
tar -czf raps-linux-x64.tar.gz raps
sha256sum raps-linux-x64.tar.gz > checksums.txt

# Test the install script (modify to use local files)
# For actual testing, use the script from the repo root
./install.sh --help
```

### 2. Test PowerShell Install Script

```powershell
# Build the binary first
cargo build --release

# Test the install script
.\install.ps1 -Help
.\install.ps1 -Version "local"  # Test with local binary
.\install.ps1 -Uninstall        # Test uninstall
```

### 3. Test Python Package (maturin)

```bash
# Navigate to python directory
cd python/

# Build locally
pip install maturin
maturin develop

# Test the installed command
raps --version

# Build a wheel
maturin build --release

# Install the wheel
pip install target/wheels/raps-*.whl
```

### 4. Test GitHub Actions Locally (act)

```bash
# Install act (GitHub Actions local runner)
brew install act  # macOS
# or see https://github.com/nektos/act

# Run the release workflow with a test tag
act push --tag v0.0.0-test

# Run specific job
act -j build-binaries
```

## Integration Testing

### Test Matrix

| Channel | Platform | Test Command |
|---------|----------|--------------|
| Bash script | Linux x64 | `curl ... \| bash && raps --version` |
| Bash script | Linux arm64 | `curl ... \| bash && raps --version` |
| Bash script | macOS x64 | `curl ... \| bash && raps --version` |
| Bash script | macOS arm64 | `curl ... \| bash && raps --version` |
| PowerShell | Windows x64 | `irm ... \| iex; raps --version` |
| PowerShell | Windows arm64 | `irm ... \| iex; raps --version` |
| pip | All platforms | `pip install raps && raps --version` |

### Smoke Tests

After any distribution channel install, run:

```bash
# Basic functionality
raps --version
raps --help

# Auth test (requires credentials)
raps auth test

# List buckets (requires credentials)
raps bucket list
```

### Verification Checklist

- [ ] Install script detects correct platform
- [ ] Install script downloads correct binary
- [ ] Checksum verification passes
- [ ] Binary is executable after install
- [ ] PATH is updated (or instructions shown)
- [ ] `raps --version` shows correct version
- [ ] Uninstall removes binary and notifies about PATH

## Troubleshooting

### Common Issues

**"Permission denied" on install**
```bash
# Use custom directory
RAPS_INSTALL_DIR=$HOME/bin ./install.sh
```

**"Command not found" after install**
```bash
# Reload shell config
source ~/.bashrc  # or ~/.zshrc
```

**PowerShell execution policy**
```powershell
# Allow script execution for current session
Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope Process
```

**maturin build fails**
```bash
# Ensure Rust target is installed
rustup target add x86_64-unknown-linux-gnu
```

## CI/CD Testing

The release workflow includes automated tests. To verify locally before pushing:

```bash
# Run the test-install workflow
gh workflow run test-install.yml

# Check workflow status
gh run list --workflow=test-install.yml
```

## Version Testing

Test specific versions work correctly:

```bash
# Test latest
RAPS_VERSION=latest ./install.sh

# Test specific version
RAPS_VERSION=4.2.0 ./install.sh

# Verify installed version matches
raps --version  # Should show 4.2.0
```

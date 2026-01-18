# Contract: GitHub Actions Release Workflow

**Feature**: 006-raps-distribution
**Date**: 2026-01-17

## Interface

### Trigger

```yaml
on:
  push:
    tags:
      - 'v*.*.*'  # Matches v4.2.0, v4.2.1-beta, etc.
```

### Outputs

Upon successful completion, the workflow produces:

1. **GitHub Release** with:
   - Binary archives for all 6 platforms
   - checksums.txt with SHA256 hashes
   - Release notes (from CHANGELOG or tag message)

2. **PyPI Package**:
   - Platform wheels for all 6 platforms
   - Source distribution (sdist)
   - Published to https://pypi.org/project/raps/

3. **Install Script Verification**:
   - Tested on ubuntu-latest, macos-latest, windows-latest
   - Verified binary works after install

## Workflow Structure

### Jobs

```yaml
jobs:
  # Job 1: Build Rust binaries for all platforms
  build-binaries:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: raps-linux-x64.tar.gz
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            artifact: raps-linux-arm64.tar.gz
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: raps-darwin-x64.tar.gz
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: raps-darwin-arm64.tar.gz
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: raps-windows-x64.zip
          - os: windows-latest
            target: aarch64-pc-windows-msvc
            artifact: raps-windows-arm64.zip

  # Job 2: Create GitHub Release with binaries
  create-release:
    needs: build-binaries
    # Uploads all artifacts, generates checksums.txt

  # Job 3: Build Python wheels with maturin
  build-wheels:
    needs: build-binaries
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        target: [x86_64, aarch64]
    # Uses maturin-action to build wheels

  # Job 4: Publish to PyPI
  publish-pypi:
    needs: build-wheels
    environment: pypi
    permissions:
      id-token: write  # OIDC trusted publishing

  # Job 5: Test install scripts
  test-install-scripts:
    needs: create-release
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    # Runs install script, verifies raps --version
```

## Job Details

### build-binaries

**Purpose**: Compile RAPS for all target platforms.

**Steps**:
1. Checkout repository
2. Setup Rust toolchain with target
3. Install cross-compilation tools (if needed)
4. Run `cargo build --release --target ${{ matrix.target }}`
5. Package binary into archive (.tar.gz for Unix, .zip for Windows)
6. Upload artifact

**Outputs**: Binary archives for each platform

### create-release

**Purpose**: Create GitHub Release with all artifacts.

**Steps**:
1. Download all binary artifacts
2. Generate checksums.txt with SHA256 hashes
3. Create GitHub Release using `softprops/action-gh-release`
4. Upload all binaries and checksums.txt

**Outputs**: GitHub Release URL

### build-wheels

**Purpose**: Build Python wheels for PyPI.

**Steps**:
1. Checkout repository
2. Download pre-built binary for this platform
3. Run maturin-action with `command: build`
4. Upload wheel artifact

**Configuration**:
```yaml
- uses: PyO3/maturin-action@v1
  with:
    command: build
    args: --release --out dist
    target: ${{ matrix.target }}
    manylinux: auto
```

**Outputs**: Platform-specific wheels

### publish-pypi

**Purpose**: Publish wheels to PyPI using OIDC.

**Prerequisites**:
- PyPI trusted publisher configured for the repository
- `pypi` environment defined in GitHub repo settings

**Steps**:
1. Download all wheel artifacts
2. Publish using `pypa/gh-action-pypi-publish`

**Configuration**:
```yaml
- uses: pypa/gh-action-pypi-publish@release/v1
  # No API token needed - uses OIDC
```

### test-install-scripts

**Purpose**: Verify install scripts work after release.

**Steps**:
1. Run install script (curl | bash or irm | iex)
2. Verify `raps --version` returns expected version
3. Run basic command (`raps --help`)
4. Run uninstall

**Matrix**:
| OS | Script | Command |
|----|--------|---------|
| ubuntu-latest | install.sh | `curl -fsSL .../install.sh \| bash` |
| macos-latest | install.sh | `curl -fsSL .../install.sh \| bash` |
| windows-latest | install.ps1 | `irm .../install.ps1 \| iex` |

## Secrets & Permissions

### Required Secrets

| Secret | Purpose | Required For |
|--------|---------|--------------|
| GITHUB_TOKEN | Auto-provided | create-release |
| (none) | OIDC handles PyPI auth | publish-pypi |

### Required Permissions

```yaml
permissions:
  contents: write    # For creating releases
  id-token: write    # For PyPI OIDC
```

### PyPI Trusted Publisher Setup

Configure in PyPI project settings:
- GitHub repository: `dmytro-yemelianov/raps`
- Workflow: `release.yml`
- Environment: `pypi`

## Error Handling

| Failure Point | Behavior |
|---------------|----------|
| Binary build fails | Job fails, no release created |
| Release creation fails | Subsequent jobs skipped |
| Wheel build fails | PyPI publish skipped for that platform |
| PyPI publish fails | Retries 3 times, then fails workflow |
| Install script test fails | Workflow reports failure but release exists |

## Timeout Configuration

| Job | Timeout |
|-----|---------|
| build-binaries | 30 minutes |
| create-release | 10 minutes |
| build-wheels | 20 minutes |
| publish-pypi | 10 minutes |
| test-install-scripts | 15 minutes |

## Example Full Workflow Run

```
Tag pushed: v4.2.0
  │
  ├─► build-binaries (6 parallel jobs, ~15 min)
  │     ├─► linux-x64: ✓
  │     ├─► linux-arm64: ✓
  │     ├─► darwin-x64: ✓
  │     ├─► darwin-arm64: ✓
  │     ├─► windows-x64: ✓
  │     └─► windows-arm64: ✓
  │
  ├─► create-release (~2 min)
  │     └─► GitHub Release v4.2.0 created with 7 files
  │
  ├─► build-wheels (6 parallel jobs, ~10 min)
  │     └─► 6 wheels built
  │
  ├─► publish-pypi (~2 min)
  │     └─► raps 4.2.0 published to PyPI
  │
  └─► test-install-scripts (3 parallel jobs, ~5 min)
        ├─► ubuntu: ✓ raps 4.2.0
        ├─► macos: ✓ raps 4.2.0
        └─► windows: ✓ raps 4.2.0

Total time: ~25 minutes
```
